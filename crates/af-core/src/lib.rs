// SPDX-License-Identifier: Apache-2.0
use af_manifest::{CoreManifest, ManifestError};
use af_rtl_inspector::{inspect_core, RtlInspectionReport, RtlInspectorError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
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
    pub artifacts: Vec<PathBuf>,
    pub warnings: Vec<String>,
    pub limitations: Vec<String>,
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
    let warnings = inspection.warnings();
    let limitations = manifest.known_limitations.clone();
    let report = CoreCheckReport {
        status: if inspection.has_errors() {
            "failed".to_string()
        } else {
            "passed".to_string()
        },
        core_dir: core_dir.to_path_buf(),
        manifest_path,
        manifest,
        inspection,
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

        let report = check_core(dir.path()).unwrap();
        assert!(report.passed());
        assert_eq!(report.limitations, vec!["test limitation"]);
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

        let err = check_core(dir.path()).unwrap_err();
        assert_eq!(err.code(), "AF_CORE_CHECK_FAILED");
    }
}
