// SPDX-License-Identifier: Apache-2.0
use af_manifest::{CoreManifest, ManifestError};
use af_vendor_db::{capability, resolve_target, VendorCapability, VendorTarget};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ResourcePlanReport {
    pub generated_by: String,
    pub status: String,
    pub core_dir: PathBuf,
    pub target: VendorTarget,
    pub capability: VendorCapability,
    pub resources: BTreeMap<String, ResourceEstimate>,
    pub risks: Vec<String>,
    pub warnings: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ResourceEstimate {
    pub estimated: u64,
    pub policy: String,
    pub uncertainty: String,
}

#[derive(Debug, Error)]
pub enum ResourceModelError {
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    #[error("failed to read RTL source `{path}`: {message}")]
    Read { path: PathBuf, message: String },
}

impl ResourceModelError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Manifest(err) => err.code(),
            Self::Read { .. } => "AF_RESOURCE_SOURCE_READ_FAILED",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            Self::Manifest(err) => err.hint(),
            Self::Read { .. } => "Check that declared RTL sources are readable.",
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Manifest(err) => err.exit_code(),
            Self::Read { .. } => 2,
        }
    }
}

pub fn plan_resources(
    core_dir: impl AsRef<Path>,
    vendor: Option<&str>,
    family: Option<&str>,
    board: Option<&str>,
) -> Result<ResourcePlanReport, ResourceModelError> {
    let core_dir = core_dir.as_ref();
    let manifest = CoreManifest::from_path(core_dir.join("af-core.toml"))?;
    let target = resolve_target(vendor, family, board);
    let capability = capability(&target);
    let mut resources = BTreeMap::new();
    let mut risks = Vec::new();
    let mut warnings = Vec::new();

    let bram = manifest
        .resources
        .memory
        .iter()
        .map(|memory| {
            let bits = u64::from(memory.width) * u64::from(memory.depth);
            bits.div_ceil(18_432).max(1)
        })
        .sum::<u64>();
    if bram > 0 {
        let policy = strongest_policy(
            manifest
                .resources
                .memory
                .iter()
                .map(|memory| memory.backend_policy.as_str()),
        );
        resources.insert(
            "bram".to_string(),
            ResourceEstimate {
                estimated: bram,
                policy: policy.to_string(),
                uncertainty: "medium".to_string(),
            },
        );
        if !capability.bram {
            risks.push(
                "AF_VENDOR_CAPABILITY_MISSING: target has no declared BRAM capability".to_string(),
            );
        }
    }

    let dsp = manifest
        .resources
        .dsp
        .iter()
        .map(|dsp| u64::from(dsp.count))
        .sum::<u64>();
    if dsp > 0 {
        let policy = strongest_policy(
            manifest
                .resources
                .dsp
                .iter()
                .map(|dsp| dsp.backend_policy.as_str()),
        );
        resources.insert(
            "dsp".to_string(),
            ResourceEstimate {
                estimated: dsp,
                policy: policy.to_string(),
                uncertainty: "medium".to_string(),
            },
        );
        if !capability.dsp {
            risks.push(
                "AF_VENDOR_CAPABILITY_MISSING: target has no declared DSP capability".to_string(),
            );
        }
        if policy == "require_vendor" && target.vendor == "generic" {
            risks.push("AF_BACKEND_REQUIRED: DSP contract requires a vendor target".to_string());
        }
    }

    let source_text = read_sources(core_dir, &manifest)?;
    let lut_estimate = estimate_luts(&source_text, &manifest);
    resources.insert(
        "lut".to_string(),
        ResourceEstimate {
            estimated: lut_estimate,
            policy: "portable".to_string(),
            uncertainty: "high".to_string(),
        },
    );

    if source_text.to_ascii_lowercase().contains("dsp") && dsp == 0 {
        risks.push("generic_dsp_mapping_may_not_meet_fmax".to_string());
    }
    if manifest.resources.memory.is_empty() && source_text.to_ascii_lowercase().contains("ram") {
        warnings.push(
            "AF_RESOURCE_CONTRACT_MISSING: RAM-like RTL text found without a memory contract"
                .to_string(),
        );
    }
    if resources.len() == 1 {
        warnings.push("No explicit resource contracts found; report is a high-uncertainty structural estimate.".to_string());
    }

    Ok(ResourcePlanReport {
        generated_by: "AccelFury IP Toolchain".to_string(),
        status: if risks.iter().any(|risk| risk.starts_with("AF_VENDOR_CAPABILITY_MISSING")) {
            "warning".to_string()
        } else {
            "passed".to_string()
        },
        core_dir: core_dir.to_path_buf(),
        target,
        capability,
        resources,
        risks,
        warnings,
        limitations: vec![
            "Offline estimate only; no synthesis, placement, routing, timing, or exact device utilization was run.".to_string(),
        ],
    })
}

fn strongest_policy<'a>(policies: impl Iterator<Item = &'a str>) -> &'static str {
    let mut out = "portable";
    for policy in policies {
        if policy == "require_vendor" {
            return "require_vendor";
        }
        if policy == "prefer_vendor" {
            out = "prefer_vendor";
        }
    }
    out
}

fn read_sources(core_dir: &Path, manifest: &CoreManifest) -> Result<String, ResourceModelError> {
    let mut out = String::new();
    for source in &manifest.sources.files {
        let path = core_dir.join(source);
        let text = fs::read_to_string(&path).map_err(|err| ResourceModelError::Read {
            path,
            message: err.to_string(),
        })?;
        out.push_str(&text);
        out.push('\n');
    }
    Ok(out)
}

fn estimate_luts(source_text: &str, manifest: &CoreManifest) -> u64 {
    let ops = ["+", "-", "^", "&", "|", "case", "always", "assign"]
        .iter()
        .map(|needle| source_text.matches(needle).count() as u64)
        .sum::<u64>();
    let port_factor = manifest.ports.len() as u64 * 8;
    (ops * 16 + port_factor).max(16)
}
