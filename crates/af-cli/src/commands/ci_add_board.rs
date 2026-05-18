// SPDX-License-Identifier: Apache-2.0

use crate::ci::{from_toml_file, to_toml_string, CiBoardConfig};
use crate::{CliError, CliOutput};
use clap::Args;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct CiAddBoardArgs {
    #[arg(long, default_value = ".")]
    pub repo: PathBuf,
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub family: String,
    #[arg(long)]
    pub top: String,
    #[arg(long)]
    pub device: String,
    #[arg(long)]
    pub nextpnr_family: Option<String>,
    #[arg(long, alias = "package")]
    pub pack_device: Option<String>,
    #[arg(long)]
    pub constraints: PathBuf,
    #[arg(long, value_delimiter = ',')]
    pub source_globs: Vec<String>,
    #[arg(long)]
    pub dry_run: bool,
}

pub fn run(args: &CiAddBoardArgs) -> Result<CliOutput, CliError> {
    let config_path = args.repo.join("af-ci.toml");
    let mut config = from_toml_file(&config_path).map_err(|err| {
        CliError::new(
            "AF_CI_ADD_BOARD_CONFIG",
            format!("read af-ci.toml: {err}"),
            "Run af ci init --project ... first.",
            3,
        )
    })?;

    validate_family_requirements(args)?;
    let constraints = args.constraints.to_string_lossy().to_string();
    if !args.repo.join(&args.constraints).exists() {
        return Err(CliError::new(
            "AF_CI_ADD_BOARD_CONSTRAINTS",
            format!("constraint file not found: {}", args.constraints.display()),
            "Pass an existing constraint path relative to repo root.",
            2,
        ));
    }

    let board = CiBoardConfig {
        name: args.name.clone(),
        enabled: true,
        family: args.family.clone(),
        top: args.top.clone(),
        device: args.device.clone(),
        nextpnr_family: args.nextpnr_family.clone().unwrap_or_default(),
        pack_device: args.pack_device.clone().unwrap_or_default(),
        constraints,
        source_globs: args.source_globs.clone(),
    };

    config.add_or_replace_board(board);
    let text = to_toml_string(&config).map_err(|err| {
        CliError::new(
            "AF_CI_ADD_BOARD_SERIALIZE",
            format!("serialize af-ci.toml: {err}"),
            "Check config structure and writable file permissions.",
            5,
        )
    })?;

    if !args.dry_run {
        fs::write(&config_path, text).map_err(|err| {
            CliError::new(
                "AF_CI_ADD_BOARD_WRITE",
                format!("write af-ci.toml: {err}"),
                "Check write permissions in repository root.",
                5,
            )
        })?;
    }

    Ok(CliOutput {
        human: if args.dry_run {
            format!("af ci add-board would update {}", config_path.display())
        } else {
            format!("af ci add-board updated {}", config_path.display())
        },
        json: json!({
            "status": "passed",
            "board": args.name,
            "repo": args.repo,
            "config": config.project.name,
        }),
    })
}

fn validate_family_requirements(args: &CiAddBoardArgs) -> Result<(), CliError> {
    let family = args.family.to_lowercase();
    if args.top.trim().is_empty() {
        return Err(CliError::new(
            "AF_CI_ADD_BOARD_TOP_REQUIRED",
            "board profile needs --top",
            "Provide a top module name for this board profile.",
            2,
        ));
    }
    if args.constraints.as_os_str().is_empty() {
        return Err(CliError::new(
            "AF_CI_ADD_BOARD_CONSTRAINTS_REQUIRED",
            "board profile needs --constraints",
            "Provide a constraint file path.",
            2,
        ));
    }
    if args.device.trim().is_empty() {
        return Err(CliError::new(
            "AF_CI_ADD_BOARD_DEVICE_REQUIRED",
            "board profile needs --device",
            "Provide --device for board constraints.",
            2,
        ));
    }

    match family.as_str() {
        "gowin" => {
            if args.nextpnr_family.as_ref().is_none() {
                return Err(CliError::new(
                    "AF_CI_ADD_BOARD_GOWIN_REQUIREMENTS",
                    "gowin profile needs --nextpnr-family",
                    "Provide --nextpnr-family.",
                    2,
                ));
            }
            if args.pack_device.as_ref().is_none() {
                return Err(CliError::new(
                    "AF_CI_ADD_BOARD_GOWIN_REQUIREMENTS",
                    "gowin profile needs --pack-device (alias --package)",
                    "Provide --pack-device or --package.",
                    2,
                ));
            }
        }
        "ice40" | "ecp5" => {
            if args.pack_device.as_ref().is_none() {
                return Err(CliError::new(
                    "AF_CI_ADD_BOARD_PACKAGE_REQUIRED",
                    "ice40/ecp5 profile needs --pack-device (alias --package)",
                    "Provide package flag (for example --package sg48).",
                    2,
                ));
            }
        }
        "xilinx" | "intel" | "generic" => {}
        _ => {
            return Err(CliError::new(
                "AF_CI_ADD_BOARD_UNSUPPORTED_FAMILY",
                "unsupported board family",
                "Supported families for add-board: gowin, ice40, ecp5.",
                2,
            ));
        }
    }

    Ok(())
}
