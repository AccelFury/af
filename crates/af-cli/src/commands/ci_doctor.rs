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
pub struct CiDoctorArgs {
    #[arg(long, default_value = ".")]
    pub repo: PathBuf,
}

pub fn run(args: &CiDoctorArgs) -> Result<CliOutput, CliError> {
    let config_path = args.repo.join("af-ci.toml");
    let config = match from_toml_file(&config_path) {
        Ok(config) => Some(config),
        Err(err) => {
            let warning = format!("af-ci.toml missing or unreadable: {err}");
            return Err(cli_error("AF_CI_DOCTOR_CONFIG", &warning, 3));
        }
    };

    let config = config.unwrap();
    let scan = scan_repo(&args.repo, &config.paths);
    let profile = detect_profile(&scan, &config.project.hdl);
    let workflow_path = args.repo.join(".github/workflows/hdl-ci.yml");
    let workflow_text = fs::read_to_string(&workflow_path).unwrap_or_default();

    let mut blocking_errors = Vec::new();
    let mut warnings = Vec::new();

    if !config.core.top.trim().is_empty() && !scan.top_candidates.contains(&config.core.top) {
        blocking_errors.push(format!(
            "configured core top '{}' was not detected in scanned RTL sources",
            config.core.top
        ));
    }

    if !workflow_path.is_file() {
        blocking_errors
            .push("missing required workflow file: .github/workflows/hdl-ci.yml".to_string());
    } else {
        match parse_yaml(&workflow_text) {
            Ok(_) => {
                let policy = validate_policy_from_config(&config, &workflow_text);
                blocking_errors.extend(policy.blocking);
                warnings.extend(policy.warnings);

                if let Ok(contract) =
                    validate_workflow_contract(&workflow_text, &config, &scan, profile)
                {
                    blocking_errors.extend(contract.blocking);
                    warnings.extend(contract.warnings);
                    if contract.upload_paths.is_empty() {
                        warnings.push("workflow contains no upload-artifact steps".to_string());
                    }
                }
            }
            Err(err) => blocking_errors.push(err),
        }
    }

    let complete_boards = complete_board_jobs(&config, &args.repo);
    if complete_boards == 0 && config.boards.iter().any(|board| board.enabled) {
        warnings.push("enabled board profiles are incomplete; P&R jobs will not run".to_string());
    }

    for board in &config.boards {
        if board.enabled {
            if !scan.board_top_candidates.contains(&board.top)
                && !scan.top_candidates.contains(&board.top)
            {
                warnings.push(format!(
                    "enabled board '{}' top '{}' was not detected in board/RTL sources",
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
                blocking_errors.push(format!(
                    "enabled board '{}' requires constraints path",
                    board.name
                ));
            } else if !args.repo.join(&board.constraints).exists() {
                blocking_errors.push(format!(
                    "enabled board '{}' constraints missing: {}",
                    board.name, board.constraints
                ));
            }
        }
    }

    if !args.repo.join("docs/ci.md").is_file() {
        warnings.push("docs/ci.md missing".to_string());
    }

    if !args.repo.join(".github/PULL_REQUEST_TEMPLATE.md").is_file() {
        warnings.push("PR checklist missing".to_string());
    }

    if !config.artifacts.generate_sha256sums {
        warnings.push("artifact SHA256SUMS generation disabled".to_string());
    }
    if !config.artifacts.store_tool_versions {
        warnings.push("tool-versions.txt capture disabled".to_string());
    }

    if config.boards.iter().any(|board| board.enabled)
        && !scan.constraints.iter().any(|path| path.exists())
    {
        warnings
            .push("enabled board(s) configured without discovered constraint files".to_string());
    }

    if !workflow_text.contains("artifacts/openfpga-ci") {
        warnings.push("artifact root does not use openfpga-ci path".to_string());
    }

    let detection = build_detection_map(&config, &scan);
    let status = if !blocking_errors.is_empty() {
        "fail"
    } else if !warnings.is_empty() {
        "warning"
    } else {
        "pass"
    };

    let exit_code = match status {
        "pass" => 0,
        "warning" => 2,
        _ => 1,
    };
    let problem_classes = crate::ci::diagnostics::problem_classes(&blocking_errors, &warnings);

    let output = CliOutput {
        human: format!("af ci doctor: {status}"),
        json: json!({
            "schema_version": "1.0",
            "project": config.project.name,
            "status": status,
            "detected": detection,
            "generated_files": Vec::<String>::new(),
            "blocking_errors": blocking_errors,
            "warnings": warnings,
            "problem_classes": problem_classes,
            "artifact_contract": allowlist().into_iter().collect::<Vec<_>>(),
            "next_actions": next_actions(status),
            "artifact": {
                "paths": {
                    "allowlist": allowlist().to_vec(),
                }
            },
            "profile": profile.as_str(),
        }),
    };

    if exit_code == 0 {
        Ok(output)
    } else {
        Err(cli_error_exit(
            output_status_error(status),
            "ci contract not satisfied",
            exit_code,
        )
        .with_details(&output.json))
    }
}

fn next_actions(status: &str) -> Vec<String> {
    if status == "pass" {
        return Vec::new();
    }
    vec![
        "af ci render --config af-ci.toml --output .github/workflows/hdl-ci.yml".to_string(),
        "af ci validate --repo . --dry-run".to_string(),
        "af ci add-board ... with complete constraints".to_string(),
    ]
}

fn output_status_error(status: &str) -> &'static str {
    match status {
        "warning" => "AF_CI_DOCTOR_WARNING",
        _ => "AF_CI_DOCTOR_FAIL",
    }
}

fn cli_error_exit(code: &str, message: &str, exit_code: i32) -> CliError {
    CliError::new(
        code,
        message,
        "Run `af ci doctor` with updated af-ci.toml and workflow contract artifacts.",
        exit_code,
    )
}

fn cli_error(code: &str, message: &str, exit_code: i32) -> CliError {
    CliError::new(
        code,
        message,
        "Run `af ci init` to generate baseline configuration and CI assets.",
        exit_code,
    )
}
