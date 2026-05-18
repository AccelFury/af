// SPDX-License-Identifier: Apache-2.0
use af_backend::{
    AfBackend, BackendCapability, BackendError, BackendId, BackendReport, BackendStatus,
    CommandRecord, CommandRunner, CommandSpec, ProcessCommandRunner, ToolInfo, ToolVersion,
};
use af_manifest::CoreManifest;
use af_security::SecurityError;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct NextpnrBackend<R = ProcessCommandRunner> {
    runner: R,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct NextpnrPlan {
    pub board: String,
    pub family: String,
    pub commands: Vec<CommandSpec>,
    pub expected_artifacts: Vec<PathBuf>,
    pub limitations: Vec<String>,
}

impl NextpnrBackend<ProcessCommandRunner> {
    pub fn process() -> Self {
        Self {
            runner: ProcessCommandRunner,
        }
    }
}

impl<R> NextpnrBackend<R>
where
    R: CommandRunner,
{
    pub fn new(runner: R) -> Self {
        Self { runner }
    }

    fn probe_program(&self, program: &'static str) -> (ToolVersion, Vec<CommandRecord>) {
        let spec = CommandSpec::new(program).arg("--version");
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
}

pub fn capabilities() -> Vec<BackendCapability> {
    NextpnrBackend::process().capabilities()
}

impl<R> AfBackend for NextpnrBackend<R>
where
    R: CommandRunner,
{
    fn name(&self) -> &'static str {
        "nextpnr"
    }

    fn capabilities(&self) -> Vec<BackendCapability> {
        vec![
            BackendCapability {
                name: "nextpnr-ice40-place-route".to_string(),
                supported: true,
                detail: Some("Plans iCE40 place-and-route from a Yosys JSON netlist plus PCF constraints.".to_string()),
            },
            BackendCapability {
                name: "nextpnr-ecp5-place-route".to_string(),
                supported: true,
                detail: Some("Plans ECP5 place-and-route from a Yosys JSON netlist plus LPF constraints.".to_string()),
            },
            BackendCapability {
                name: "nextpnr-gowin-place-route".to_string(),
                supported: true,
                detail: Some("Plans Gowin place-and-route from a Yosys JSON netlist plus CST constraints.".to_string()),
            },
            BackendCapability {
                name: "nextpnr-report-capture".to_string(),
                supported: true,
                detail: Some("Declares expected JSON/timing/resource report artifacts without claiming timing signoff.".to_string()),
            },
        ]
    }

    fn probe(&self, _plan: &af_backend::BuildPlan) -> Result<ToolInfo, BackendError> {
        let (ice40, _) = self.probe_program("nextpnr-ice40");
        let (ecp5, _) = self.probe_program("nextpnr-ecp5");
        let (gowin, _) = self.probe_program("nextpnr-gowin");
        let available = ice40.available || ecp5.available || gowin.available;
        Ok(ToolInfo {
            backend_id: BackendId(self.name().to_string()),
            tool_name: "nextpnr".to_string(),
            executable: "nextpnr-*".into(),
            version: ice40.version.or(ecp5.version).or(gowin.version),
            available,
            diagnostics: Vec::new(),
        })
    }

    fn doctor(&self) -> Result<BackendReport, BackendError> {
        let (ice40, mut commands) = self.probe_program("nextpnr-ice40");
        let (ecp5, ecp5_commands) = self.probe_program("nextpnr-ecp5");
        let (gowin, gowin_commands) = self.probe_program("nextpnr-gowin");
        commands.extend(ecp5_commands);
        commands.extend(gowin_commands);
        let available = ice40.available || ecp5.available || gowin.available;
        let mut report = BackendReport::new(
            self.name(),
            if available {
                BackendStatus::Passed
            } else {
                BackendStatus::Unavailable
            },
        );
        report.tool_versions.extend([ice40, ecp5, gowin]);
        report.commands.extend(commands);
        report.limitations.push(
            "nextpnr availability is not board timing signoff; P&R requires complete device/package/constraint metadata and review of generated timing/resource reports."
                .to_string(),
        );
        if !available {
            report
                .warnings
                .push("No supported nextpnr executable was found in PATH.".to_string());
        }
        Ok(report)
    }

    fn lint(
        &self,
        _manifest: &CoreManifest,
        _core_dir: &Path,
        _build_root: &Path,
    ) -> Result<BackendReport, BackendError> {
        self.doctor()
    }

    fn sim(
        &self,
        _manifest: &CoreManifest,
        _core_dir: &Path,
        _build_root: &Path,
    ) -> Result<BackendReport, BackendError> {
        Ok(BackendReport::unavailable(
            self.name(),
            "nextpnr",
            "nextpnr is a place-and-route backend, not a simulation backend.",
        ))
    }
}

pub fn plan_nextpnr(board: &str, build_root: &Path) -> NextpnrPlan {
    let family = infer_family(board);
    let reports = build_root.join("pnr");
    let netlist = build_root.join("synth/synth_core.json");
    let routed = reports.join(format!("{board}.json"));
    let log = reports.join(format!("{board}.log"));
    let timing = reports.join(format!("{board}.timing.rpt"));
    let commands = match family {
        "ice40" => vec![nextpnr_command("nextpnr-ice40", &netlist, &routed)],
        "ecp5" => vec![nextpnr_command("nextpnr-ecp5", &netlist, &routed)],
        "gowin" => vec![nextpnr_command("nextpnr-gowin", &netlist, &routed)],
        _ => Vec::new(),
    };
    let mut limitations = vec![
        "This is an offline P&R command plan; af does not execute nextpnr unless a complete board/device/constraint flow is added."
            .to_string(),
        "Generated nextpnr reports are evidence inputs, not timing signoff by themselves."
            .to_string(),
    ];
    if commands.is_empty() {
        limitations.push(format!(
            "Board `{board}` does not map to a supported nextpnr family; add an iCE40, ECP5, or Gowin board profile before executing P&R."
        ));
    }
    NextpnrPlan {
        board: board.to_string(),
        family: family.to_string(),
        commands,
        expected_artifacts: vec![routed, log, timing],
        limitations,
    }
}

fn nextpnr_command(program: &str, netlist: &Path, routed: &Path) -> CommandSpec {
    CommandSpec::new(program).args([
        "--json".to_string(),
        netlist.display().to_string(),
        "--write".to_string(),
        routed.display().to_string(),
    ])
}

fn infer_family(board: &str) -> &'static str {
    let lowered = board.to_ascii_lowercase();
    if lowered.contains("ice40") || lowered.contains("icebreaker") {
        "ice40"
    } else if lowered.contains("ecp5") || lowered.contains("ulx3s") {
        "ecp5"
    } else if lowered.contains("gowin") || lowered.contains("tang") {
        "gowin"
    } else {
        "generic"
    }
}

fn first_non_empty_line(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|line| !line.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_family_specific_nextpnr_command() {
        let plan = plan_nextpnr("ulx3s_ecp5", Path::new(".af-build"));
        assert_eq!(plan.family, "ecp5");
        assert_eq!(plan.commands[0].program, "nextpnr-ecp5");
        assert!(plan.commands[0].args.contains(&"--json".to_string()));
        assert!(plan
            .expected_artifacts
            .iter()
            .any(|path| path.display().to_string().contains("timing")));
    }
}
