// SPDX-License-Identifier: Apache-2.0
pub use af_security::{CommandOutput, CommandRunner, CommandSpec, ProcessCommandRunner};

use af_manifest::CoreManifest;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct BackendId(pub String);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BackendDiagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ToolInfo {
    pub backend_id: BackendId,
    pub tool_name: String,
    pub executable: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub available: bool,
    #[serde(default)]
    pub diagnostics: Vec<BackendDiagnostic>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BackendCapability {
    pub name: String,
    pub supported: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum BackendTarget {
    #[default]
    Lint,
    Simulate,
    Package,
    GenerateWrapper,
    Synthesize,
    BuildBitstream,
    Flash,
    Formal,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PreparedRun {
    pub backend_id: BackendId,
    pub working_dir: PathBuf,
    #[serde(default)]
    pub commands: Vec<CommandSpec>,
    #[serde(default)]
    pub expected_artifacts: Vec<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExecutedCommand {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub exit_code: Option<i32>,
    pub stdout_log: PathBuf,
    pub stderr_log: PathBuf,
    pub duration_ms: u64,
}

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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub allow_network: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
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
            env: output.spec.env,
            allow_network: output.spec.allow_network,
            timeout_seconds: output.spec.timeout_seconds,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct BuildPlan {
    #[serde(default)]
    pub core_root: PathBuf,
    #[serde(default)]
    pub build_root: PathBuf,
    #[serde(default)]
    pub core_ref: String,
    #[serde(default)]
    pub board_id: Option<String>,
    #[serde(default)]
    pub target: BackendTarget,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub options: BTreeMap<String, String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_id: Option<BackendId>,
    pub status: BackendStatus,
    #[serde(default)]
    pub tool_versions: Vec<ToolVersion>,
    #[serde(default)]
    pub tool_info: Vec<ToolInfo>,
    #[serde(default)]
    pub commands: Vec<CommandRecord>,
    #[serde(default)]
    pub commands_executed: Vec<ExecutedCommand>,
    #[serde(default)]
    pub artifacts: Vec<PathBuf>,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub limitations: Vec<String>,
    #[serde(default)]
    pub diagnostics: Vec<BackendDiagnostic>,
    #[serde(default)]
    pub metrics: BTreeMap<String, String>,
}

impl BackendReport {
    pub fn new(backend: impl Into<String>, status: BackendStatus) -> Self {
        Self {
            backend: backend.into(),
            backend_id: None,
            status,
            tool_versions: Vec::new(),
            tool_info: Vec::new(),
            commands: Vec::new(),
            commands_executed: Vec::new(),
            artifacts: Vec::new(),
            warnings: Vec::new(),
            limitations: Vec::new(),
            diagnostics: Vec::new(),
            metrics: BTreeMap::new(),
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
    fn id(&self) -> BackendId {
        BackendId(self.name().to_string())
    }
    fn capabilities(&self) -> Vec<BackendCapability> {
        Vec::new()
    }
    fn probe(&self, _plan: &BuildPlan) -> Result<ToolInfo, BackendError> {
        Err(BackendError::Unsupported {
            backend: self.name().to_string(),
        })
    }
    fn prepare(&self, _plan: &BuildPlan) -> Result<PreparedRun, BackendError> {
        Err(BackendError::Unsupported {
            backend: self.name().to_string(),
        })
    }
    fn run(
        &self,
        _prepared: &PreparedRun,
        _runner: &dyn CommandRunner,
    ) -> Result<BackendReport, BackendError> {
        Err(BackendError::Unsupported {
            backend: self.name().to_string(),
        })
    }
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
