use af_backend_fusesoc::{write_core, FuseSocError};
use af_core::{check_core, CoreError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum WrapperTarget {
    FuseSoc,
}

impl WrapperTarget {
    pub fn parse(input: &str) -> Result<Self, WrapperGenError> {
        match input {
            "fusesoc" => Ok(Self::FuseSoc),
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
}

impl WrapperGenError {
    pub fn code(&self) -> &'static str {
        match self {
            WrapperGenError::UnsupportedTarget { .. } => "AF_WRAPPER_TARGET_UNSUPPORTED",
            WrapperGenError::Core(err) => err.code(),
            WrapperGenError::FuseSoc(err) => err.code(),
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            WrapperGenError::UnsupportedTarget { .. } => "Use --target fusesoc for MVP.",
            WrapperGenError::Core(err) => err.hint(),
            WrapperGenError::FuseSoc(err) => err.hint(),
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            WrapperGenError::UnsupportedTarget { .. } => 2,
            WrapperGenError::Core(err) => err.exit_code(),
            WrapperGenError::FuseSoc(err) => err.exit_code(),
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
    }
}
