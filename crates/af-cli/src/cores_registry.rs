// SPDX-License-Identifier: Apache-2.0
//! Manifesto-aligned universal-core registry loader and validator.
//!
//! Reads `registries/cores.registry.json` and surfaces it both for
//! `af registry check` (validation) and `af core registry list`
//! (priority/portability-filtered listing). The registry tracks each
//! `af_*` core with priority (P0/P1/P2), portability level (U0..U4),
//! maturity, and declared verification gates.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub const REGISTRY_RELATIVE_PATH: &str = "registries/cores.registry.json";
pub const SUPPORTED_SCHEMA_VERSION: &str = "0.1";

const VALID_CATEGORIES: &[&str] = &[
    "field_arithmetic",
    "ntt_fft",
    "hash",
    "merkle",
    "plonk",
    "stark",
    "r1cs",
    "msm_toy",
    "ecc_toy",
    "stream_infra",
    "memory_infra",
    "board_debug",
    "dsp",
    "signal_processing",
    "image_video",
    "audio",
    "ai_tinyml",
    "softcore_peripheral",
];

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CoresRegistry {
    pub schema_version: String,
    #[serde(default)]
    pub generated_by: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub cores: Vec<RegisteredCore>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RegisteredCore {
    pub core_id: String,
    pub category: String,
    pub priority: String,
    pub portability_level: String,
    pub maturity: String,
    pub summary: String,
    #[serde(default)]
    pub verification_required: Vec<String>,
    #[serde(default)]
    pub reference_path: Option<String>,
    #[serde(default)]
    pub tracking_issue: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CoresRegistryReport {
    pub valid: bool,
    pub path: PathBuf,
    pub core_count: usize,
    pub issues: Vec<RegistryIssue>,
    pub warnings: Vec<RegistryIssue>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RegistryIssue {
    pub code: String,
    pub message: String,
    pub hint: String,
}

#[derive(Debug)]
pub enum LoadError {
    Read { path: PathBuf, message: String },
    Parse { path: PathBuf, message: String },
}

impl LoadError {
    pub fn code(&self) -> &'static str {
        match self {
            LoadError::Read { .. } => "AF_CORES_REGISTRY_READ_FAILED",
            LoadError::Parse { .. } => "AF_CORES_REGISTRY_PARSE_FAILED",
        }
    }

    pub fn message(&self) -> String {
        match self {
            LoadError::Read { path, message } => {
                format!("failed to read `{}`: {message}", path.display())
            }
            LoadError::Parse { path, message } => {
                format!("failed to parse `{}`: {message}", path.display())
            }
        }
    }

    pub fn hint(&self) -> &'static str {
        "Ensure registries/cores.registry.json exists and matches schemas/cores.registry.schema.json."
    }
}

pub fn load(root: &Path) -> Result<CoresRegistry, LoadError> {
    let path = root.join(REGISTRY_RELATIVE_PATH);
    let raw = std::fs::read_to_string(&path).map_err(|err| LoadError::Read {
        path: path.clone(),
        message: err.to_string(),
    })?;
    serde_json::from_str(&raw).map_err(|err| LoadError::Parse {
        path,
        message: err.to_string(),
    })
}

pub fn registry_path(root: &Path) -> PathBuf {
    root.join(REGISTRY_RELATIVE_PATH)
}

pub fn check(root: &Path) -> CoresRegistryReport {
    let path = registry_path(root);
    if !path.exists() {
        // The cores registry is optional: tests, scaffolds, and forks may
        // exist without one. Surface a warning so users know the manifesto
        // axis exists, but do not fail the registry check.
        return CoresRegistryReport {
            valid: true,
            path,
            core_count: 0,
            issues: Vec::new(),
            warnings: vec![RegistryIssue {
                code: "AF_CORES_REGISTRY_NOT_PRESENT".to_string(),
                message: "registries/cores.registry.json is not present; manifesto axes (priority/portability_level) are not tracked here".to_string(),
                hint: "Create registries/cores.registry.json (see schemas/cores.registry.schema.json) to track P0/P1 universal cores.".to_string(),
            }],
        };
    }
    let registry = match load(root) {
        Ok(registry) => registry,
        Err(err) => {
            return CoresRegistryReport {
                valid: false,
                path,
                core_count: 0,
                issues: vec![RegistryIssue {
                    code: err.code().to_string(),
                    message: err.message(),
                    hint: err.hint().to_string(),
                }],
                warnings: Vec::new(),
            };
        }
    };

    let mut issues = Vec::new();
    let mut warnings = Vec::new();

    if registry.schema_version != SUPPORTED_SCHEMA_VERSION {
        issues.push(RegistryIssue {
            code: "AF_CORES_REGISTRY_SCHEMA_UNSUPPORTED".to_string(),
            message: format!(
                "unsupported cores.registry.json schema_version `{}`",
                registry.schema_version
            ),
            hint: format!("Use schema_version = \"{SUPPORTED_SCHEMA_VERSION}\"."),
        });
    }

    let categories: BTreeSet<&str> = VALID_CATEGORIES.iter().copied().collect();
    let mut seen_ids = BTreeSet::new();

    for core in &registry.cores {
        if !seen_ids.insert(core.core_id.as_str()) {
            issues.push(RegistryIssue {
                code: "AF_CORES_REGISTRY_DUPLICATE_ID".to_string(),
                message: format!("duplicate core_id `{}`", core.core_id),
                hint: "Each core_id must appear once in cores.registry.json.".to_string(),
            });
        }
        if !is_core_id(&core.core_id) {
            issues.push(RegistryIssue {
                code: "AF_CORES_REGISTRY_INVALID_ID".to_string(),
                message: format!("core_id `{}` is invalid", core.core_id),
                hint: "Use lowercase letters, digits, and underscores; identifier must start with `af_`.".to_string(),
            });
        }
        if !categories.contains(core.category.as_str()) {
            issues.push(RegistryIssue {
                code: "AF_CORES_REGISTRY_CATEGORY_UNKNOWN".to_string(),
                message: format!(
                    "core `{}` references unknown category `{}`",
                    core.core_id, core.category
                ),
                hint: "Use a category listed in registries/ip_categories.json.".to_string(),
            });
        }
        if !matches!(core.priority.as_str(), "P0" | "P1" | "P2") {
            issues.push(RegistryIssue {
                code: "AF_CORES_REGISTRY_PRIORITY_INVALID".to_string(),
                message: format!(
                    "core `{}` has unsupported priority `{}`",
                    core.core_id, core.priority
                ),
                hint: "Use priority = \"P0\", \"P1\", or \"P2\".".to_string(),
            });
        }
        if !matches!(
            core.portability_level.as_str(),
            "U0" | "U1" | "U2" | "U3" | "U4"
        ) {
            issues.push(RegistryIssue {
                code: "AF_CORES_REGISTRY_PORTABILITY_INVALID".to_string(),
                message: format!(
                    "core `{}` has unsupported portability_level `{}`",
                    core.core_id, core.portability_level
                ),
                hint: "Use portability_level in U0..U4.".to_string(),
            });
        }
        if !matches!(
            core.maturity.as_str(),
            "experimental" | "preview" | "beta" | "stable" | "deprecated"
        ) {
            issues.push(RegistryIssue {
                code: "AF_CORES_REGISTRY_MATURITY_INVALID".to_string(),
                message: format!(
                    "core `{}` has unsupported maturity `{}`",
                    core.core_id, core.maturity
                ),
                hint: "Use maturity in experimental|preview|beta|stable|deprecated.".to_string(),
            });
        }
        if core.summary.trim().is_empty() {
            issues.push(RegistryIssue {
                code: "AF_CORES_REGISTRY_SUMMARY_EMPTY".to_string(),
                message: format!("core `{}` has an empty summary", core.core_id),
                hint: "Add a one-line description of the core.".to_string(),
            });
        }
        for gate in &core.verification_required {
            if !matches!(
                gate.as_str(),
                "simulation"
                    | "formal-cdc-assumption"
                    | "formal-occupancy"
                    | "formal-equivalence"
                    | "random-stress"
                    | "board-demo"
                    | "synthesis-report"
            ) {
                issues.push(RegistryIssue {
                    code: "AF_CORES_REGISTRY_VERIFICATION_UNKNOWN".to_string(),
                    message: format!(
                        "core `{}` declares unknown verification gate `{gate}`",
                        core.core_id
                    ),
                    hint: "Use a kind listed in schemas/cores.registry.schema.json.".to_string(),
                });
            }
        }
        if let Some(reference) = &core.reference_path {
            let path = root.join(reference);
            if !path.exists() {
                warnings.push(RegistryIssue {
                    code: "AF_CORES_REGISTRY_REFERENCE_MISSING".to_string(),
                    message: format!(
                        "core `{}` reference_path `{reference}` does not exist",
                        core.core_id
                    ),
                    hint: "Create the manifest or remove reference_path until it is available."
                        .to_string(),
                });
            }
        }
    }

    CoresRegistryReport {
        valid: issues.is_empty(),
        path,
        core_count: registry.cores.len(),
        issues,
        warnings,
    }
}

fn is_core_id(value: &str) -> bool {
    if !value.starts_with("af_") {
        return false;
    }
    value
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_registry(root: &Path, contents: &str) {
        let dir = root.join("registries");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("cores.registry.json"), contents).unwrap();
    }

    #[test]
    fn accepts_well_formed_registry() {
        let temp = TempDir::new().unwrap();
        write_registry(
            temp.path(),
            r#"{
  "schema_version": "0.1",
  "cores": [
    {
      "core_id": "af_reset_sync",
      "category": "stream_infra",
      "priority": "P0",
      "portability_level": "U0",
      "maturity": "preview",
      "summary": "Reset synchronizer.",
      "verification_required": ["formal-cdc-assumption", "simulation"]
    }
  ]
}"#,
        );
        let report = check(temp.path());
        assert!(report.valid, "{report:?}");
        assert_eq!(report.core_count, 1);
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn rejects_duplicates_and_unknown_values() {
        let temp = TempDir::new().unwrap();
        write_registry(
            temp.path(),
            r#"{
  "schema_version": "0.1",
  "cores": [
    {
      "core_id": "af_uart",
      "category": "softcore_peripheral",
      "priority": "P0",
      "portability_level": "U0",
      "maturity": "experimental",
      "summary": "UART."
    },
    {
      "core_id": "af_uart",
      "category": "no_such_category",
      "priority": "P9",
      "portability_level": "U7",
      "maturity": "godlike",
      "summary": "",
      "verification_required": ["mock-vibes"]
    }
  ]
}"#,
        );
        let report = check(temp.path());
        assert!(!report.valid);
        let codes: BTreeSet<&str> = report.issues.iter().map(|i| i.code.as_str()).collect();
        for code in [
            "AF_CORES_REGISTRY_DUPLICATE_ID",
            "AF_CORES_REGISTRY_CATEGORY_UNKNOWN",
            "AF_CORES_REGISTRY_PRIORITY_INVALID",
            "AF_CORES_REGISTRY_PORTABILITY_INVALID",
            "AF_CORES_REGISTRY_MATURITY_INVALID",
            "AF_CORES_REGISTRY_SUMMARY_EMPTY",
            "AF_CORES_REGISTRY_VERIFICATION_UNKNOWN",
        ] {
            assert!(codes.contains(code), "missing {code}; got {codes:?}");
        }
    }

    #[test]
    fn warns_when_reference_path_missing() {
        let temp = TempDir::new().unwrap();
        write_registry(
            temp.path(),
            r#"{
  "schema_version": "0.1",
  "cores": [
    {
      "core_id": "af_pdm_rx",
      "category": "audio",
      "priority": "P2",
      "portability_level": "U0",
      "maturity": "preview",
      "summary": "PDM RX.",
      "reference_path": "examples/af-pdm-rx/af-core.toml"
    }
  ]
}"#,
        );
        let report = check(temp.path());
        assert!(report.valid, "{report:?}");
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(
            report.warnings[0].code,
            "AF_CORES_REGISTRY_REFERENCE_MISSING"
        );
    }

    #[test]
    fn shipped_registry_loads() {
        // When CARGO_MANIFEST_DIR is af-cli/, repo root is two levels up.
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let repo_root = Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
        let report = check(repo_root);
        assert!(
            report.valid,
            "shipped cores.registry.json must be valid: {report:?}"
        );
        assert!(report.core_count >= 13, "expected manifesto P0/P1 set");
    }
}
