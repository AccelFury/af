// SPDX-License-Identifier: Apache-2.0

use crate::ci::config::CiConfig;

#[derive(Debug, Default, Clone)]
pub struct PolicyFinding {
    pub blocking: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn validate_policy_from_config(config: &CiConfig, workflow: &str) -> PolicyFinding {
    let mut finding = PolicyFinding::default();
    let lowered = workflow.to_lowercase();

    if config.policy.no_vendor_tools_in_public_ci && has_vendor_tool_run(&lowered) {
        finding
            .blocking
            .push("workflow contains vendor-tool launch without explicit policy".to_string());
    }

    if workflow.contains("| sh") || workflow.contains("|bash") {
        finding
            .blocking
            .push("unsafe command piped into shell detected".to_string());
    }

    if workflow.contains("path: .\n")
        || workflow.contains("path: ./\n")
        || workflow.contains("path: .,")
    {
        finding
            .blocking
            .push("wildcard artifact upload path is not allowed".to_string());
    }

    if config.policy.no_unknown_script_execution && has_risky_paths(workflow) {
        finding
            .blocking
            .push("workflow can expose local secrets or credential artifacts".to_string());
    }

    if workflow.contains(".ssh") && config.policy.no_unknown_script_execution {
        finding
            .blocking
            .push("workflow can expose .ssh files".to_string());
    }

    if lowered.contains("secrets.") {
        finding.warnings.push(
            "workflow references secrets explicitly; avoid shipping them in reports".to_string(),
        );
    }

    finding
}

fn has_vendor_tool_run(lowered: &str) -> bool {
    [
        "vivado",
        "quartus",
        "impl",
        "powertools",
        "xilinx",
        "quartus_sh",
        "gw_sh",
        "programmer_cli",
        "diamond",
    ]
    .iter()
    .any(|tool| lowered.contains(tool))
}

fn has_risky_paths(workflow: &str) -> bool {
    workflow.contains(".env")
        || workflow.contains(".pem")
        || workflow.contains(".key")
        || workflow.contains("secret")
        || workflow.contains("password")
}
