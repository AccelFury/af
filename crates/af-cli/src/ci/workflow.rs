// SPDX-License-Identifier: Apache-2.0

use crate::ci::{
    artifacts,
    config::{CiBoardConfig, CiConfig},
    detector::ProjectProfile,
    scanner::RepoScan,
};
use serde_yaml::Value;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Default)]
pub struct WorkflowContractCheck {
    pub blocking: Vec<String>,
    pub warnings: Vec<String>,
    pub upload_paths: Vec<String>,
}

pub fn required_job_names(
    profile: ProjectProfile,
    config: &CiConfig,
    scan: &RepoScan,
    repo_root: &Path,
) -> Vec<String> {
    let mut jobs = vec!["synth_core".to_string()];
    if should_add_simulation_job(profile, config, scan) {
        jobs.push("sim".to_string());
    }
    if scan.has_sby {
        jobs.push("formal".to_string());
    }

    for idx in 0..complete_board_jobs(config, repo_root) {
        jobs.push(format!("pnr_{idx}"));
    }
    jobs.push("package_artifacts".to_string());

    jobs.sort();
    jobs.dedup();
    jobs
}

pub fn complete_board_jobs(config: &CiConfig, repo_root: &Path) -> usize {
    config
        .boards
        .iter()
        .filter(|board| board.enabled && is_board_complete(board, repo_root))
        .count()
}

pub fn is_board_complete(board: &CiBoardConfig, repo_root: &Path) -> bool {
    if board.top.trim().is_empty() {
        return false;
    }
    if board.constraints.trim().is_empty() {
        return false;
    }
    if !repo_root.join(&board.constraints).is_file() {
        return false;
    }

    let family = board.family.to_lowercase();
    match family.as_str() {
        "gowin" => {
            !board.device.is_empty()
                && !board.nextpnr_family.is_empty()
                && !board.pack_device.is_empty()
        }
        "ice40" | "ecp5" => !board.device.is_empty() && !board.pack_device.is_empty(),
        _ => true,
    }
}

pub fn should_add_simulation_job(
    profile: ProjectProfile,
    config: &CiConfig,
    scan: &RepoScan,
) -> bool {
    if !config.simulation.enabled {
        return false;
    }
    if !config.simulation.command.trim().is_empty() {
        return true;
    }
    if scan.has_make_test_target {
        return matches!(
            profile,
            ProjectProfile::VerilogWithIverilogMake | ProjectProfile::VerilatorCpp,
        );
    }
    false
}

pub fn parse_yaml(text: &str) -> Result<Value, String> {
    serde_yaml::from_str(text).map_err(|err| format!("workflow is not valid YAML: {err}"))
}

pub fn parse_yaml_for_jobs(text: &str) -> Result<Value, String> {
    parse_yaml(text)
}

pub fn has_job(yaml: &Value, job_name: &str) -> bool {
    yaml.get("jobs")
        .and_then(Value::as_mapping)
        .is_some_and(|jobs| jobs.contains_key(Value::String(job_name.to_string())))
}

pub fn list_jobs(yaml: &Value) -> Vec<String> {
    let mut names = BTreeSet::new();
    if let Some(jobs) = yaml.get("jobs").and_then(Value::as_mapping) {
        for key in jobs.keys() {
            if let Some(name) = key.as_str() {
                names.insert(name.to_string());
            }
        }
    }
    names.into_iter().collect()
}

pub fn extract_upload_artifact_paths(workflow: &Value) -> Vec<String> {
    let mut paths = Vec::new();
    let Some(jobs) = workflow.get("jobs").and_then(Value::as_mapping) else {
        return paths;
    };

    for job in jobs.values() {
        let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
            continue;
        };

        for step in steps {
            let Some(uses) = step.get("uses").and_then(Value::as_str) else {
                continue;
            };
            if !uses.contains("actions/upload-artifact@") {
                continue;
            }
            let Some(with_map) = step.get("with").and_then(Value::as_mapping) else {
                continue;
            };

            if let Some(raw_path) = with_map.get(Value::String("path".to_string())) {
                collect_path_values(raw_path, &mut paths);
            }
        }
    }

    paths
}

fn collect_path_values(node: &Value, out: &mut Vec<String>) {
    match node {
        Value::String(path) => out.extend(
            path.lines()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| value.to_string()),
        ),
        Value::Sequence(seq) => {
            for item in seq {
                collect_path_values(item, out);
            }
        }
        _ => {}
    }
}

pub fn validate_workflow_contract(
    workflow_text: &str,
    config: &CiConfig,
    scan: &RepoScan,
    profile: ProjectProfile,
) -> Result<WorkflowContractCheck, String> {
    let workflow = parse_yaml_for_jobs(workflow_text)?;
    let mut check = WorkflowContractCheck::default();
    validate_static_workflow_contract(&workflow, workflow_text, config, &mut check);

    if workflow.get("jobs").is_none() {
        check
            .blocking
            .push("workflow missing jobs block".to_string());
        return Ok(check);
    }

    let jobs = list_jobs(&workflow);
    let required = required_job_names(profile, config, scan, scan.repo_root.as_path());
    for name in required {
        if !jobs.contains(&name) {
            check
                .blocking
                .push(format!("required workflow job '{name}' is missing"));
        }
    }

    let upload_paths = extract_upload_artifact_paths(&workflow);
    check.upload_paths = upload_paths.clone();
    for path in &upload_paths {
        if path.contains("..") || path == "." || path == "./" {
            check
                .blocking
                .push(format!("artifact upload path '{path}' is not allowed"));
            continue;
        }

        let allowed = if config.policy.artifact_allowlist_only {
            artifacts::is_allowed(path)
        } else {
            true
        };
        if !allowed {
            let item = format!("artifact upload path '{path}' is not in allowlist");
            if config.policy.artifact_allowlist_only {
                check.blocking.push(item);
            } else {
                check.warnings.push(item);
            }
        }
    }
    validate_upload_contract(&upload_paths, config, &mut check);

    Ok(check)
}

fn validate_static_workflow_contract(
    workflow: &Value,
    workflow_text: &str,
    config: &CiConfig,
    check: &mut WorkflowContractCheck,
) {
    let lowered = workflow_text.to_lowercase();
    if !workflow_text.contains("pull_request:") {
        check
            .blocking
            .push("workflow missing pull_request trigger".to_string());
    }
    if !workflow_text.contains("workflow_dispatch:") {
        check
            .blocking
            .push("workflow missing workflow_dispatch trigger".to_string());
    }
    if !(workflow_text.contains("permissions:") && workflow_text.contains("contents: read")) {
        check
            .blocking
            .push("workflow missing contents: read permission".to_string());
    }
    if workflow_run_steps_missing_pipefail(workflow) {
        check
            .blocking
            .push("workflow run block missing set -euo pipefail".to_string());
    }

    if config.yosys.enabled {
        if !lowered.contains("hierarchy -check") {
            check
                .blocking
                .push("workflow missing Yosys hierarchy -check".to_string());
        }
        if !lowered.contains("write_json")
            || !workflow_text.contains(&format!("{}/synth", config.sorted_artifacts_root()))
        {
            check
                .blocking
                .push("workflow missing Yosys JSON write_json artifact".to_string());
        }
    }

    if config.artifacts.store_tool_versions && !workflow_text.contains("tool-versions.txt") {
        check
            .blocking
            .push("workflow missing tool-versions.txt artifact".to_string());
    }
    if config.artifacts.generate_sha256sums && !workflow_text.contains("SHA256SUMS") {
        check
            .blocking
            .push("workflow missing SHA256SUMS artifact".to_string());
    }
}

fn workflow_run_steps_missing_pipefail(workflow: &Value) -> bool {
    let Some(jobs) = workflow.get("jobs").and_then(Value::as_mapping) else {
        return false;
    };

    for job in jobs.values() {
        let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
            continue;
        };
        for step in steps {
            let Some(run) = step.get("run").and_then(Value::as_str) else {
                continue;
            };
            if !run.contains("set -euo pipefail") {
                return true;
            }
        }
    }

    false
}

fn validate_upload_contract(
    upload_paths: &[String],
    config: &CiConfig,
    check: &mut WorkflowContractCheck,
) {
    if upload_paths.is_empty() {
        return;
    }

    let has_synth_json = upload_paths
        .iter()
        .any(|path| path.contains("/synth/") && path.ends_with(".json"));
    if config.yosys.enabled && !has_synth_json {
        check
            .blocking
            .push("workflow does not upload Yosys JSON artifacts".to_string());
    }

    let has_tool_versions = upload_paths
        .iter()
        .any(|path| path.ends_with("/logs/tool-versions.txt"));
    if config.artifacts.store_tool_versions && !has_tool_versions {
        check
            .blocking
            .push("workflow does not upload tool-versions.txt artifact".to_string());
    }

    let has_sha256sums = upload_paths
        .iter()
        .any(|path| path.ends_with("/SHA256SUMS"));
    if config.artifacts.generate_sha256sums && !has_sha256sums {
        check
            .blocking
            .push("workflow does not upload SHA256SUMS artifact".to_string());
    }
}
