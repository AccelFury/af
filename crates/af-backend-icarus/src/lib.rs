// SPDX-License-Identifier: Apache-2.0
use af_backend::{
    AfBackend, BackendCapability, BackendError, BackendId, BackendReport, BackendStatus,
    CommandRecord, CommandRunner, CommandSpec, ProcessCommandRunner, ToolInfo, ToolVersion,
};
use af_manifest::CoreManifest;
use af_security::SecurityError;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct IcarusBackend<R = ProcessCommandRunner> {
    runner: R,
}

impl IcarusBackend<ProcessCommandRunner> {
    pub fn process() -> Self {
        Self {
            runner: ProcessCommandRunner,
        }
    }
}

impl<R> IcarusBackend<R>
where
    R: CommandRunner,
{
    pub fn new(runner: R) -> Self {
        Self { runner }
    }

    fn probe_program(
        &self,
        program: &'static str,
        args: &[&str],
    ) -> (ToolVersion, Vec<CommandRecord>) {
        let spec = CommandSpec::new(program).args(args.iter().copied());
        match self.runner.run(&spec) {
            Ok(output) => {
                let version_text = first_non_empty_line(&output.stdout)
                    .or_else(|| first_non_empty_line(&output.stderr))
                    .unwrap_or("version output was empty")
                    .to_string();
                (
                    ToolVersion::available(program, version_text),
                    vec![CommandRecord::from(output)],
                )
            }
            Err(SecurityError::CommandUnavailable { message, .. }) => {
                (ToolVersion::unavailable(program, message), Vec::new())
            }
            Err(err) => (
                ToolVersion::unavailable(program, err.to_string()),
                Vec::new(),
            ),
        }
    }

    fn probe_tools(&self) -> (Vec<ToolVersion>, Vec<CommandRecord>) {
        let (iverilog, mut commands) = self.probe_program("iverilog", &["-V"]);
        let (vvp, vvp_commands) = self.probe_program("vvp", &["-V"]);
        commands.extend(vvp_commands);
        (vec![iverilog, vvp], commands)
    }
}

pub fn capabilities() -> Vec<BackendCapability> {
    IcarusBackend::process().capabilities()
}

impl<R> AfBackend for IcarusBackend<R>
where
    R: CommandRunner,
{
    fn name(&self) -> &'static str {
        "icarus"
    }

    fn capabilities(&self) -> Vec<BackendCapability> {
        vec![
            BackendCapability {
                name: "iverilog-elaboration".to_string(),
                supported: true,
                detail: Some(
                    "Compiles declared RTL with iverilog without invoking a shell.".to_string(),
                ),
            },
            BackendCapability {
                name: "vvp-simulation".to_string(),
                supported: true,
                detail: Some(
                    "Runs compiled Icarus VVP output when a Verilog testbench is declared."
                        .to_string(),
                ),
            },
        ]
    }

    fn probe(&self, _plan: &af_backend::BuildPlan) -> Result<ToolInfo, BackendError> {
        let (versions, _) = self.probe_tools();
        let available = versions.iter().all(|tool| tool.available);
        Ok(ToolInfo {
            backend_id: BackendId(self.name().to_string()),
            tool_name: "icarus-verilog".to_string(),
            executable: "iverilog".into(),
            version: versions
                .iter()
                .find(|tool| tool.tool == "iverilog")
                .and_then(|tool| tool.version.clone()),
            available,
            diagnostics: Vec::new(),
        })
    }

    fn doctor(&self) -> Result<BackendReport, BackendError> {
        let (versions, commands) = self.probe_tools();
        let available = versions.iter().all(|tool| tool.available);
        let mut report = BackendReport::new(
            self.name(),
            if available {
                BackendStatus::Passed
            } else {
                BackendStatus::Unavailable
            },
        );
        report.tool_versions.extend(versions);
        report.commands.extend(commands);
        if !available {
            report
                .warnings
                .push("Icarus Verilog requires both iverilog and vvp in PATH.".to_string());
        }
        Ok(report)
    }

    fn lint(
        &self,
        manifest: &CoreManifest,
        core_dir: &Path,
        build_root: &Path,
    ) -> Result<BackendReport, BackendError> {
        let mut report = self.doctor()?;
        if !matches!(report.status, BackendStatus::Passed) {
            return Ok(report);
        }
        let spec = icarus_lint_command(manifest, core_dir);
        report.status = BackendStatus::Failed;
        report.artifacts.push(build_root.to_path_buf());
        report.limitations.push(
            "Icarus lint is compile/elaboration evidence only; it is not timing, CDC/RDC, synthesis, or hardware signoff."
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
                .push("iverilog elaboration returned a non-zero exit code".to_string());
        }
        Ok(report)
    }

    fn sim(
        &self,
        manifest: &CoreManifest,
        core_dir: &Path,
        build_root: &Path,
    ) -> Result<BackendReport, BackendError> {
        let mut report = self.doctor()?;
        if !matches!(report.status, BackendStatus::Passed) {
            return Ok(report);
        }
        report.limitations.push(
            "Icarus simulation runs declared Verilog testbench sources only; it is not a full regression unless the manifest declares that regression."
                .to_string(),
        );
        if manifest.testbenches.is_empty() {
            report
                .warnings
                .push("No testbenches declared; Icarus simulation skipped.".to_string());
            return Ok(report);
        }

        let out_dir = build_root.join("icarus");
        std::fs::create_dir_all(&out_dir).map_err(|err| BackendError::Failed {
            backend: self.name().to_string(),
            message: format!("failed to create `{}`: {err}", out_dir.display()),
        })?;
        let vvp_path = out_dir.join(format!("{}.vvp", manifest.core));
        let compile = icarus_sim_compile_command(manifest, core_dir, &vvp_path);
        let compile_output = self.runner.run(&compile)?;
        let compile_passed = compile_output.exit_code == Some(0);
        report.commands.push(CommandRecord::from(compile_output));
        report.artifacts.push(vvp_path.clone());
        if !compile_passed {
            report.status = BackendStatus::Failed;
            report
                .warnings
                .push("iverilog testbench compilation failed".to_string());
            return Ok(report);
        }

        let run = CommandSpec::new("vvp")
            .arg(vvp_path.display().to_string())
            .cwd(core_dir);
        let run_output = self.runner.run(&run)?;
        let run_passed = run_output.exit_code == Some(0);
        report.commands.push(CommandRecord::from(run_output));
        report.status = if run_passed {
            BackendStatus::Passed
        } else {
            BackendStatus::Failed
        };
        if !run_passed {
            report.warnings.push("vvp simulation failed".to_string());
        }
        Ok(report)
    }
}

pub fn icarus_lint_command(manifest: &CoreManifest, core_dir: &Path) -> CommandSpec {
    let mut args = vec![
        iverilog_standard_flag(manifest),
        "-Wall".to_string(),
        "-tnull".to_string(),
        "-s".to_string(),
        manifest.rtl.top.clone(),
    ];
    for include_dir in &manifest.sources.include_dirs {
        args.push(format!("-I{include_dir}"));
    }
    args.extend(manifest.sources.files.iter().cloned());
    CommandSpec::new("iverilog").args(args).cwd(core_dir)
}

pub fn icarus_sim_compile_command(
    manifest: &CoreManifest,
    core_dir: &Path,
    output: &Path,
) -> CommandSpec {
    let testbench = manifest.testbenches.first();
    let top = testbench
        .map(|tb| tb.top.clone())
        .unwrap_or_else(|| manifest.rtl.top.clone());
    let mut args = vec![
        iverilog_standard_flag(manifest),
        "-Wall".to_string(),
        "-s".to_string(),
        top,
        "-o".to_string(),
        output.display().to_string(),
    ];
    for include_dir in &manifest.sources.include_dirs {
        args.push(format!("-I{include_dir}"));
    }
    args.extend(manifest.sources.files.iter().cloned());
    if let Some(testbench) = testbench {
        args.extend(testbench.rtl_sources.iter().cloned());
        args.extend(
            testbench
                .sources
                .iter()
                .filter(|source| is_verilog_source(source))
                .cloned(),
        );
    }
    dedup_preserve_order(&mut args);
    CommandSpec::new("iverilog").args(args).cwd(core_dir)
}

fn is_verilog_source(source: &str) -> bool {
    source.ends_with(".v") || source.ends_with(".vh") || source.ends_with(".sv")
}

fn iverilog_standard_flag(manifest: &CoreManifest) -> String {
    if manifest
        .rtl
        .language
        .to_ascii_lowercase()
        .contains("systemverilog")
        || manifest
            .sources
            .files
            .iter()
            .any(|source| source.ends_with(".sv"))
        || manifest
            .testbenches
            .iter()
            .flat_map(|testbench| testbench.sources.iter().chain(testbench.rtl_sources.iter()))
            .any(|source| source.ends_with(".sv"))
    {
        "-g2012".to_string()
    } else {
        "-g2001".to_string()
    }
}

fn dedup_preserve_order(items: &mut Vec<String>) {
    let mut seen = std::collections::BTreeSet::new();
    items.retain(|item| seen.insert(item.clone()));
}

fn first_non_empty_line(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|line| !line.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
include_dirs = ["rtl"]

[[testbenches]]
name = "tb"
top = "tb_demo"
sources = ["tb/tb_demo.v"]
rtl_sources = ["rtl/demo.v"]
"#,
            "af-core.toml",
        )
        .unwrap()
    }

    #[test]
    fn builds_icarus_lint_argv_without_shell() {
        let spec = icarus_lint_command(&manifest(), Path::new("core"));
        assert_eq!(spec.program, "iverilog");
        assert!(spec.args.contains(&"-tnull".to_string()));
        assert!(spec.args.contains(&"demo".to_string()));
        assert_eq!(spec.cwd, Some(PathBuf::from("core")));
    }

    #[test]
    fn builds_icarus_sim_compile_argv() {
        let spec = icarus_sim_compile_command(
            &manifest(),
            Path::new("core"),
            Path::new(".af-build/icarus/demo.vvp"),
        );
        assert_eq!(spec.program, "iverilog");
        assert!(spec.args.contains(&"tb_demo".to_string()));
        assert!(spec.args.contains(&"tb/tb_demo.v".to_string()));
    }
}
