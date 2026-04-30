use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("path `{path}` is empty")]
    EmptyPath { path: String },
    #[error("absolute paths are not allowed: `{path}`")]
    AbsolutePath { path: String },
    #[error("path traversal is not allowed: `{path}`")]
    PathTraversal { path: String },
    #[error("path prefix is not allowed: `{path}`")]
    PathPrefix { path: String },
    #[error("command program is empty")]
    EmptyProgram,
    #[error("command program contains a NUL byte")]
    InvalidProgram,
    #[error("command `{program}` is unavailable: {message}")]
    CommandUnavailable { program: String, message: String },
    #[error("failed to execute command `{program}`: {message}")]
    CommandExecution { program: String, message: String },
    #[error("failed to create directory `{path}`: {message}")]
    CreateDir { path: PathBuf, message: String },
    #[error("failed to write file `{path}`: {message}")]
    WriteFile { path: PathBuf, message: String },
}

impl SecurityError {
    pub fn code(&self) -> &'static str {
        match self {
            SecurityError::EmptyPath { .. } => "AF_PATH_EMPTY",
            SecurityError::AbsolutePath { .. } => "AF_PATH_ABSOLUTE",
            SecurityError::PathTraversal { .. } => "AF_PATH_TRAVERSAL",
            SecurityError::PathPrefix { .. } => "AF_PATH_PREFIX",
            SecurityError::EmptyProgram => "AF_COMMAND_EMPTY",
            SecurityError::InvalidProgram => "AF_COMMAND_INVALID",
            SecurityError::CommandUnavailable { .. } => "AF_BACKEND_UNAVAILABLE",
            SecurityError::CommandExecution { .. } => "AF_COMMAND_EXECUTION_FAILED",
            SecurityError::CreateDir { .. } => "AF_CREATE_DIR_FAILED",
            SecurityError::WriteFile { .. } => "AF_WRITE_FILE_FAILED",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            SecurityError::EmptyPath { .. } => "Provide a non-empty relative path.",
            SecurityError::AbsolutePath { .. } => "Use a path relative to the core directory.",
            SecurityError::PathTraversal { .. } => "Remove `..` segments from the manifest path.",
            SecurityError::PathPrefix { .. } => {
                "Use portable relative paths without drive prefixes."
            }
            SecurityError::EmptyProgram | SecurityError::InvalidProgram => {
                "Use a concrete executable name and pass arguments separately."
            }
            SecurityError::CommandUnavailable { .. } => {
                "Install the backend tool or remove that backend from the requested command."
            }
            SecurityError::CommandExecution { .. } => {
                "Check executable permissions, PATH, and the requested working directory."
            }
            SecurityError::CreateDir { .. } | SecurityError::WriteFile { .. } => {
                "Check filesystem permissions and the selected build root."
            }
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            SecurityError::CommandUnavailable { .. } => 4,
            SecurityError::CreateDir { .. } | SecurityError::WriteFile { .. } => 5,
            _ => 2,
        }
    }
}

pub fn normalize_relative_path(path: &str) -> Result<PathBuf, SecurityError> {
    if path.trim().is_empty() {
        return Err(SecurityError::EmptyPath {
            path: path.to_string(),
        });
    }

    let parsed = Path::new(path);
    if parsed.is_absolute() {
        return Err(SecurityError::AbsolutePath {
            path: path.to_string(),
        });
    }

    let mut normalized = PathBuf::new();
    for component in parsed.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(SecurityError::PathTraversal {
                    path: path.to_string(),
                });
            }
            Component::RootDir => {
                return Err(SecurityError::AbsolutePath {
                    path: path.to_string(),
                });
            }
            Component::Prefix(_) => {
                return Err(SecurityError::PathPrefix {
                    path: path.to_string(),
                });
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        return Err(SecurityError::EmptyPath {
            path: path.to_string(),
        });
    }

    Ok(normalized)
}

pub fn safe_join(base: impl AsRef<Path>, relative: &str) -> Result<PathBuf, SecurityError> {
    Ok(base.as_ref().join(normalize_relative_path(relative)?))
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<PathBuf>,
}

impl CommandSpec {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            cwd: None,
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CommandOutput {
    pub spec: CommandSpec,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub trait CommandRunner: Send + Sync {
    fn run(&self, spec: &CommandSpec) -> Result<CommandOutput, SecurityError>;
}

#[derive(Clone, Debug, Default)]
pub struct ProcessCommandRunner;

impl ProcessCommandRunner {
    fn validate_program(program: &str) -> Result<(), SecurityError> {
        if program.is_empty() {
            return Err(SecurityError::EmptyProgram);
        }
        if program.as_bytes().contains(&0) {
            return Err(SecurityError::InvalidProgram);
        }
        Ok(())
    }
}

impl CommandRunner for ProcessCommandRunner {
    fn run(&self, spec: &CommandSpec) -> Result<CommandOutput, SecurityError> {
        Self::validate_program(&spec.program)?;

        let mut command = Command::new(OsStr::new(&spec.program));
        command.args(spec.args.iter().map(OsStr::new));
        if let Some(cwd) = &spec.cwd {
            command.current_dir(cwd);
        }

        let output = command.output().map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => SecurityError::CommandUnavailable {
                program: spec.program.clone(),
                message: err.to_string(),
            },
            _ => SecurityError::CommandExecution {
                program: spec.program.clone(),
                message: err.to_string(),
            },
        })?;

        Ok(CommandOutput {
            spec: spec.clone(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_safe_relative_paths() {
        assert_eq!(
            normalize_relative_path("./rtl/core.sv").unwrap(),
            PathBuf::from("rtl/core.sv")
        );
    }

    #[test]
    fn rejects_traversal() {
        let err = normalize_relative_path("../secret.sv").unwrap_err();
        assert_eq!(err.code(), "AF_PATH_TRAVERSAL");
    }

    #[test]
    fn rejects_absolute_paths() {
        let err = normalize_relative_path("/tmp/core.sv").unwrap_err();
        assert_eq!(err.code(), "AF_PATH_ABSOLUTE");
    }
}
