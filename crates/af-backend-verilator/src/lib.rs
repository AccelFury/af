// SPDX-License-Identifier: Apache-2.0
use af_backend::{
    AfBackend, BackendCapability, BackendError, BackendId, BackendReport, BackendStatus,
    CommandRecord, CommandRunner, CommandSpec, ProcessCommandRunner, ToolInfo, ToolVersion,
};
use af_manifest::CoreManifest;
use af_security::SecurityError;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct VerilatorBackend<R = ProcessCommandRunner> {
    runner: R,
}

impl VerilatorBackend<ProcessCommandRunner> {
    pub fn process() -> Self {
        Self {
            runner: ProcessCommandRunner,
        }
    }
}

impl<R> VerilatorBackend<R>
where
    R: CommandRunner,
{
    pub fn new(runner: R) -> Self {
        Self { runner }
    }

    fn version_command(&self) -> CommandSpec {
        CommandSpec::new("verilator").arg("--version")
    }

    fn probe_version(&self) -> (ToolVersion, Vec<CommandRecord>) {
        let spec = self.version_command();
        match self.runner.run(&spec) {
            Ok(output) => {
                let version_text = first_non_empty_line(&output.stdout)
                    .or_else(|| first_non_empty_line(&output.stderr))
                    .unwrap_or("verilator version output was empty")
                    .to_string();
                let record = CommandRecord::from(output);
                (
                    ToolVersion::available("verilator", version_text),
                    vec![record],
                )
            }
            Err(SecurityError::CommandUnavailable { message, .. }) => {
                (ToolVersion::unavailable("verilator", message), Vec::new())
            }
            Err(err) => (
                ToolVersion::unavailable("verilator", err.to_string()),
                Vec::new(),
            ),
        }
    }
}

impl<R> AfBackend for VerilatorBackend<R>
where
    R: CommandRunner,
{
    fn name(&self) -> &'static str {
        "verilator"
    }

    fn capabilities(&self) -> Vec<BackendCapability> {
        vec![
            BackendCapability {
                name: "lint".to_string(),
                supported: true,
                detail: Some("Runs verilator --lint-only over declared RTL sources.".to_string()),
            },
            BackendCapability {
                name: "smoke-sim".to_string(),
                supported: true,
                detail: Some(
                    "MVP smoke path validates declared testbench sources with Verilator."
                        .to_string(),
                ),
            },
        ]
    }

    fn probe(&self, _plan: &af_backend::BuildPlan) -> Result<ToolInfo, BackendError> {
        let (version, _) = self.probe_version();
        Ok(ToolInfo {
            backend_id: BackendId(self.name().to_string()),
            tool_name: "verilator".to_string(),
            executable: "verilator".into(),
            version: version.version,
            available: version.available,
            diagnostics: Vec::new(),
        })
    }

    fn doctor(&self) -> Result<BackendReport, BackendError> {
        let (version, commands) = self.probe_version();
        let mut report = BackendReport::new(
            self.name(),
            if version.available {
                BackendStatus::Passed
            } else {
                BackendStatus::Unavailable
            },
        );
        report.tool_versions.push(version);
        report.commands.extend(commands);
        if !matches!(report.status, BackendStatus::Passed) {
            report.warnings.push(
                "Verilator is not installed or not visible in PATH; lint/sim commands will report BackendUnavailable.".to_string(),
            );
        }
        Ok(report)
    }

    fn lint(
        &self,
        manifest: &CoreManifest,
        core_dir: &Path,
        build_root: &Path,
    ) -> Result<BackendReport, BackendError> {
        let (version, version_commands) = self.probe_version();
        if !version.available {
            let mut report = BackendReport::new(self.name(), BackendStatus::Unavailable);
            report.tool_versions.push(version);
            report.commands.extend(version_commands);
            report
                .warnings
                .push("Verilator backend unavailable".to_string());
            return Ok(report);
        }

        let spec = verilator_lint_command(manifest, core_dir);
        let mut report = BackendReport::new(self.name(), BackendStatus::Failed);
        report.tool_versions.push(version);
        report.commands.extend(version_commands);
        report.artifacts.push(build_root.to_path_buf());
        report
            .limitations
            .push("MVP lint delegates syntax and elaboration checks to Verilator; it is not a CDC/RDC/timing signoff.".to_string());

        let output = self.runner.run(&spec)?;
        let passed = output.exit_code == Some(0);
        report.commands.push(CommandRecord::from(output));
        report.status = if passed {
            BackendStatus::Passed
        } else {
            BackendStatus::Failed
        };
        if !passed {
            report
                .warnings
                .push("Verilator lint returned a non-zero exit code".to_string());
        }
        Ok(report)
    }

    fn sim(
        &self,
        manifest: &CoreManifest,
        core_dir: &Path,
        build_root: &Path,
    ) -> Result<BackendReport, BackendError> {
        let (version, version_commands) = self.probe_version();
        if !version.available {
            let mut report = BackendReport::new(self.name(), BackendStatus::Unavailable);
            report.tool_versions.push(version);
            report.commands.extend(version_commands);
            report
                .warnings
                .push("Verilator backend unavailable".to_string());
            return Ok(report);
        }

        let mut report = BackendReport::new(self.name(), BackendStatus::Passed);
        report.tool_versions.push(version);
        report.commands.extend(version_commands);
        report.artifacts.push(build_root.to_path_buf());
        report
            .limitations
            .push("MVP smoke simulation validates declared testbench sources with Verilator lint; it does not run a full behavioral regression.".to_string());

        if manifest.testbenches.is_empty() {
            report
                .warnings
                .push("No testbenches declared; smoke simulation skipped.".to_string());
            return Ok(report);
        }

        let spec = verilator_smoke_command(manifest, core_dir);
        let output = self.runner.run(&spec)?;
        let passed = output.exit_code == Some(0);
        report.commands.push(CommandRecord::from(output));
        report.status = if passed {
            BackendStatus::Passed
        } else {
            BackendStatus::Failed
        };
        if !passed {
            report
                .warnings
                .push("Verilator smoke simulation check returned a non-zero exit code".to_string());
        }
        Ok(report)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct VerilatorArgs {
    pub args: Vec<String>,
}

pub fn verilator_lint_command(manifest: &CoreManifest, core_dir: &Path) -> CommandSpec {
    let mut args = vec![
        "--lint-only".to_string(),
        "--top-module".to_string(),
        manifest.rtl.top.clone(),
    ];
    for include_dir in &manifest.sources.include_dirs {
        args.push(format!("-I{include_dir}"));
    }
    args.extend(manifest.sources.files.iter().cloned());

    CommandSpec::new("verilator").args(args).cwd(core_dir)
}

pub fn verilator_smoke_command(manifest: &CoreManifest, core_dir: &Path) -> CommandSpec {
    let top = manifest
        .testbenches
        .first()
        .map(|tb| tb.top.clone())
        .unwrap_or_else(|| manifest.rtl.top.clone());

    let has_cpp_testbench = manifest
        .testbenches
        .iter()
        .flat_map(|tb| tb.sources.iter())
        .any(|source| {
            source.ends_with(".cpp") || source.ends_with(".cc") || source.ends_with(".cxx")
        });
    let mut args = if has_cpp_testbench {
        vec![
            "--cc".to_string(),
            "--exe".to_string(),
            "--build".to_string(),
            "--timing".to_string(),
            "--top-module".to_string(),
            top,
        ]
    } else {
        vec![
            "--lint-only".to_string(),
            "--timing".to_string(),
            "--top-module".to_string(),
            top,
        ]
    };
    for include_dir in &manifest.sources.include_dirs {
        args.push(format!("-I{include_dir}"));
    }
    args.extend(manifest.sources.files.iter().cloned());
    for testbench in &manifest.testbenches {
        args.extend(testbench.sources.iter().cloned());
    }

    CommandSpec::new("verilator").args(args).cwd(core_dir)
}

fn first_non_empty_line(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|line| !line.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use af_security::{CommandOutput, SecurityError};
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct FakeRunner {
        outputs: Arc<Mutex<Vec<CommandOutput>>>,
    }

    impl FakeRunner {
        fn new(outputs: Vec<CommandOutput>) -> Self {
            Self {
                outputs: Arc::new(Mutex::new(outputs)),
            }
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(&self, spec: &CommandSpec) -> Result<CommandOutput, SecurityError> {
            let mut outputs = self.outputs.lock().unwrap();
            if outputs.is_empty() {
                return Ok(CommandOutput {
                    spec: spec.clone(),
                    exit_code: Some(0),
                    stdout: String::new(),
                    stderr: String::new(),
                });
            }
            let mut output = outputs.remove(0);
            output.spec = spec.clone();
            Ok(output)
        }
    }

    fn manifest() -> CoreManifest {
        CoreManifest::from_toml_str(
            r#"
af_version = "0.1"
name = "demo"
vendor = "accelfury"
library = "ip"
core = "demo"
version = "0.1.0"

[rtl]
top = "demo"
language = "systemverilog"

[sources]
files = ["rtl/demo.sv"]
"#,
            "af-core.toml",
        )
        .unwrap()
    }

    #[test]
    fn builds_lint_argv() {
        let spec = verilator_lint_command(&manifest(), Path::new("core"));
        assert_eq!(spec.program, "verilator");
        assert_eq!(spec.args[0], "--lint-only");
        assert!(spec.args.contains(&"--top-module".to_string()));
        assert!(spec.args.contains(&"rtl/demo.sv".to_string()));
    }

    #[test]
    fn returns_passed_report_from_fake_runner() {
        let version = CommandOutput {
            spec: CommandSpec::new("verilator"),
            exit_code: Some(0),
            stdout: "Verilator 5.000".to_string(),
            stderr: String::new(),
        };
        let lint = CommandOutput {
            spec: CommandSpec::new("verilator"),
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        };
        let backend = VerilatorBackend::new(FakeRunner::new(vec![version, lint]));
        let report = backend
            .lint(&manifest(), Path::new("."), Path::new(".af-build"))
            .unwrap();
        assert_eq!(report.status, BackendStatus::Passed);
        assert_eq!(report.commands.len(), 2);
    }
}
