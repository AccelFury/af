// SPDX-License-Identifier: Apache-2.0

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn read_text(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|_| panic!("cannot read {}", path.display()))
}

fn tracked_files(root: &Path) -> Vec<String> {
    let output = std::process::Command::new("git")
        .arg("ls-files")
        .current_dir(root)
        .output()
        .unwrap_or_else(|err| panic!("git ls-files failed to start: {err}"));
    assert!(
        output.status.success(),
        "git ls-files failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn public_tree_excludes_internal_work_surfaces() {
    let root = repo_root();
    let agent_docs = ["docs/", "agent/"].concat();
    let agent_reports = ["reports/", "agent/"].concat();
    let runtime_dir = ["run", "time/"].concat();
    let private_design_dir = ["docs/", "complex", "_core/"].concat();
    let private_design_docs = ["docs/", "complex", "-cores/"].concat();
    let rfc_dir = ["docs/", "r", "fcs/"].concat();
    let provenance_file = ["docs/", "pro", "venance.md"].concat();
    let agent_pr_schema = ["schemas/", "agent", "-pr.schema.json"].concat();
    let request_schema = ["schemas/", "agent", "-request.schema.json"].concat();
    let agent_issue_template = [".github/ISSUE_TEMPLATE/", "agent", "_request.yml"].concat();
    let internal_skill_installer =
        ["scripts/", "install-project-", "cod", "ex", "-skills.sh"].concat();
    // TODO.md is intentionally public: it surfaces open and recently-closed
    // lifecycle gaps (AF.TODO.* identifiers) that public reviewers may need to
    // cite. Private worklogs live elsewhere.
    // Project `skills/af-*` is a tracked source of truth for installable Codex
    // skills, not per-user state. Keep private indexes and runtime workspaces
    // forbidden without rejecting the canonical skill sources.
    let skills_index = ["skills/", "INDEX.md"].concat();
    let forbidden_prefixes = [
        agent_docs.as_str(),
        agent_reports.as_str(),
        runtime_dir.as_str(),
        private_design_dir.as_str(),
        private_design_docs.as_str(),
        rfc_dir.as_str(),
    ];
    let forbidden_files = [
        provenance_file.as_str(),
        agent_pr_schema.as_str(),
        request_schema.as_str(),
        agent_issue_template.as_str(),
        internal_skill_installer.as_str(),
        skills_index.as_str(),
    ];

    for file in tracked_files(&root) {
        assert!(
            !forbidden_prefixes
                .iter()
                .any(|prefix| file.starts_with(prefix)),
            "internal surface must not be tracked publicly: {file}"
        );
        assert!(
            !forbidden_files.contains(&file.as_str()),
            "internal file must not be tracked publicly: {file}"
        );
    }
}

#[test]
fn public_docs_do_not_link_to_internal_workspace() {
    let root = repo_root();
    for rel in [
        "README.md",
        "CONTRIBUTING.md",
        "docs/cli-reference.md",
        "docs/production-readiness.md",
        "reports/README.md",
    ] {
        let text = read_text(&root.join(rel));
        let agent_docs = ["docs/", "agent/"].concat();
        let agent_reports = ["reports/", "agent/"].concat();
        let skills_index = ["skills/", "INDEX.md"].concat();
        let private_request_tag = ["agent", "-request"].concat();
        let private_author_marker = ["created", "_by"].concat();
        let openai_generated = ["Open", "AI GPT"].concat();
        let local_home = ["/home/", "elixirus"].concat();
        let private_workspace = [".", "cod", "ex/"].concat();
        for forbidden in [
            agent_docs.as_str(),
            agent_reports.as_str(),
            skills_index.as_str(),
            private_request_tag.as_str(),
            private_author_marker.as_str(),
            openai_generated.as_str(),
            local_home.as_str(),
            private_workspace.as_str(),
        ] {
            assert!(
                !text.contains(forbidden),
                "{rel} must not expose private/internal marker {forbidden:?}"
            );
        }
    }
}

#[test]
fn public_readme_is_llm_friendly_without_private_governance() {
    let root = repo_root();
    let readme = read_text(&root.join("README.md"));
    for required in [
        "What `af` Does",
        "What `af` Does Not Prove",
        "Quick Start",
        "Common Workflows",
        "LLM and Automation Guidance",
        "cargo run -p af-cli --bin af -- doctor --json",
        "cargo test --workspace",
        "ignored workspace",
        "docs/production-readiness.md",
    ] {
        assert!(readme.contains(required), "README missing {required:?}");
    }
}
