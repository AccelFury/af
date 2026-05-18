// SPDX-License-Identifier: Apache-2.0
use af_manifest::{CoreManifest, ManifestError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use toml::Value;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ArchitectureCheckReport {
    pub generated_by: String,
    pub status: String,
    pub project_dir: PathBuf,
    pub checked: Vec<String>,
    pub issues: Vec<ArchitectureIssue>,
    pub warnings: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ArchitectureIssue {
    pub code: String,
    pub message: String,
    pub hint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Error)]
pub enum ArchitectureError {
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    #[error("failed to read `{path}`: {message}")]
    Read { path: PathBuf, message: String },
    #[error("failed to parse `{path}`: {message}")]
    Parse { path: PathBuf, message: String },
}

impl ArchitectureError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Manifest(err) => err.code(),
            Self::Read { .. } => "AF_ARCH_READ_FAILED",
            Self::Parse { .. } => "AF_ARCH_PARSE_FAILED",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            Self::Manifest(err) => err.hint(),
            Self::Read { .. } => "Check that af-arch.toml and declared sources are readable.",
            Self::Parse { .. } => "Fix af-arch.toml syntax.",
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Manifest(err) => err.exit_code(),
            Self::Read { .. } | Self::Parse { .. } => 2,
        }
    }
}

pub fn check_architecture(
    project_dir: impl AsRef<Path>,
) -> Result<ArchitectureCheckReport, ArchitectureError> {
    let project_dir = project_dir.as_ref();
    let manifest = CoreManifest::from_path(project_dir.join("af-core.toml"))?;
    let arch = read_arch(project_dir)?;
    let mut report = ArchitectureCheckReport {
        generated_by: "AccelFury IP Toolchain".to_string(),
        status: "passed".to_string(),
        project_dir: project_dir.to_path_buf(),
        checked: vec![
            "common layer vendor leakage".to_string(),
            "resource contracts".to_string(),
            "CDC contracts".to_string(),
            "backend matrix limitations".to_string(),
            "constructor metadata".to_string(),
            "verification gates".to_string(),
        ],
        issues: Vec::new(),
        warnings: Vec::new(),
        limitations: vec![
            "Architecture check is structural and marker-based; it is not a full SystemVerilog parser.".to_string(),
        ],
    };

    check_common_sources(project_dir, &manifest, &mut report)?;
    check_vendor_layout(&manifest, &mut report);
    check_resource_contracts(project_dir, &manifest, &mut report)?;
    check_cdc_contracts(&manifest, arch.as_ref(), &mut report);
    check_backend_limitations(&manifest, &mut report);
    check_constructor(&manifest, project_dir, &mut report);
    check_verification_gates(&manifest, project_dir, &mut report);

    if !report.issues.is_empty() {
        report.status = "failed".to_string();
    } else if !report.warnings.is_empty() {
        report.status = "warning".to_string();
    }
    Ok(report)
}

fn read_arch(project_dir: &Path) -> Result<Option<Value>, ArchitectureError> {
    let path = project_dir.join("af-arch.toml");
    if !path.is_file() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path).map_err(|err| ArchitectureError::Read {
        path: path.clone(),
        message: err.to_string(),
    })?;
    toml::from_str(&raw)
        .map(Some)
        .map_err(|err| ArchitectureError::Parse {
            path,
            message: err.to_string(),
        })
}

fn check_common_sources(
    project_dir: &Path,
    manifest: &CoreManifest,
    report: &mut ArchitectureCheckReport,
) -> Result<(), ArchitectureError> {
    for source in &manifest.sources.files {
        if !(source.starts_with("rtl/common/") || source.starts_with("common/")) {
            continue;
        }
        let path = project_dir.join(source);
        let text = fs::read_to_string(&path).map_err(|err| ArchitectureError::Read {
            path: path.clone(),
            message: err.to_string(),
        })?;
        let lower = text.to_ascii_lowercase();
        for marker in vendor_markers() {
            if lower.contains(marker) {
                report.issues.push(ArchitectureIssue {
                    code: "AF_ARCH_LAYER_VIOLATION".to_string(),
                    message: format!("common source `{source}` contains vendor/platform marker `{marker}`"),
                    hint: "Move vendor primitives, PLL/MMCM, hard IP, and board-specific code to vendor/<vendor>/.".to_string(),
                    path: Some(path.clone()),
                });
            }
        }
    }
    Ok(())
}

fn check_vendor_layout(manifest: &CoreManifest, report: &mut ArchitectureCheckReport) {
    for source in &manifest.sources.files {
        if let Some(rest) = source.strip_prefix("vendor/") {
            if rest.split('/').next().is_none_or(str::is_empty) {
                report.issues.push(ArchitectureIssue {
                    code: "AF_ARCH_LAYER_VIOLATION".to_string(),
                    message: format!("vendor source `{source}` is not under vendor/<vendor>"),
                    hint: "Place backend-specific files under vendor/<vendor>/... and keep common RTL generic.".to_string(),
                    path: None,
                });
            }
        }
    }
}

fn check_resource_contracts(
    project_dir: &Path,
    manifest: &CoreManifest,
    report: &mut ArchitectureCheckReport,
) -> Result<(), ArchitectureError> {
    if !manifest.resources.memory.is_empty() || !manifest.resources.dsp.is_empty() {
        return Ok(());
    }
    for source in &manifest.sources.files {
        let path = project_dir.join(source);
        let text = fs::read_to_string(&path).map_err(|err| ArchitectureError::Read {
            path: path.clone(),
            message: err.to_string(),
        })?;
        let lower = text.to_ascii_lowercase();
        if ["ram", "bram", "fifo", "dsp", "mult"]
            .iter()
            .any(|marker| lower.contains(marker))
        {
            report.issues.push(ArchitectureIssue {
                code: "AF_RESOURCE_CONTRACT_MISSING".to_string(),
                message: format!("resource-like RTL marker found in `{source}` without a manifest resource contract"),
                hint: "Declare [[resources.memory]] or [[resources.dsp]] with backend_policy and latency/resource intent.".to_string(),
                path: Some(path),
            });
        }
    }
    Ok(())
}

fn check_cdc_contracts(
    manifest: &CoreManifest,
    arch: Option<&Value>,
    report: &mut ArchitectureCheckReport,
) {
    if manifest.clocks.len() <= 1 {
        return;
    }
    let has_cdc = arch
        .and_then(|value| value.get("cdc"))
        .is_some_and(|cdc| match cdc {
            Value::Array(entries) => !entries.is_empty(),
            Value::Table(table) => !table.is_empty(),
            _ => false,
        });
    if !has_cdc {
        report.issues.push(ArchitectureIssue {
            code: "AF_CDC_CONTRACT_MISSING".to_string(),
            message: "multiple clock domains are declared without CDC contracts in af-arch.toml".to_string(),
            hint: "Describe every CDC crossing, synchronizer/FIFO strategy, reset interaction, and verification obligation.".to_string(),
            path: None,
        });
    }
}

fn check_backend_limitations(manifest: &CoreManifest, report: &mut ArchitectureCheckReport) {
    for variant in &manifest.backend_variants {
        if matches!(variant.status.as_str(), "planned" | "unsupported")
            && manifest.known_limitations.is_empty()
        {
            report.issues.push(ArchitectureIssue {
                code: "AF_BACKEND_REQUIRED".to_string(),
                message: format!(
                    "backend variant `{}` is `{}` without known limitations",
                    variant.name, variant.status
                ),
                hint: "Add known_limitations that state which backend is planned or unsupported and what remains unverified.".to_string(),
                path: None,
            });
        }
    }
}

fn check_verification_gates(
    manifest: &CoreManifest,
    project_dir: &Path,
    report: &mut ArchitectureCheckReport,
) {
    for gate in &manifest.verification_required {
        let kind = gate.kind.as_str();
        if let Some(evidence) = &gate.evidence {
            let path = project_dir.join(evidence);
            if !path.exists() {
                report.issues.push(ArchitectureIssue {
                    code: "AF_VERIFICATION_EVIDENCE_MISSING".to_string(),
                    message: format!(
                        "verification gate `{kind}` references missing evidence `{evidence}`"
                    ),
                    hint:
                        "Generate or commit the evidence artifact, or remove the evidence path until it is available."
                            .to_string(),
                    path: Some(path),
                });
            }
        } else {
            report.warnings.push(format!(
                "AF_VERIFICATION_EVIDENCE_PLANNED: verification gate `{kind}` is declared without an evidence path"
            ));
        }
    }
}

fn check_constructor(
    manifest: &CoreManifest,
    project_dir: &Path,
    report: &mut ArchitectureCheckReport,
) {
    if !manifest
        .constructor
        .as_ref()
        .is_some_and(|constructor| constructor.export)
    {
        return;
    }
    let constructor_dir = project_dir.join("constructor");
    if !constructor_dir.exists() {
        report.warnings.push(
            "AF_CONSTRUCTOR_EXPORT_INCOMPLETE: constructor export is enabled but constructor/ metadata has not been generated yet".to_string(),
        );
    }
}

fn vendor_markers() -> &'static [&'static str] {
    &[
        "ramb",
        "dsp48",
        "xpm_",
        "mmcm",
        "plle",
        "pll_",
        "clk_wiz",
        "altsyncram",
        "scfifo",
        "dcfifo",
        "gowin_",
        "sb_ram",
        "ehxpll",
        "pcie",
        "serdes",
    ]
}
