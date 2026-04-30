use af_manifest::CoreManifest;
use af_security::{safe_join, SecurityError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RtlInspectorError {
    #[error(transparent)]
    Security(#[from] SecurityError),
    #[error("failed to read RTL file `{path}`: {message}")]
    Read { path: PathBuf, message: String },
}

impl RtlInspectorError {
    pub fn code(&self) -> &'static str {
        match self {
            RtlInspectorError::Security(err) => err.code(),
            RtlInspectorError::Read { .. } => "AF_RTL_READ_FAILED",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            RtlInspectorError::Security(err) => err.hint(),
            RtlInspectorError::Read { .. } => "Check that the declared RTL file is readable.",
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            RtlInspectorError::Security(err) => err.exit_code(),
            RtlInspectorError::Read { .. } => 2,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum RtlIssueSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RtlIssue {
    pub severity: RtlIssueSeverity,
    pub code: String,
    pub message: String,
    pub hint: String,
}

impl RtlIssue {
    fn error(code: &str, message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            severity: RtlIssueSeverity::Error,
            code: code.to_string(),
            message: message.into(),
            hint: hint.into(),
        }
    }

    fn warning(code: &str, message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            severity: RtlIssueSeverity::Warning,
            code: code.to_string(),
            message: message.into(),
            hint: hint.into(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct RtlInspectionReport {
    pub scanned_files: Vec<PathBuf>,
    pub issues: Vec<RtlIssue>,
}

impl RtlInspectionReport {
    pub fn has_errors(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == RtlIssueSeverity::Error)
    }

    pub fn warnings(&self) -> Vec<String> {
        self.issues
            .iter()
            .filter(|issue| issue.severity == RtlIssueSeverity::Warning)
            .map(|issue| issue.message.clone())
            .collect()
    }
}

pub fn inspect_core(
    core_dir: impl AsRef<Path>,
    manifest: &CoreManifest,
) -> Result<RtlInspectionReport, RtlInspectorError> {
    let core_dir = core_dir.as_ref();
    let mut report = RtlInspectionReport::default();
    let mut source_text = String::new();

    for include_dir in &manifest.sources.include_dirs {
        let path = safe_join(core_dir, include_dir)?;
        if !path.is_dir() {
            report.issues.push(RtlIssue::warning(
                "AF_INCLUDE_DIR_MISSING",
                format!("include directory `{include_dir}` does not exist"),
                "Create the include directory or remove it from sources.include_dirs.",
            ));
        }
    }

    for source in &manifest.sources.files {
        let path = safe_join(core_dir, source)?;
        if !path.is_file() {
            report.issues.push(RtlIssue::error(
                "AF_SOURCE_MISSING",
                format!("source file `{source}` does not exist"),
                "Create the file or update sources.files in af-core.toml.",
            ));
            continue;
        }
        report.scanned_files.push(path.clone());
        let text = fs::read_to_string(&path).map_err(|err| RtlInspectorError::Read {
            path,
            message: err.to_string(),
        })?;
        source_text.push_str(&text);
        source_text.push('\n');
    }

    for testbench in &manifest.testbenches {
        for source in &testbench.sources {
            let path = safe_join(core_dir, source)?;
            if !path.is_file() {
                report.issues.push(RtlIssue::error(
                    "AF_TESTBENCH_SOURCE_MISSING",
                    format!(
                        "testbench `{}` source file `{source}` does not exist",
                        testbench.name
                    ),
                    "Create the file or update the testbench sources list.",
                ));
            }
        }
    }

    if !source_text.is_empty() && !top_appears_in_source(&source_text, manifest) {
        report.issues.push(RtlIssue::error(
            "AF_TOP_MISSING",
            format!(
                "top `{}` was not found in declared RTL sources",
                manifest.rtl.top
            ),
            "Ensure rtl.top matches a module/entity declared in sources.files.",
        ));
    }

    Ok(report)
}

fn top_appears_in_source(source_text: &str, manifest: &CoreManifest) -> bool {
    match manifest.rtl.language.as_str() {
        "vhdl" => contains_token_sequence(source_text, "entity", &manifest.rtl.top),
        _ => contains_token_sequence(source_text, "module", &manifest.rtl.top),
    }
}

fn contains_token_sequence(source_text: &str, first: &str, second: &str) -> bool {
    let mut previous = "";
    for token in source_text
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
        .filter(|token| !token.is_empty())
    {
        if previous == first && token == second {
            return true;
        }
        previous = token;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn manifest(top: &str) -> CoreManifest {
        CoreManifest::from_toml_str(
            &format!(
                r#"
af_version = "0.1"
name = "demo"
vendor = "accelfury"
library = "ip"
core = "demo"
version = "0.1.0"

[rtl]
top = "{top}"

[sources]
files = ["rtl/demo.sv"]
"#
            ),
            "af-core.toml",
        )
        .unwrap()
    }

    #[test]
    fn finds_declared_top() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(dir.path().join("rtl/demo.sv"), "module demo; endmodule\n").unwrap();
        let report = inspect_core(dir.path(), &manifest("demo")).unwrap();
        assert!(!report.has_errors());
    }

    #[test]
    fn reports_missing_source() {
        let dir = tempdir().unwrap();
        let report = inspect_core(dir.path(), &manifest("demo")).unwrap();
        assert!(report.has_errors());
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "AF_SOURCE_MISSING"));
    }

    #[test]
    fn reports_missing_top() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(dir.path().join("rtl/demo.sv"), "module other; endmodule\n").unwrap();
        let report = inspect_core(dir.path(), &manifest("demo")).unwrap();
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "AF_TOP_MISSING"));
    }
}
