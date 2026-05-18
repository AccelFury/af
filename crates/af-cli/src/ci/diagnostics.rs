// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub enum Severity {
    Blocker,
    Warning,
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub severity: Severity,
    pub message: String,
}

impl Finding {
    pub fn blocker(msg: impl Into<String>) -> Self {
        Self {
            severity: Severity::Blocker,
            message: msg.into(),
        }
    }

    pub fn warning(msg: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: msg.into(),
        }
    }
}

pub fn problem_classes(blocking: &[String], warnings: &[String]) -> Vec<String> {
    let mut classes = BTreeSet::new();
    for message in blocking.iter().chain(warnings.iter()) {
        classes.insert(classify_problem(message).to_string());
    }
    classes.into_iter().collect()
}

fn classify_problem(message: &str) -> &'static str {
    let lowered = message.to_lowercase();
    if lowered.contains("configured core top") {
        "config_top_not_detected"
    } else if lowered.contains("workflow file") || lowered.contains("cannot read workflow") {
        "workflow_missing"
    } else if lowered.contains("valid yaml") || lowered.contains("invalid yaml") {
        "workflow_yaml_invalid"
    } else if lowered.contains("jobs block") {
        "workflow_jobs_missing"
    } else if lowered.contains("required workflow job") {
        "workflow_job_missing"
    } else if lowered.contains("pull_request trigger")
        || lowered.contains("workflow_dispatch trigger")
    {
        "workflow_trigger_missing"
    } else if lowered.contains("contents: read permission") {
        "workflow_permissions_missing"
    } else if lowered.contains("set -euo pipefail") {
        "workflow_shell_safety_missing"
    } else if lowered.contains("vendor-tool") {
        "vendor_tool_policy_violation"
    } else if lowered.contains("piped into shell") {
        "unsafe_shell_pipe"
    } else if lowered.contains("wildcard artifact")
        || lowered.contains("artifact upload path")
            && (lowered.contains("not allowed") || lowered.contains(".."))
    {
        "artifact_upload_unsafe"
    } else if lowered.contains("not in allowlist") {
        "artifact_allowlist_violation"
    } else if lowered.contains("secrets")
        || lowered.contains("credential")
        || lowered.contains(".ssh")
    {
        "secret_artifact_policy_violation"
    } else if lowered.contains("yosys hierarchy") || lowered.contains("hierarchy -check") {
        "yosys_hierarchy_check_missing"
    } else if lowered.contains("yosys json") || lowered.contains("write_json") {
        "synth_json_missing"
    } else if lowered.contains("tool-versions.txt") {
        "tool_versions_missing"
    } else if lowered.contains("sha256sums") {
        "sha256_missing"
    } else if lowered.contains("board") && lowered.contains("top") {
        "board_top_missing"
    } else if lowered.contains("constraints") {
        "board_constraints_missing"
    } else if lowered.contains("incomplete") && lowered.contains("board") {
        "board_profile_incomplete"
    } else if lowered.contains("docs/ci.md") {
        "docs_ci_missing"
    } else if lowered.contains("pr checklist") {
        "pr_template_missing"
    } else if lowered.contains("artifact root") {
        "artifact_root_nonstandard"
    } else if lowered.contains("path from config") {
        "config_path_missing"
    } else {
        "ci_contract_issue"
    }
}
