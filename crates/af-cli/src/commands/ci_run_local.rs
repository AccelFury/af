// SPDX-License-Identifier: Apache-2.0

use crate::ci::{detect_profile, from_toml_file, render_workflow, scan_repo, ProjectProfile};
use crate::{CliError, CliOutput};
use clap::Args;
use serde_json::json;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Args, Debug)]
pub struct CiRunLocalArgs {
    #[arg(long, default_value = ".")]
    pub repo: PathBuf,
    #[arg(long)]
    pub profile: String,
    #[arg(long)]
    pub dry_run: bool,
}

pub fn run(args: &CiRunLocalArgs) -> Result<CliOutput, CliError> {
    let profile = args.profile.as_str();
    let config = from_toml_file(&args.repo.join("af-ci.toml")).map_err(|err| {
        CliError::new(
            "AF_CI_RUN_LOCAL_CONFIG",
            format!("cannot read af-ci.toml: {err}"),
            "Run af ci init for baseline config.",
            3,
        )
    })?;
    let scan = scan_repo(&args.repo, &config.paths);
    let detection_profile = detect_profile(&scan, &config.project.hdl);

    let status = match profile {
        "sim" => run_simulation_probe(&scan, detection_profile),
        "synth" => run_synth_probe(),
        "doctor" => 0,
        _ => {
            return Err(CliError::new(
                "AF_CI_RUN_LOCAL_PROFILE",
                format!("unsupported local profile `{}`", profile),
                "Supported profiles: sim, synth, doctor.",
                2,
            ))
        }
    };

    if args.dry_run {
        let rendered = render_workflow(&config, &scan, detection_profile);
        return Ok(CliOutput {
            human: format!("af ci run-local {profile}: dry-run complete"),
            json: json!({
                "status": "passed",
                "dry_run": true,
                "profile": profile,
                "workflow_preview_len": rendered.len(),
            }),
        });
    }

    if status == 0 {
        Ok(CliOutput {
            human: format!("af ci run-local {profile}: completed"),
            json: json!({
                "status": "passed",
                "profile": profile,
                "code": status,
            }),
        })
    } else {
        Err(CliError::new(
            "AF_CI_RUN_LOCAL_FAIL",
            format!("local profile `{profile}` reported exit code {status}"),
            "Install required tools in PATH or run with --dry-run.",
            2,
        ))
    }
}

fn run_simulation_probe(scan: &crate::ci::scanner::RepoScan, profile: ProjectProfile) -> i32 {
    if !scan.has_make_test_target {
        return 2;
    }
    if !command_available("make") {
        return 2;
    }
    if profile == ProjectProfile::VerilogWithIverilogMake
        && (!command_available("iverilog") || !command_available("vvp"))
    {
        2
    } else {
        0
    }
}

fn run_synth_probe() -> i32 {
    if command_available("yosys") {
        0
    } else {
        2
    }
}

fn command_available(program: &str) -> bool {
    if program.contains(std::path::MAIN_SEPARATOR) {
        return is_executable(Path::new(program));
    }
    env::var_os("PATH")
        .is_some_and(|paths| env::split_paths(&paths).any(|dir| is_executable(&dir.join(program))))
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}
