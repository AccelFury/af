// SPDX-License-Identifier: Apache-2.0
use af_backend_fusesoc::{write_core, FuseSocError};
use af_backend_litex::generate_litex_skeleton;
use af_core::{check_core, CoreError};
use af_security::{safe_join, SecurityError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum WrapperTarget {
    FuseSoc,
    LiteX,
}

impl WrapperTarget {
    pub fn parse(input: &str) -> Result<Self, WrapperGenError> {
        match input {
            "fusesoc" => Ok(Self::FuseSoc),
            "litex" => Ok(Self::LiteX),
            other => Err(WrapperGenError::UnsupportedTarget {
                target: other.to_string(),
            }),
        }
    }
}

#[derive(Debug, Error)]
pub enum WrapperGenError {
    #[error("unsupported wrapper target `{target}`")]
    UnsupportedTarget { target: String },
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error(transparent)]
    FuseSoc(#[from] FuseSocError),
    #[error(transparent)]
    Security(#[from] SecurityError),
    #[error("failed to write LiteX wrapper `{path}`: {message}")]
    Write { path: PathBuf, message: String },
}

impl WrapperGenError {
    pub fn code(&self) -> &'static str {
        match self {
            WrapperGenError::UnsupportedTarget { .. } => "AF_WRAPPER_TARGET_UNSUPPORTED",
            WrapperGenError::Core(err) => err.code(),
            WrapperGenError::FuseSoc(err) => err.code(),
            WrapperGenError::Security(err) => err.code(),
            WrapperGenError::Write { .. } => "AF_WRAPPER_WRITE_FAILED",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            WrapperGenError::UnsupportedTarget { .. } => "Use --target fusesoc or --target litex.",
            WrapperGenError::Core(err) => err.hint(),
            WrapperGenError::FuseSoc(err) => err.hint(),
            WrapperGenError::Security(err) => err.hint(),
            WrapperGenError::Write { .. } => {
                "Check filesystem permissions and the selected build root."
            }
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            WrapperGenError::UnsupportedTarget { .. } => 2,
            WrapperGenError::Core(err) => err.exit_code(),
            WrapperGenError::FuseSoc(err) => err.exit_code(),
            WrapperGenError::Security(err) => err.exit_code(),
            WrapperGenError::Write { .. } => 5,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct WrapperReport {
    pub target: WrapperTarget,
    pub artifacts: Vec<PathBuf>,
    pub warnings: Vec<String>,
    pub limitations: Vec<String>,
}

pub fn generate_wrapper(
    core_dir: impl AsRef<Path>,
    build_root: impl AsRef<Path>,
    target: WrapperTarget,
    board: Option<&str>,
) -> Result<WrapperReport, WrapperGenError> {
    let core_report = check_core(core_dir)?;
    let build_root = build_root.as_ref();
    match target {
        WrapperTarget::FuseSoc => {
            let output_dir = build_root.join("fusesoc");
            let artifact = write_core(&core_report.manifest, output_dir)?;
            Ok(WrapperReport {
                target,
                artifacts: vec![artifact.path],
                warnings: core_report.warnings,
                limitations: core_report.limitations,
            })
        }
        WrapperTarget::LiteX => {
            let output_dir = build_root.join("litex");
            fs::create_dir_all(&output_dir).map_err(|err| WrapperGenError::Write {
                path: output_dir.clone(),
                message: err.to_string(),
            })?;
            let skeleton = generate_litex_skeleton(&core_report.manifest, board);
            let output = safe_join(&output_dir, &skeleton.file_name)?;
            fs::write(&output, &skeleton.content).map_err(|err| WrapperGenError::Write {
                path: output.clone(),
                message: err.to_string(),
            })?;
            let mut warnings = core_report.warnings;
            warnings.extend(skeleton.warnings);
            let mut limitations = core_report.limitations;
            limitations.extend(skeleton.limitations);
            Ok(WrapperReport {
                target,
                artifacts: vec![output],
                warnings,
                limitations,
            })
        }
    }
}
