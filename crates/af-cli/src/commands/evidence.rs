// SPDX-License-Identifier: Apache-2.0
//
// `af evidence ingest` handler and the shared signal/release-gate helpers.
//
// Extracted from main.rs to keep the binary entrypoint compact. Public types
// EvidenceKind / EvidenceStatus live in main.rs alongside their clap
// argument definitions; this module provides the implementation only.

use crate::{to_pretty_json, CliError, CliOutput, EvidenceKind, EvidenceStatus};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct EvidenceIngestReport {
    schema_version: &'static str,
    kind: &'static str,
    status: &'static str,
    evidence_kind: String,
    evidence_status: String,
    core: Option<String>,
    tool: Option<String>,
    source: EvidenceSource,
    copied_artifact: PathBuf,
    output: PathBuf,
    signals: Vec<String>,
    release_gate: EvidenceGate,
    limitations: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ci_run: Option<CiRunRecord>,
}

#[derive(Debug, Deserialize)]
struct CiRunIngestInput {
    #[serde(default)]
    workflow_run_url: Option<String>,
    commit_sha: String,
    conclusion: String,
    #[serde(default)]
    artifact_bundle: Option<String>,
    #[serde(default)]
    sha256sums: Option<String>,
}

#[derive(Debug, Serialize)]
struct CiRunRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    workflow_run_url: Option<String>,
    commit_sha: String,
    conclusion: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    artifact_bundle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sha256sums: Option<String>,
}

const CI_RUN_ALLOWED_CONCLUSIONS: &[&str] = &[
    "success",
    "failure",
    "cancelled",
    "neutral",
    "timed_out",
    "action_required",
    "skipped",
    "stale",
];

fn parse_ci_run_input(bytes: &[u8]) -> Result<(CiRunIngestInput, EvidenceStatus), CliError> {
    let input: CiRunIngestInput = serde_json::from_slice(bytes).map_err(|err| {
        CliError::new(
            "AF_EVIDENCE_CI_RUN_INVALID",
            format!("failed to parse `ci-run` evidence JSON: {err}"),
            "Pass a JSON document with required fields `commit_sha` and `conclusion`, plus optional `workflow_run_url`, `artifact_bundle`, `sha256sums`.",
            2,
        )
    })?;
    let sha = input.commit_sha.trim();
    if sha.is_empty() {
        return Err(CliError::new(
            "AF_EVIDENCE_CI_RUN_INVALID",
            "`ci-run` evidence requires a non-empty `commit_sha`.".to_string(),
            "Set `commit_sha` to the git revision the workflow ran against (typically `${{ github.sha }}`).",
            2,
        ));
    }
    if sha.len() < 7 || sha.len() > 64 {
        return Err(CliError::new(
            "AF_EVIDENCE_CI_RUN_INVALID",
            format!(
                "`ci-run` commit_sha must be 7-64 hex characters; got {} characters.",
                sha.len()
            ),
            "Use the short or full git SHA (7-64 hex chars).",
            2,
        ));
    }
    if !sha.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(CliError::new(
            "AF_EVIDENCE_CI_RUN_INVALID",
            "`ci-run` commit_sha must contain only hex characters [0-9a-fA-F].".to_string(),
            "Use the git SHA, not a branch name or symbolic ref.",
            2,
        ));
    }
    let conclusion = input.conclusion.trim().to_ascii_lowercase();
    if !CI_RUN_ALLOWED_CONCLUSIONS
        .iter()
        .any(|allowed| *allowed == conclusion)
    {
        return Err(CliError::new(
            "AF_EVIDENCE_CI_RUN_INVALID",
            format!(
                "`ci-run` conclusion `{}` is not in the allowed set {:?}.",
                input.conclusion, CI_RUN_ALLOWED_CONCLUSIONS
            ),
            "Use the GitHub Actions `conclusion` value: success | failure | cancelled | neutral | timed_out | action_required | skipped | stale.",
            2,
        ));
    }
    let status = if conclusion == "success" {
        EvidenceStatus::Passed
    } else {
        EvidenceStatus::Failed
    };
    Ok((
        CiRunIngestInput {
            workflow_run_url: input.workflow_run_url,
            commit_sha: sha.to_string(),
            conclusion,
            artifact_bundle: input.artifact_bundle,
            sha256sums: input.sha256sums,
        },
        status,
    ))
}

#[derive(Debug, Serialize)]
struct EvidenceSource {
    path: PathBuf,
    byte_len: u64,
    line_count: usize,
    fingerprint_fnv1a64: String,
}

#[derive(Debug, Serialize)]
struct EvidenceGate {
    status: String,
    reason: String,
    required_for_release: bool,
}

pub fn evidence_ingest(
    kind: EvidenceKind,
    input: &Path,
    core: Option<&str>,
    tool: Option<&str>,
    status: Option<EvidenceStatus>,
    output: Option<&PathBuf>,
    build_root: &Path,
) -> Result<CliOutput, CliError> {
    let bytes = fs::read(input).map_err(|err| {
        CliError::new(
            "AF_EVIDENCE_READ_FAILED",
            format!("failed to read evidence input `{}`: {err}", input.display()),
            "Pass --input <path> pointing to a readable simulator, lint, formal, synthesis, PnR, programming, or hardware evidence artifact.",
            2,
        )
    })?;
    let text = String::from_utf8_lossy(&bytes);
    let (ci_run_record, inferred_status, signals_override) = if matches!(kind, EvidenceKind::CiRun)
    {
        let (parsed, parsed_status) = parse_ci_run_input(&bytes)?;
        let signals = vec![
            format!("ci_run.conclusion={}", parsed.conclusion),
            format!("ci_run.commit_sha={}", parsed.commit_sha),
        ];
        let record = CiRunRecord {
            workflow_run_url: parsed.workflow_run_url.clone(),
            commit_sha: parsed.commit_sha.clone(),
            conclusion: parsed.conclusion.clone(),
            artifact_bundle: parsed.artifact_bundle.clone(),
            sha256sums: parsed.sha256sums.clone(),
        };
        (Some(record), parsed_status, Some(signals))
    } else {
        (None, infer_evidence_status(&text), None)
    };
    let evidence_status = status.unwrap_or(inferred_status);
    let signals = signals_override.unwrap_or_else(|| evidence_signals(&text, evidence_status));

    let input_name = input
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("evidence");
    let safe_input_name = safe_artifact_name(input_name);
    let copied_artifact = build_root
        .join("evidence")
        .join(kind.as_str())
        .join(&safe_input_name);
    if let Some(parent) = copied_artifact.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            CliError::new(
                "AF_EVIDENCE_WRITE_FAILED",
                format!(
                    "failed to create evidence artifact directory `{}`: {err}",
                    parent.display()
                ),
                "Check build-root permissions or choose a writable --build-root.",
                5,
            )
        })?;
    }
    if input != copied_artifact {
        fs::write(&copied_artifact, &bytes).map_err(|err| {
            CliError::new(
                "AF_EVIDENCE_WRITE_FAILED",
                format!(
                    "failed to copy evidence artifact to `{}`: {err}",
                    copied_artifact.display()
                ),
                "Check build-root permissions or choose a writable --build-root.",
                5,
            )
        })?;
    }

    let output_path = output.cloned().unwrap_or_else(|| {
        build_root.join("reports/evidence").join(format!(
            "{}-{}.json",
            kind.report_stem(),
            safe_artifact_name(input_name)
        ))
    });
    let gate = evidence_gate(evidence_status);
    let report = EvidenceIngestReport {
        schema_version: "0.1",
        kind: "accelfury.evidence_ingest",
        status: "passed",
        evidence_kind: kind.as_str().to_string(),
        evidence_status: evidence_status.as_str().to_string(),
        core: core.map(str::to_string),
        tool: tool.map(str::to_string),
        source: EvidenceSource {
            path: input.to_path_buf(),
            byte_len: bytes.len() as u64,
            line_count: text.lines().count(),
            fingerprint_fnv1a64: format!("{:016x}", fnv1a64(&bytes)),
        },
        copied_artifact,
        output: output_path.clone(),
        signals,
        release_gate: gate,
        limitations: vec![
            "Evidence ingestion normalizes existing artifacts; it does not rerun the originating tool."
                .to_string(),
            "fingerprint_fnv1a64 is a deterministic content fingerprint, not a cryptographic signature."
                .to_string(),
        ],
        ci_run: ci_run_record,
    };

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            CliError::new(
                "AF_EVIDENCE_WRITE_FAILED",
                format!(
                    "failed to create evidence report directory `{}`: {err}",
                    parent.display()
                ),
                "Check build-root permissions or choose a writable --output path.",
                5,
            )
        })?;
    }
    fs::write(&output_path, to_pretty_json(&report)).map_err(|err| {
        CliError::new(
            "AF_EVIDENCE_WRITE_FAILED",
            format!(
                "failed to write evidence report `{}`: {err}",
                output_path.display()
            ),
            "Check build-root permissions or choose a writable --output path.",
            5,
        )
    })?;

    Ok(CliOutput {
        human: format!("evidence report written: {}", output_path.display()),
        json: json!(report),
    })
}

fn infer_evidence_status(text: &str) -> EvidenceStatus {
    let lowered = text.to_ascii_lowercase();
    if lowered.contains("fatal")
        || lowered.contains(" assertion failed")
        || lowered.contains("failed")
        || lowered.contains(" error:")
        || lowered.contains("errors: 1")
    {
        EvidenceStatus::Failed
    } else if lowered.contains("pass")
        || lowered.contains("passed")
        || lowered.contains("success")
        || lowered.contains("0 errors")
    {
        EvidenceStatus::Passed
    } else {
        EvidenceStatus::Unknown
    }
}

fn evidence_signals(text: &str, status: EvidenceStatus) -> Vec<String> {
    let mut signals = Vec::new();
    let lowered = text.to_ascii_lowercase();
    for marker in [
        "pass", "passed", "success", "failed", "fatal", "error", "0 errors",
    ] {
        if lowered.contains(marker) {
            signals.push(marker.to_string());
        }
    }
    if signals.is_empty() {
        signals.push(format!("status:{}", status.as_str()));
    }
    signals.sort();
    signals.dedup();
    signals
}

fn evidence_gate(status: EvidenceStatus) -> EvidenceGate {
    match status {
        EvidenceStatus::Passed => EvidenceGate {
            status: "satisfied".to_string(),
            reason: "ingested evidence status is passed".to_string(),
            required_for_release: true,
        },
        EvidenceStatus::Warning => EvidenceGate {
            status: "warning".to_string(),
            reason: "ingested evidence contains warnings and needs review before release"
                .to_string(),
            required_for_release: true,
        },
        EvidenceStatus::Failed => EvidenceGate {
            status: "blocked".to_string(),
            reason: "ingested evidence status is failed".to_string(),
            required_for_release: true,
        },
        EvidenceStatus::Unknown => EvidenceGate {
            status: "blocked".to_string(),
            reason:
                "evidence status could not be inferred; pass --status after reviewing the artifact"
                    .to_string(),
            required_for_release: true,
        },
    }
}

fn safe_artifact_name(name: &str) -> String {
    let sanitized = name
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '_',
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "evidence".to_string()
    } else {
        sanitized
    }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
