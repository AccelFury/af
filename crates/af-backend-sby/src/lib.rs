// SPDX-License-Identifier: Apache-2.0
use af_backend::{
    AfBackend, BackendCapability, BackendError, BackendId, BackendReport, BackendStatus,
    CommandRecord, CommandRunner, CommandSpec, ProcessCommandRunner, ToolInfo, ToolVersion,
};
use af_manifest::CoreManifest;
use af_security::SecurityError;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct SbyBackend<R = ProcessCommandRunner> {
    runner: R,
}

impl SbyBackend<ProcessCommandRunner> {
    pub fn process() -> Self {
        Self {
            runner: ProcessCommandRunner,
        }
    }
}

impl<R> SbyBackend<R>
where
    R: CommandRunner,
{
    pub fn new(runner: R) -> Self {
        Self { runner }
    }

    fn probe_version(&self) -> (ToolVersion, Vec<CommandRecord>) {
        let spec = CommandSpec::new("sby").arg("--version");
        match self.runner.run(&spec) {
            Ok(output) => {
                let version_text = first_non_empty_line(&output.stdout)
                    .or_else(|| first_non_empty_line(&output.stderr))
                    .unwrap_or("sby version output was empty")
                    .to_string();
                (
                    ToolVersion::available("sby", version_text),
                    vec![CommandRecord::from(output)],
                )
            }
            Err(SecurityError::CommandUnavailable { message, .. }) => {
                (ToolVersion::unavailable("sby", message), Vec::new())
            }
            Err(err) => (ToolVersion::unavailable("sby", err.to_string()), Vec::new()),
        }
    }
}

pub fn capabilities() -> Vec<BackendCapability> {
    SbyBackend::process().capabilities()
}

impl<R> AfBackend for SbyBackend<R>
where
    R: CommandRunner,
{
    fn name(&self) -> &'static str {
        "sby"
    }

    fn capabilities(&self) -> Vec<BackendCapability> {
        vec![BackendCapability {
            name: "sby-formal".to_string(),
            supported: true,
            detail: Some(
                "Runs declared SymbiYosys .sby files when [formal].enabled is true.".to_string(),
            ),
        }]
    }

    fn probe(&self, _plan: &af_backend::BuildPlan) -> Result<ToolInfo, BackendError> {
        let (version, _) = self.probe_version();
        Ok(ToolInfo {
            backend_id: BackendId(self.name().to_string()),
            tool_name: "SymbiYosys".to_string(),
            executable: "sby".into(),
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
        report.limitations.push(
            "SymbiYosys evidence proves only the declared .sby targets; it is not exhaustive formal signoff unless the project declares and reviews that coverage."
                .to_string(),
        );
        if !matches!(report.status, BackendStatus::Passed) {
            report
                .warnings
                .push("sby is not installed or not visible in PATH.".to_string());
        }
        Ok(report)
    }

    fn lint(
        &self,
        manifest: &CoreManifest,
        core_dir: &Path,
        build_root: &Path,
    ) -> Result<BackendReport, BackendError> {
        self.run_formal(manifest, core_dir, build_root)
    }

    fn sim(
        &self,
        _manifest: &CoreManifest,
        _core_dir: &Path,
        _build_root: &Path,
    ) -> Result<BackendReport, BackendError> {
        Ok(BackendReport::unavailable(
            self.name(),
            "sby",
            "SymbiYosys is a formal backend, not a simulation backend.",
        ))
    }
}

impl<R> SbyBackend<R>
where
    R: CommandRunner,
{
    pub fn run_formal(
        &self,
        manifest: &CoreManifest,
        core_dir: &Path,
        build_root: &Path,
    ) -> Result<BackendReport, BackendError> {
        let mut report = self.doctor()?;
        if !matches!(report.status, BackendStatus::Passed) {
            return Ok(report);
        }

        let Some(formal) = &manifest.formal else {
            report
                .warnings
                .push("No [formal] block declared; formal run skipped.".to_string());
            return Ok(report);
        };
        if !formal.enabled {
            report
                .warnings
                .push("[formal].enabled is false; formal run skipped.".to_string());
            return Ok(report);
        }
        if formal.files.is_empty() {
            report.status = BackendStatus::Failed;
            report
                .warnings
                .push("[formal].enabled is true but formal.files is empty.".to_string());
            return Ok(report);
        }

        report.status = BackendStatus::Passed;
        report.artifacts.push(build_root.join("formal"));
        for file in &formal.files {
            let spec = sby_command(file, core_dir);
            let output = self.runner.run(&spec)?;
            let passed = output.exit_code == Some(0);
            report.commands.push(CommandRecord::from(output));
            if !passed {
                report.status = BackendStatus::Failed;
                report
                    .warnings
                    .push(format!("sby target `{file}` returned a non-zero exit code"));
            }
        }
        Ok(report)
    }
}

pub fn sby_command(file: &str, core_dir: &Path) -> CommandSpec {
    CommandSpec::new("sby")
        .args(["-f".to_string(), file.to_string()])
        .cwd(core_dir)
}

fn first_non_empty_line(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|line| !line.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_sby_command_without_shell() {
        let spec = sby_command("formal/demo.sby", Path::new("core"));
        assert_eq!(spec.program, "sby");
        assert_eq!(spec.args, vec!["-f", "formal/demo.sby"]);
        assert_eq!(spec.cwd, Some("core".into()));
    }
}
