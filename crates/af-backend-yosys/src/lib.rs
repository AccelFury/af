// SPDX-License-Identifier: Apache-2.0
use af_backend::{
    AfBackend, BackendCapability, BackendError, BackendId, BackendReport, BackendStatus,
    CommandRecord, CommandRunner, CommandSpec, ProcessCommandRunner, ToolInfo, ToolVersion,
};
use af_manifest::CoreManifest;
use af_security::SecurityError;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct YosysBackend<R = ProcessCommandRunner> {
    runner: R,
}

impl YosysBackend<ProcessCommandRunner> {
    pub fn process() -> Self {
        Self {
            runner: ProcessCommandRunner,
        }
    }
}

impl<R> YosysBackend<R>
where
    R: CommandRunner,
{
    pub fn new(runner: R) -> Self {
        Self { runner }
    }

    fn probe_version(&self) -> (ToolVersion, Vec<CommandRecord>) {
        let spec = CommandSpec::new("yosys").arg("-V");
        match self.runner.run(&spec) {
            Ok(output) => {
                let version_text = first_non_empty_line(&output.stdout)
                    .or_else(|| first_non_empty_line(&output.stderr))
                    .unwrap_or("yosys version output was empty")
                    .to_string();
                (
                    ToolVersion::available("yosys", version_text),
                    vec![CommandRecord::from(output)],
                )
            }
            Err(SecurityError::CommandUnavailable { message, .. }) => {
                (ToolVersion::unavailable("yosys", message), Vec::new())
            }
            Err(err) => (
                ToolVersion::unavailable("yosys", err.to_string()),
                Vec::new(),
            ),
        }
    }
}

pub fn capabilities() -> Vec<BackendCapability> {
    YosysBackend::process().capabilities()
}

impl<R> AfBackend for YosysBackend<R>
where
    R: CommandRunner,
{
    fn name(&self) -> &'static str {
        "yosys"
    }

    fn capabilities(&self) -> Vec<BackendCapability> {
        vec![
            BackendCapability {
                name: "yosys-syntax-smoke".to_string(),
                supported: true,
                detail: Some(
                    "Runs yosys read_verilog/hierarchy/check over declared RTL sources."
                        .to_string(),
                ),
            },
            BackendCapability {
                name: "yosys-synthesis-smoke".to_string(),
                supported: true,
                detail: Some(
                    "MVP smoke elaboration only; no timing closure or vendor bitstream flow."
                        .to_string(),
                ),
            },
        ]
    }

    fn probe(&self, _plan: &af_backend::BuildPlan) -> Result<ToolInfo, BackendError> {
        let (version, _) = self.probe_version();
        Ok(ToolInfo {
            backend_id: BackendId(self.name().to_string()),
            tool_name: "yosys".to_string(),
            executable: "yosys".into(),
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
                "Yosys is not installed or not visible in PATH; syntax/synthesis smoke checks will report BackendUnavailable.".to_string(),
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
                .push("Yosys backend unavailable".to_string());
            return Ok(report);
        }

        let spec = yosys_smoke_command(manifest, core_dir);
        let mut report = BackendReport::new(self.name(), BackendStatus::Failed);
        report.tool_versions.push(version);
        report.commands.extend(version_commands);
        report.artifacts.push(build_root.to_path_buf());
        report.limitations.push(
            "Yosys smoke checks syntax/elaboration only; they are not timing, CDC/RDC, or vendor signoff."
                .to_string(),
        );

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
                .push("Yosys syntax/synthesis smoke returned a non-zero exit code".to_string());
        }
        Ok(report)
    }

    fn sim(
        &self,
        _manifest: &CoreManifest,
        _core_dir: &Path,
        _build_root: &Path,
    ) -> Result<BackendReport, BackendError> {
        Ok(BackendReport::unavailable(
            self.name(),
            "yosys",
            "Yosys does not provide the MVP simulation path; use Verilator for `af core sim`.",
        ))
    }
}

pub fn yosys_smoke_command(manifest: &CoreManifest, core_dir: &Path) -> CommandSpec {
    let mut read = String::from("read_verilog");
    if uses_systemverilog(manifest) {
        read.push_str(" -sv");
    }
    for include_dir in &manifest.sources.include_dirs {
        read.push_str(" -I");
        read.push_str(include_dir);
    }
    for source in &manifest.sources.files {
        read.push(' ');
        read.push_str(source);
    }

    let script = format!(
        "{read}; hierarchy -check -top {}; proc; opt; check",
        manifest.rtl.top
    );
    CommandSpec::new("yosys")
        .args(["-q".to_string(), "-p".to_string(), script])
        .cwd(core_dir)
}

fn uses_systemverilog(manifest: &CoreManifest) -> bool {
    let language = manifest.rtl.language.to_ascii_lowercase();
    language.contains("systemverilog")
        || language.contains("sv")
        || manifest
            .sources
            .files
            .iter()
            .any(|source| source.ends_with(".sv"))
}

fn first_non_empty_line(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|line| !line.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use af_security::CommandOutput;
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
language = "verilog-2001"

[sources]
files = ["rtl/demo.v"]
"#,
            "af-core.toml",
        )
        .unwrap()
    }

    #[test]
    fn builds_yosys_smoke_argv_without_shell() {
        let spec = yosys_smoke_command(&manifest(), Path::new("core"));
        assert_eq!(spec.program, "yosys");
        assert_eq!(spec.args[0], "-q");
        assert_eq!(spec.args[1], "-p");
        assert!(spec.args[2].contains("read_verilog rtl/demo.v"));
        assert!(spec.args[2].contains("hierarchy -check -top demo"));
    }

    #[test]
    fn returns_passed_report_from_fake_runner() {
        let version = CommandOutput {
            spec: CommandSpec::new("yosys"),
            exit_code: Some(0),
            stdout: "Yosys 0.40".to_string(),
            stderr: String::new(),
        };
        let smoke = CommandOutput {
            spec: CommandSpec::new("yosys"),
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        };
        let backend = YosysBackend::new(FakeRunner::new(vec![version, smoke]));
        let report = backend
            .lint(&manifest(), Path::new("."), Path::new(".af-build"))
            .unwrap();
        assert_eq!(report.status, BackendStatus::Passed);
        assert_eq!(report.commands.len(), 2);
    }
}
