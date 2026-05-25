// SPDX-License-Identifier: Apache-2.0
//! Agent issue interface — offline helpers for LLM/AI agents that drive
//! `af` from the outside. The module produces:
//!
//! - a deterministic context bundle (af version, reproducibility, commit
//!   SHA, repo discovery) for inclusion in issue bodies,
//! - rendered Markdown issue bodies for each supported issue kind,
//! - a pre-filled GitHub `new issue` URL,
//! - a ready-to-paste `gh issue create` command line.
//!
//! Hard rule: this module **never** hits the network. It does not invoke
//! `gh`, never POSTs, and never reads tokens. Submission is the caller's
//! explicit action.

use af_report::Reproducibility;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Issue kinds supported by `af agent`. Each maps 1:1 to a file under
/// `.github/ISSUE_TEMPLATE/`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IssueKind {
    Bug,
    Feature,
    Question,
    BoardBringup,
    BoardRequest,
    IpRequest,
    AgentReport,
}

impl IssueKind {
    pub const ALL: &'static [IssueKind] = &[
        IssueKind::Bug,
        IssueKind::Feature,
        IssueKind::Question,
        IssueKind::BoardBringup,
        IssueKind::BoardRequest,
        IssueKind::IpRequest,
        IssueKind::AgentReport,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            IssueKind::Bug => "bug",
            IssueKind::Feature => "feature",
            IssueKind::Question => "question",
            IssueKind::BoardBringup => "board-bringup",
            IssueKind::BoardRequest => "board-request",
            IssueKind::IpRequest => "ip-request",
            IssueKind::AgentReport => "agent-report",
        }
    }

    pub fn template_file(self) -> &'static str {
        match self {
            IssueKind::Bug => "bug_report.md",
            IssueKind::Feature => "feature_request.md",
            IssueKind::Question => "question.md",
            IssueKind::BoardBringup => "board_bringup.md",
            IssueKind::BoardRequest => "board_request.md",
            IssueKind::IpRequest => "ip_request.md",
            IssueKind::AgentReport => "agent_report.md",
        }
    }

    pub fn default_labels(self) -> &'static [&'static str] {
        match self {
            IssueKind::Bug => &["bug", "agent-generated"],
            IssueKind::Feature => &["enhancement", "agent-generated"],
            IssueKind::Question => &["question", "agent-generated"],
            IssueKind::BoardBringup => &["hardware", "agent-generated"],
            IssueKind::BoardRequest => &["board", "agent-generated"],
            IssueKind::IpRequest => &["ip-request", "agent-generated"],
            IssueKind::AgentReport => &["agent-generated"],
        }
    }

    pub fn title_prefix(self) -> &'static str {
        match self {
            IssueKind::Bug => "[bug]",
            IssueKind::Feature => "[feat]",
            IssueKind::Question => "[question]",
            IssueKind::BoardBringup => "[board-bringup]",
            IssueKind::BoardRequest => "[board-request]",
            IssueKind::IpRequest => "[ip-request]",
            IssueKind::AgentReport => "[agent]",
        }
    }
}

impl std::str::FromStr for IssueKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for k in IssueKind::ALL {
            if k.as_str() == s {
                return Ok(*k);
            }
        }
        Err(format!(
            "unknown issue kind `{s}` (expected one of: {})",
            IssueKind::ALL
                .iter()
                .map(|k| k.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

/// Context bundle collected for inclusion in issue bodies.
#[derive(Clone, Debug, Serialize)]
pub struct AgentContext {
    pub af_version: String,
    pub reproducibility: Reproducibility,
    pub current_commit_sha: Option<String>,
    pub repo_owner: String,
    pub repo_name: String,
    pub working_dir: PathBuf,
}

impl AgentContext {
    pub fn gather(repo_root: &Path) -> Self {
        let repro = Reproducibility::capture(&[]);
        let sha = current_commit_sha(repo_root);
        let (owner, name) =
            discover_github_repo(repo_root).unwrap_or_else(|| ("AccelFury".into(), "af".into()));
        Self {
            af_version: env!("CARGO_PKG_VERSION").to_string(),
            reproducibility: repro,
            current_commit_sha: sha,
            repo_owner: owner,
            repo_name: name,
            working_dir: repo_root.to_path_buf(),
        }
    }
}

/// Inspect the working tree for the HEAD commit SHA via `git rev-parse`.
/// Returns `None` outside a git repo or when `git` is missing.
fn current_commit_sha(repo_root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .current_dir(repo_root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sha.is_empty() {
        None
    } else {
        Some(sha)
    }
}

/// Parse `git config remote.origin.url` and return `(owner, repo)` if it
/// points at a GitHub repository. Supports `git@github.com:<o>/<r>.git`
/// and `https://github.com/<o>/<r>(.git)?` forms. Returns `None` for
/// other hosts.
pub fn discover_github_repo(repo_root: &Path) -> Option<(String, String)> {
    let output = Command::new("git")
        .args(["config", "--get", "remote.origin.url"])
        .current_dir(repo_root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    parse_github_remote(&url)
}

fn parse_github_remote(url: &str) -> Option<(String, String)> {
    let tail = if let Some(rest) = url.strip_prefix("git@github.com:") {
        rest
    } else {
        url.strip_prefix("https://github.com/")
            .or_else(|| url.strip_prefix("http://github.com/"))
            .or_else(|| url.strip_prefix("ssh://git@github.com/"))?
    };
    let tail = tail.strip_suffix(".git").unwrap_or(tail);
    let mut parts = tail.splitn(2, '/');
    let owner = parts.next()?.trim();
    let name = parts.next()?.trim();
    if owner.is_empty() || name.is_empty() {
        return None;
    }
    Some((owner.to_string(), name.to_string()))
}

/// RFC 3986 percent-encoding for query-string components. Preserves
/// unreserved set (ALPHA / DIGIT / `-._~`) and percent-encodes
/// everything else, including spaces, as `%XX`.
pub fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        let unreserved = byte.is_ascii_alphanumeric()
            || byte == b'-'
            || byte == b'_'
            || byte == b'.'
            || byte == b'~';
        if unreserved {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Render the canonical Markdown body for a kind. Body shape is fixed
/// across kinds; only the `## Reproduction` hint text differs.
pub fn render_issue_markdown(
    kind: IssueKind,
    title: &str,
    summary: Option<&str>,
    context: &AgentContext,
    from_error_json: Option<&str>,
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "<!-- Generated by `af agent issue --kind {}` -->\n\n",
        kind.as_str()
    ));
    out.push_str(&format!("# {} {}\n\n", kind.title_prefix(), title));

    out.push_str("## Summary\n\n");
    if let Some(s) = summary {
        out.push_str(s.trim_end());
        out.push('\n');
    } else {
        out.push_str("<!-- One sentence: what `af` invocation or workflow produced this. -->\n");
    }
    out.push('\n');

    out.push_str("## Reproduction\n\n");
    out.push_str("```bash\n");
    out.push_str("# Exact commands the agent ran, always with --json.\n");
    out.push_str("```\n\n");

    if let Some(json) = from_error_json {
        out.push_str("## Structured failure\n\n");
        out.push_str("```json\n");
        out.push_str(json.trim_end());
        out.push('\n');
        out.push_str("```\n\n");
    }

    out.push_str("## Agent context\n\n");
    let sha = context.current_commit_sha.as_deref().unwrap_or("unknown");
    out.push_str(&format!("- `af_version`: `{}`\n", context.af_version));
    out.push_str(&format!("- `commit_sha`: `{sha}`\n"));
    out.push_str(&format!(
        "- `host_os` / `host_arch`: `{}` / `{}`\n",
        context.reproducibility.host_os, context.reproducibility.host_arch
    ));
    out.push_str(&format!(
        "- `environment_hash`: `{}`\n",
        context.reproducibility.environment_hash
    ));
    out.push_str(&format!(
        "- `repo`: `{}/{}`\n",
        context.repo_owner, context.repo_name
    ));
    out.push_str(&format!(
        "- `working_dir`: `{}`\n",
        context.working_dir.display()
    ));
    let agent_name = std::env::var("AF_AGENT_NAME").unwrap_or_else(|_| "unspecified".to_string());
    out.push_str(&format!("- `agent_name`: `{agent_name}`\n"));
    out.push_str("- `automated_submission`: `true`\n\n");

    out.push_str("## Why this is being filed by an agent\n\n");
    out.push_str(
        "<!-- One paragraph: recurring failure, contract gap, missing capability, doc lie, ... -->\n\n",
    );

    out.push_str("## Suggested next step (optional)\n\n");
    out.push_str(
        "<!-- One paragraph: minimal change that would resolve the issue, or a specific question. -->\n",
    );

    out
}

/// Build the GitHub pre-filled `new issue` URL. Returns `(url, warnings)`.
/// A warning is emitted when the URL exceeds 7500 characters (GitHub
/// silently truncates very long pre-filled bodies).
pub fn render_gh_url(
    owner: &str,
    repo: &str,
    kind: IssueKind,
    title: &str,
    body: &str,
    labels: &[&str],
) -> (String, Vec<String>) {
    let labels_param = labels.join(",");
    let url = format!(
        "https://github.com/{owner}/{repo}/issues/new?template={tpl}&title={title}&body={body}&labels={labels}",
        tpl = percent_encode(kind.template_file()),
        title = percent_encode(title),
        body = percent_encode(body),
        labels = percent_encode(&labels_param),
    );
    let mut warnings = Vec::new();
    if url.len() > 7500 {
        warnings.push(format!(
            "URL length is {} bytes; GitHub typically truncates pre-filled bodies past ~7500. Prefer `gh issue create --body-file <path>`.",
            url.len()
        ));
    }
    (url, warnings)
}

/// Build a `gh issue create` invocation as a single shell-safe string.
/// Caller is responsible for actually running it — this function never
/// invokes a subprocess.
pub fn render_gh_cli(
    owner: &str,
    repo: &str,
    title: &str,
    body_file: &Path,
    labels: &[&str],
) -> String {
    let mut out = format!(
        "gh issue create --repo {} --title {} --body-file {}",
        shell_quote(&format!("{owner}/{repo}")),
        shell_quote(title),
        shell_quote(&body_file.display().to_string()),
    );
    for label in labels {
        out.push_str(" --label ");
        out.push_str(&shell_quote(label));
    }
    out
}

/// Minimal POSIX single-quote escape: `O'Brien` → `'O'\''Brien'`.
fn shell_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_kind_roundtrip() {
        for k in IssueKind::ALL {
            let s = k.as_str();
            let parsed: IssueKind = s.parse().unwrap();
            assert_eq!(parsed, *k);
        }
        assert!("not-a-kind".parse::<IssueKind>().is_err());
    }

    #[test]
    fn template_file_mapping_is_complete() {
        for k in IssueKind::ALL {
            assert!(!k.template_file().is_empty());
            assert!(k.template_file().ends_with(".md"));
        }
    }

    #[test]
    fn default_labels_always_contain_agent_generated() {
        for k in IssueKind::ALL {
            assert!(
                k.default_labels().contains(&"agent-generated"),
                "kind `{}` missing agent-generated label",
                k.as_str()
            );
        }
    }

    #[test]
    fn parse_github_remote_handles_ssh_and_https() {
        assert_eq!(
            parse_github_remote("git@github.com:AccelFury/af.git"),
            Some(("AccelFury".into(), "af".into()))
        );
        assert_eq!(
            parse_github_remote("https://github.com/AccelFury/af.git"),
            Some(("AccelFury".into(), "af".into()))
        );
        assert_eq!(
            parse_github_remote("https://github.com/AccelFury/af"),
            Some(("AccelFury".into(), "af".into()))
        );
        assert_eq!(parse_github_remote("https://gitlab.com/x/y.git"), None);
        assert_eq!(parse_github_remote(""), None);
    }

    #[test]
    fn percent_encode_preserves_unreserved_and_encodes_others() {
        assert_eq!(percent_encode("Hello-World_1.0~"), "Hello-World_1.0~");
        assert_eq!(percent_encode("Hello World"), "Hello%20World");
        assert_eq!(percent_encode("a/b?c=d"), "a%2Fb%3Fc%3Dd");
        // Unicode → multi-byte UTF-8, each byte encoded
        assert_eq!(percent_encode("é"), "%C3%A9");
    }

    #[test]
    fn render_gh_url_includes_template_and_labels() {
        let (url, warnings) = render_gh_url(
            "AccelFury",
            "af",
            IssueKind::Bug,
            "Hello, world",
            "body",
            &["bug", "agent-generated"],
        );
        assert!(url.starts_with("https://github.com/AccelFury/af/issues/new?"));
        assert!(url.contains("template=bug_report.md"));
        assert!(url.contains("title=Hello%2C%20world"));
        assert!(url.contains("labels=bug%2Cagent-generated"));
        assert!(warnings.is_empty());
    }

    #[test]
    fn render_gh_cli_quotes_special_chars() {
        let cli = render_gh_cli(
            "AccelFury",
            "af",
            "It's broken",
            Path::new("/tmp/body.md"),
            &["bug"],
        );
        assert!(cli.contains("gh issue create"));
        assert!(cli.contains("--repo 'AccelFury/af'"));
        assert!(cli.contains("--body-file '/tmp/body.md'"));
        assert!(cli.contains("--label 'bug'"));
        // single-quote escape pattern: '\''  (close, escaped quote, reopen)
        assert!(cli.contains("'It'\\''s broken'"));
    }

    #[test]
    fn shell_quote_escapes_inner_apostrophe() {
        assert_eq!(shell_quote("O'Brien"), "'O'\\''Brien'");
        assert_eq!(shell_quote("simple"), "'simple'");
    }

    #[test]
    fn render_issue_markdown_includes_required_sections() {
        let context = AgentContext {
            af_version: "0.1.0".to_string(),
            reproducibility: Reproducibility {
                host_os: "linux".to_string(),
                host_arch: "x86_64".to_string(),
                environment_hash: "deadbeef00000000".to_string(),
                af_version: "0.1.0".to_string(),
            },
            current_commit_sha: Some("abc1234".to_string()),
            repo_owner: "AccelFury".to_string(),
            repo_name: "af".to_string(),
            working_dir: PathBuf::from("/tmp/repo"),
        };
        let md = render_issue_markdown(
            IssueKind::AgentReport,
            "smoke",
            Some("smoke summary"),
            &context,
            None,
        );
        assert!(md.contains("## Summary"));
        assert!(md.contains("smoke summary"));
        assert!(md.contains("## Agent context"));
        assert!(md.contains("`af_version`: `0.1.0`"));
        assert!(md.contains("`commit_sha`: `abc1234`"));
        assert!(md.contains("`environment_hash`: `deadbeef00000000`"));
        assert!(md.contains("`repo`: `AccelFury/af`"));
        assert!(md.contains("`automated_submission`: `true`"));
        // No "Structured failure" section without --from-error.
        assert!(!md.contains("## Structured failure"));
    }

    #[test]
    fn render_issue_markdown_emits_structured_failure_when_given() {
        let context = AgentContext {
            af_version: "0.1.0".to_string(),
            reproducibility: Reproducibility::capture(&[]),
            current_commit_sha: None,
            repo_owner: "AccelFury".to_string(),
            repo_name: "af".to_string(),
            working_dir: PathBuf::from("."),
        };
        let md = render_issue_markdown(
            IssueKind::Bug,
            "fail",
            None,
            &context,
            Some(r#"{"code":"AF_X","message":"y","hint":"z","exit_code":2}"#),
        );
        assert!(md.contains("## Structured failure"));
        assert!(md.contains("\"AF_X\""));
        assert!(md.contains("`commit_sha`: `unknown`"));
    }
}
