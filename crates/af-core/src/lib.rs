// SPDX-License-Identifier: Apache-2.0
use af_manifest::{CoreManifest, ManifestError};
use af_rtl_inspector::{inspect_core, RtlInspectionReport, RtlInspectorError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const APPROVED_CORE_LICENSE: &str = "AccelFury Source Available License v1.0";
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("missing manifest `{path}`")]
    MissingManifest { path: PathBuf },
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    #[error(transparent)]
    Inspector(#[from] RtlInspectorError),
    #[error("core check failed")]
    CheckFailed { report: Box<CoreCheckReport> },
}

impl CoreError {
    pub fn code(&self) -> &'static str {
        match self {
            CoreError::MissingManifest { .. } => "AF_MANIFEST_MISSING",
            CoreError::Manifest(err) => err.code(),
            CoreError::Inspector(err) => err.code(),
            CoreError::CheckFailed { .. } => "AF_CORE_CHECK_FAILED",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            CoreError::MissingManifest { .. } => {
                "Run this command from a core directory or pass a directory containing af-core.toml."
            }
            CoreError::Manifest(err) => err.hint(),
            CoreError::Inspector(err) => err.hint(),
            CoreError::CheckFailed { .. } => "Fix the listed core structure issues and retry.",
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            CoreError::Manifest(err) => err.exit_code(),
            CoreError::Inspector(err) => err.exit_code(),
            _ => 2,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreCheckReport {
    pub status: String,
    pub core_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub manifest: CoreManifest,
    pub inspection: RtlInspectionReport,
    #[serde(default)]
    pub legal_issues: Vec<CoreLegalIssue>,
    pub artifacts: Vec<PathBuf>,
    pub warnings: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CoreLegalIssue {
    pub code: String,
    pub message: String,
    pub hint: String,
}

impl CoreLegalIssue {
    fn new(code: &str, message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            hint: hint.into(),
        }
    }
}

impl CoreCheckReport {
    pub fn passed(&self) -> bool {
        self.status == "passed"
    }
}

pub fn manifest_path_for(core_dir: impl AsRef<Path>) -> PathBuf {
    core_dir.as_ref().join("af-core.toml")
}

pub fn load_manifest_from_core_dir(core_dir: impl AsRef<Path>) -> Result<CoreManifest, CoreError> {
    let manifest_path = manifest_path_for(core_dir);
    if !manifest_path.is_file() {
        return Err(CoreError::MissingManifest {
            path: manifest_path,
        });
    }
    Ok(CoreManifest::from_path(manifest_path)?)
}

/// Parse and structurally validate a core manifest (manifest + RTL inspection).
///
/// Unlike `check_core`, this skips legal-policy validation so consumers that
/// only need a trustworthy manifest+source contract (compatibility check,
/// signoff plan, dependency graph) can fail-closed on broken structure without
/// also requiring the full LICENSE/COMMERCIAL-LICENSE.md set.
pub fn load_validated_manifest(core_dir: impl AsRef<Path>) -> Result<CoreManifest, CoreError> {
    let core_dir = core_dir.as_ref();
    let manifest_path = manifest_path_for(core_dir);
    if !manifest_path.is_file() {
        return Err(CoreError::MissingManifest {
            path: manifest_path,
        });
    }
    let manifest = CoreManifest::from_path(&manifest_path)?;
    let inspection = inspect_core(core_dir, &manifest)?;
    if inspection.has_errors() {
        let warnings = inspection.warnings();
        let limitations = manifest.known_limitations.clone();
        let report = CoreCheckReport {
            status: "failed".to_string(),
            core_dir: core_dir.to_path_buf(),
            manifest_path,
            manifest,
            inspection,
            legal_issues: Vec::new(),
            artifacts: Vec::new(),
            warnings,
            limitations,
        };
        return Err(CoreError::CheckFailed {
            report: Box::new(report),
        });
    }
    Ok(manifest)
}

pub fn check_core(core_dir: impl AsRef<Path>) -> Result<CoreCheckReport, CoreError> {
    let core_dir = core_dir.as_ref();
    let manifest_path = manifest_path_for(core_dir);
    if !manifest_path.is_file() {
        return Err(CoreError::MissingManifest {
            path: manifest_path,
        });
    }

    let manifest = CoreManifest::from_path(&manifest_path)?;
    let inspection = inspect_core(core_dir, &manifest)?;
    let legal_issues = validate_core_legal_policy(core_dir, &manifest);
    let warnings = inspection.warnings();
    let limitations = manifest.known_limitations.clone();
    let report = CoreCheckReport {
        status: if inspection.has_errors() || !legal_issues.is_empty() {
            "failed".to_string()
        } else {
            "passed".to_string()
        },
        core_dir: core_dir.to_path_buf(),
        manifest_path,
        manifest,
        inspection,
        legal_issues,
        artifacts: Vec::new(),
        warnings,
        limitations,
    };

    if report.passed() {
        Ok(report)
    } else {
        Err(CoreError::CheckFailed {
            report: Box::new(report),
        })
    }
}

fn validate_core_legal_policy(core_dir: &Path, manifest: &CoreManifest) -> Vec<CoreLegalIssue> {
    let mut issues = Vec::new();
    match manifest.metadata.license.as_deref().map(str::trim) {
        Some(APPROVED_CORE_LICENSE) => {}
        Some(other) => issues.push(CoreLegalIssue::new(
            "AF_LEGAL_LICENSE_POLICY_MISMATCH",
            format!(
                "metadata.license `{other}` does not match approved policy `{APPROVED_CORE_LICENSE}`"
            ),
            "Set [metadata].license to the approved AccelFury source-available license policy.",
        )),
        None => issues.push(CoreLegalIssue::new(
            "AF_LEGAL_LICENSE_POLICY_MISSING",
            "metadata.license is missing",
            "Set [metadata].license to the approved AccelFury source-available license policy.",
        )),
    }

    let license_text = read_required_legal_file(core_dir, "LICENSE", &mut issues);
    let commercial_text = read_required_legal_file(core_dir, "COMMERCIAL-LICENSE.md", &mut issues);

    for (file_name, text) in [
        ("LICENSE", license_text.as_deref()),
        ("COMMERCIAL-LICENSE.md", commercial_text.as_deref()),
    ] {
        if let Some(text) = text {
            let lower = text.to_ascii_lowercase();
            if ["placeholder", "tbd", "not confirmed", "not_confirmed"]
                .iter()
                .any(|marker| lower.contains(marker))
            {
                issues.push(CoreLegalIssue::new(
                    "AF_LEGAL_PLACEHOLDER_TEXT",
                    format!("{file_name} contains placeholder or unapproved legal text"),
                    "Replace placeholder legal text with the approved AccelFury license policy.",
                ));
            }
        }
    }

    if let Some(text) = commercial_text {
        let lower = text.to_ascii_lowercase();
        let has_paid_commercial_license =
            lower.contains("separate") && lower.contains("commercial license");
        let has_closed_source_trigger =
            lower.contains("closed-source") || lower.contains("proprietary");
        let has_support_boundary = lower.contains("support") && lower.contains("warranty");
        if !(has_paid_commercial_license && has_closed_source_trigger && has_support_boundary) {
            issues.push(CoreLegalIssue::new(
                "AF_LEGAL_COMMERCIAL_BOUNDARY_INCOMPLETE",
                "COMMERCIAL-LICENSE.md does not describe commercial/support boundary",
                "State that closed-source/commercial use needs a separate paid commercial license and describe support/warranty boundaries.",
            ));
        }
    }

    issues
}

fn read_required_legal_file(
    core_dir: &Path,
    file_name: &str,
    issues: &mut Vec<CoreLegalIssue>,
) -> Option<String> {
    let path = core_dir.join(file_name);
    if !path.is_file() {
        issues.push(CoreLegalIssue::new(
            "AF_LEGAL_FILE_MISSING",
            format!("required legal file `{file_name}` is missing"),
            "Add LICENSE and COMMERCIAL-LICENSE.md before running core check or release gates.",
        ));
        return None;
    }
    match std::fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(err) => {
            issues.push(CoreLegalIssue::new(
                "AF_LEGAL_FILE_READ_FAILED",
                format!("failed to read `{file_name}`: {err}"),
                "Check file permissions and encoding.",
            ));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_manifest(dir: &Path) {
        fs::write(
            dir.join("af-core.toml"),
            r#"
af_version = "0.1"
name = "demo"
vendor = "accelfury"
library = "ip"
core = "demo"
version = "0.1.0"
known_limitations = ["test limitation"]

[metadata]
license = "AccelFury Source Available License v1.0"

[rtl]
top = "demo"
language = "verilog-2001"

[sources]
files = ["rtl/demo.v"]

[[ports]]
name = "clk"
direction = "input"
width = 1

[[ports]]
name = "done"
direction = "output"
width = 1
"#,
        )
        .unwrap();
    }

    fn write_legal_files(dir: &Path) {
        fs::write(
            dir.join("LICENSE"),
            "AccelFury Source Available License v1.0\n\nCopyright (c) 2026 AccelFury.\n",
        )
        .unwrap();
        fs::write(
            dir.join("COMMERCIAL-LICENSE.md"),
            "# Commercial Licensing\n\nClosed-source and commercial use requires a separate paid commercial license from AccelFury.\nCommercial triggers include closed-source FPGA products and proprietary repositories.\nContact AccelFury for commercial terms, support, warranty options, and custom integration work.\n",
        )
        .unwrap();
    }

    #[test]
    fn passes_valid_core() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(
            dir.path().join("rtl/demo.v"),
            r#"`default_nettype none
module demo (
  input wire clk,
  output reg done
);
  always @(posedge clk) begin
    done <= 1'b1;
  end
endmodule
`default_nettype wire
"#,
        )
        .unwrap();
        write_manifest(dir.path());
        write_legal_files(dir.path());

        let report = check_core(dir.path()).unwrap();
        assert!(report.passed());
        assert_eq!(report.limitations, vec!["test limitation"]);
    }

    #[test]
    fn fails_placeholder_legal_text() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(
            dir.path().join("rtl/demo.v"),
            r#"`default_nettype none
module demo (
  input wire clk,
  output reg done
);
  always @(posedge clk) begin
    done <= 1'b1;
  end
endmodule
`default_nettype wire
"#,
        )
        .unwrap();
        write_manifest(dir.path());
        fs::write(dir.path().join("LICENSE"), "TBD placeholder").unwrap();
        fs::write(
            dir.path().join("COMMERCIAL-LICENSE.md"),
            "Commercial license not confirmed",
        )
        .unwrap();

        let err = check_core(dir.path()).unwrap_err();
        assert_eq!(err.code(), "AF_CORE_CHECK_FAILED");
        if let CoreError::CheckFailed { report } = err {
            assert!(report
                .legal_issues
                .iter()
                .any(|issue| issue.code == "AF_LEGAL_PLACEHOLDER_TEXT"));
        } else {
            panic!("expected check failure report");
        }
    }

    #[test]
    fn fails_missing_top() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(
            dir.path().join("rtl/demo.v"),
            r#"`default_nettype none
module other (
  input wire clk,
  output reg done
);
endmodule
`default_nettype wire
"#,
        )
        .unwrap();
        write_manifest(dir.path());
        write_legal_files(dir.path());

        let err = check_core(dir.path()).unwrap_err();
        assert_eq!(err.code(), "AF_CORE_CHECK_FAILED");
    }

    #[test]
    fn load_validated_manifest_accepts_valid_core_without_legal_files() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(
            dir.path().join("rtl/demo.v"),
            r#"`default_nettype none
module demo (
  input wire clk,
  output reg done
);
  always @(posedge clk) begin
    done <= 1'b1;
  end
endmodule
`default_nettype wire
"#,
        )
        .unwrap();
        write_manifest(dir.path());
        // Intentionally no LICENSE / COMMERCIAL-LICENSE.md — should still pass.

        let manifest = load_validated_manifest(dir.path()).unwrap();
        assert_eq!(manifest.core, "demo");
    }

    #[test]
    fn load_validated_manifest_rejects_missing_source() {
        let dir = tempdir().unwrap();
        // No rtl/demo.v on disk: inspect_core must reject this.
        write_manifest(dir.path());
        let err = load_validated_manifest(dir.path()).unwrap_err();
        assert_eq!(err.code(), "AF_CORE_CHECK_FAILED");
        match err {
            CoreError::CheckFailed { report } => {
                assert_eq!(report.status, "failed");
                assert!(report.legal_issues.is_empty());
                assert!(report.inspection.has_errors());
            }
            other => panic!("expected CheckFailed, got {other:?}"),
        }
    }

    #[test]
    fn load_validated_manifest_reports_missing_manifest() {
        let dir = tempdir().unwrap();
        let err = load_validated_manifest(dir.path()).unwrap_err();
        assert_eq!(err.code(), "AF_MANIFEST_MISSING");
    }
}
