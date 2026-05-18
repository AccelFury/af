// SPDX-License-Identifier: Apache-2.0

use crate::ci::{
    allowlist, complete_board_jobs, detect_profile, from_toml_file, is_board_complete, parse_yaml,
    policy::validate_policy_from_config, report::build_detection_map, scan_repo,
    validate_workflow_contract,
};
use crate::{CliError, CliOutput};
use clap::Args;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct CiValidateArgs {
    #[arg(long, default_value = ".")]
    pub repo: PathBuf,
    #[arg(long, default_value = "af-ci.toml")]
    pub config: PathBuf,
}

pub fn run(args: &CiValidateArgs) -> Result<CliOutput, CliError> {
    let config_path = if args.config.is_absolute() {
        args.config.clone()
    } else {
        args.repo.join(&args.config)
    };
    let config = from_toml_file(&config_path).map_err(|err| {
        CliError::new(
            "AF_CI_VALIDATE_CONFIG",
            format!("invalid config `{}`: {err}", config_path.display()),
            "Run `af ci init` to generate af-ci.toml.",
            3,
        )
    })?;

    let workflow_path = args.repo.join(".github/workflows/hdl-ci.yml");
    let workflow_text = fs::read_to_string(&workflow_path).map_err(|err| {
        CliError::new(
            "AF_CI_VALIDATE_WORKFLOW",
            format!("cannot read workflow `{}`: {err}", workflow_path.display()),
            "Create workflow with `af ci render`.",
            3,
        )
    })?;

    let mut blocking = Vec::new();
    let mut warnings = Vec::new();
    let scan = scan_repo(&args.repo, &config.paths);
    let profile = detect_profile(&scan, &config.project.hdl);
    let detection = build_detection_map(&config, &scan);

    let policy = validate_policy_from_config(&config, &workflow_text);
    blocking.extend(policy.blocking);
    warnings.extend(policy.warnings);

    if let Err(err) = parse_yaml(&workflow_text) {
        blocking.push(err);
    } else {
        if let Ok(contract) = validate_workflow_contract(&workflow_text, &config, &scan, profile) {
            blocking.extend(contract.blocking);
            warnings.extend(contract.warnings);
            if contract.upload_paths.is_empty() {
                warnings.push("workflow contains no upload-artifact steps".to_string());
            }
        }

        if !config.core.top.trim().is_empty() && !scan.top_candidates.contains(&config.core.top) {
            blocking.push(format!(
                "configured core top '{}' was not detected in scanned RTL sources",
                config.core.top
            ));
        }

        let complete_boards = complete_board_jobs(&config, &args.repo);
        if complete_boards == 0 && !config.boards.iter().any(|board| board.enabled) {
            warnings.push(
                "no complete board profiles enabled; P&R jobs will not be generated".to_string(),
            );
        }

        for board in &config.boards {
            if board.enabled {
                if !scan.board_top_candidates.contains(&board.top)
                    && !scan.top_candidates.contains(&board.top)
                {
                    blocking.push(format!(
                        "enabled board '{}' top '{}' was not detected in scanned board/RTL sources",
                        board.name, board.top
                    ));
                }
                if !is_board_complete(board, &args.repo) {
                    warnings.push(format!(
                        "enabled board '{}' is incomplete and cannot run P&R",
                        board.name
                    ));
                }

                if board.constraints.trim().is_empty() {
                    blocking.push(format!(
                        "enabled board '{}' requires constraints path",
                        board.name
                    ));
                } else if !args.repo.join(&board.constraints).exists() {
                    blocking.push(format!(
                        "enabled board '{}' constraints missing: {}",
                        board.name, board.constraints
                    ));
                }
            } else if !board.constraints.trim().is_empty()
                && !args.repo.join(&board.constraints).exists()
            {
                warnings.push(format!(
                    "board '{}' references missing constraints path: {}",
                    board.name, board.constraints
                ));
            }
        }
    }

    for relative in config
        .paths
        .rtl
        .iter()
        .chain(config.paths.tb.iter())
        .chain(config.paths.sim.iter())
        .chain(config.paths.formal.iter())
        .chain(config.paths.boards.iter())
    {
        if relative.trim().is_empty() {
            continue;
        }
        if !args.repo.join(relative).exists() {
            warnings.push(format!("path from config does not exist: {relative}"));
        }
    }

    let status = if !blocking.is_empty() {
        "fail"
    } else if !warnings.is_empty() {
        "warning"
    } else {
        "pass"
    };
    let problem_classes = crate::ci::diagnostics::problem_classes(&blocking, &warnings);

    if status == "fail" {
        return Err(CliError::new(
            "AF_CI_VALIDATE_FAIL",
            "validation found blocking errors",
            "Inspect command output and fix listed failures.",
            1,
        )
        .with_details(&json!({
            "schema_version": "1.0",
            "project": config.project.name,
            "status": status,
            "detected": detection,
            "blocking_errors": blocking,
            "warnings": warnings,
            "problem_classes": problem_classes,
            "artifact_contract": allowlist().into_iter().collect::<Vec<_>>(),
            "required_artifacts": crate::ci::artifacts::artifact_paths(&config),
            "allowed_artifacts": crate::ci::artifacts::allowlist(),
        })));
    }

    Ok(CliOutput {
        human: format!("af ci validate: {status}"),
        json: json!({
            "schema_version": "1.0",
            "project": config.project.name,
            "status": status,
            "detected": detection,
            "blocking_errors": blocking,
            "warnings": warnings,
            "problem_classes": problem_classes,
            "artifact_contract": allowlist().into_iter().collect::<Vec<_>>(),
            "required_artifacts": crate::ci::artifacts::artifact_paths(&config),
            "allowed_artifacts": crate::ci::artifacts::allowlist(),
        }),
    })
}
