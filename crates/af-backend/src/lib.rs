pub use af_security::{CommandOutput, CommandRunner, CommandSpec, ProcessCommandRunner};

use af_manifest::CoreManifest;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum BackendStatus {
    Passed,
    Failed,
    Unavailable,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ToolVersion {
    pub tool: String,
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl ToolVersion {
    pub fn available(tool: &str, version: impl Into<String>) -> Self {
        Self {
            tool: tool.to_string(),
            available: true,
            version: Some(version.into()),
            message: None,
        }
    }

    pub fn unavailable(tool: &str, message: impl Into<String>) -> Self {
        Self {
            tool: tool.to_string(),
            available: false,
            version: None,
            message: Some(message.into()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CommandRecord {
    pub program: String,
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<PathBuf>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl From<CommandOutput> for CommandRecord {
    fn from(output: CommandOutput) -> Self {
        Self {
            program: output.spec.program,
            args: output.spec.args,
            cwd: output.spec.cwd,
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: output.stderr,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct BuildPlan {
    #[serde(default)]
    pub commands: Vec<CommandSpec>,
    #[serde(default)]
    pub artifacts: Vec<PathBuf>,
    #[serde(default)]
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BackendReport {
    pub backend: String,
    pub status: BackendStatus,
    #[serde(default)]
    pub tool_versions: Vec<ToolVersion>,
    #[serde(default)]
    pub commands: Vec<CommandRecord>,
    #[serde(default)]
    pub artifacts: Vec<PathBuf>,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub limitations: Vec<String>,
}

impl BackendReport {
    pub fn new(backend: impl Into<String>, status: BackendStatus) -> Self {
        Self {
            backend: backend.into(),
            status,
            tool_versions: Vec::new(),
            commands: Vec::new(),
            artifacts: Vec::new(),
            warnings: Vec::new(),
            limitations: Vec::new(),
        }
    }

    pub fn unavailable(
        backend: impl Into<String>,
        tool: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        let tool = tool.into();
        let message = message.into();
        let mut report = Self::new(backend, BackendStatus::Unavailable);
        report
            .tool_versions
            .push(ToolVersion::unavailable(&tool, message.clone()));
        report
            .warnings
            .push(format!("{tool} is unavailable: {message}"));
        report
    }
}

#[derive(Debug, Error)]
pub enum BackendError {
    #[error("backend `{backend}` is unsupported")]
    Unsupported { backend: String },
    #[error("backend `{backend}` is unavailable: {message}")]
    Unavailable { backend: String, message: String },
    #[error("backend `{backend}` failed: {message}")]
    Failed { backend: String, message: String },
    #[error(transparent)]
    Security(#[from] af_security::SecurityError),
}

impl BackendError {
    pub fn code(&self) -> &'static str {
        match self {
            BackendError::Unsupported { .. } => "AF_BACKEND_UNSUPPORTED",
            BackendError::Unavailable { .. } => "AF_BACKEND_UNAVAILABLE",
            BackendError::Failed { .. } => "AF_BACKEND_FAILED",
            BackendError::Security(err) => err.code(),
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            BackendError::Unsupported { .. } => "Use one of the supported MVP backends.",
            BackendError::Unavailable { .. } => {
                "Install the backend tool or use a fake/test backend."
            }
            BackendError::Failed { .. } => "Inspect the backend command stderr in the report.",
            BackendError::Security(err) => err.hint(),
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            BackendError::Unavailable { .. } => 4,
            BackendError::Security(err) => err.exit_code(),
            _ => 3,
        }
    }
}

pub trait AfBackend {
    fn name(&self) -> &'static str;
    fn doctor(&self) -> Result<BackendReport, BackendError>;
    fn lint(
        &self,
        manifest: &CoreManifest,
        core_dir: &Path,
        build_root: &Path,
    ) -> Result<BackendReport, BackendError>;
    fn sim(
        &self,
        manifest: &CoreManifest,
        core_dir: &Path,
        build_root: &Path,
    ) -> Result<BackendReport, BackendError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use af_security::{CommandOutput, SecurityError};
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct FakeRunner {
        seen: Arc<Mutex<Vec<CommandSpec>>>,
    }

    impl CommandRunner for FakeRunner {
        fn run(&self, spec: &CommandSpec) -> Result<CommandOutput, SecurityError> {
            self.seen.lock().unwrap().push(spec.clone());
            Ok(CommandOutput {
                spec: spec.clone(),
                exit_code: Some(0),
                stdout: "fake 1.0".to_string(),
                stderr: String::new(),
            })
        }
    }

    #[test]
    fn fake_backend_runner_records_argv_without_shell() {
        let runner = FakeRunner::default();
        let spec = CommandSpec::new("fake-tool").args(["--version"]);
        let output = runner.run(&spec).unwrap();
        let record = CommandRecord::from(output);
        assert_eq!(record.program, "fake-tool");
        assert_eq!(record.args, vec!["--version"]);
        assert!(!record.args.join(" ").contains(';'));
    }
}
