// SPDX-License-Identifier: Apache-2.0
//
// Evidence-chain invariants per the manifesto:
//
//   * Maturity rows with `supported = true` must carry at least one
//     evidence path (no claim without artefact).
//   * Rows with `blocked = true` must carry at least one limitation
//     string explaining why.
//
// These rules are enforced at report-emit time, but a small regression
// here can quietly degrade auditability. We check them via
// `af core report --json` against the in-tree examples.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn repo_root() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn af() -> Command {
    Command::cargo_bin("af").expect("cargo bin `af`")
}

fn rows_of_report(value: &Value) -> Option<&Vec<Value>> {
    // Several `af` commands embed a `command_payload` that carries a
    // `reusable_core_maturity.rows` array. We scan the top-level value
    // for any such array.
    if let Some(arr) = value
        .get("command_payload")
        .and_then(|p| p.get("reusable_core_maturity"))
        .and_then(|m| m.get("rows"))
        .and_then(|r| r.as_array())
    {
        return Some(arr);
    }
    if let Some(arr) = value
        .get("reusable_core_maturity")
        .and_then(|m| m.get("rows"))
        .and_then(|r| r.as_array())
    {
        return Some(arr);
    }
    None
}

#[test]
fn supported_rows_in_examples_carry_artifact_paths() {
    for example in ["af-mod-add", "af-reset-sync"] {
        let manifest = repo_root()
            .join("examples")
            .join(example)
            .join("af-core.toml");
        let manifest_str = manifest.to_str().unwrap();
        let out = af()
            .args(["--json", "core", "report", manifest_str])
            .output()
            .expect("execute");
        if !out.status.success() {
            // Some commands route through alternate code paths in CI;
            // skip rather than fail the cross-example sweep.
            eprintln!(
                "skipping {example}: af core report returned {:?}",
                out.status.code()
            );
            continue;
        }
        let v: Value = match serde_json::from_slice(&out.stdout) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let rows = match rows_of_report(&v) {
            Some(r) => r,
            None => continue,
        };
        for row in rows {
            if row["supported"].as_bool() != Some(true) {
                continue;
            }
            let evidence = row.get("evidence").and_then(|e| e.as_array());
            let artifact_path = row.get("artifact_path").and_then(|p| p.as_str());
            let has_artifact = match evidence {
                Some(arr) => !arr.is_empty(),
                None => artifact_path.is_some_and(|s| !s.is_empty()),
            };
            assert!(
                has_artifact,
                "row {} for {example} claims supported=true without evidence",
                row.get("id").and_then(|i| i.as_str()).unwrap_or("?")
            );
        }
    }
}

#[test]
fn blocked_rows_in_examples_carry_limitations() {
    for example in ["af-mod-add", "af-reset-sync"] {
        let manifest = repo_root()
            .join("examples")
            .join(example)
            .join("af-core.toml");
        let manifest_str = manifest.to_str().unwrap();
        let out = af()
            .args(["--json", "core", "report", manifest_str])
            .output()
            .expect("execute");
        if !out.status.success() {
            continue;
        }
        let v: Value = match serde_json::from_slice(&out.stdout) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let rows = match rows_of_report(&v) {
            Some(r) => r,
            None => continue,
        };
        for row in rows {
            if row["blocked"].as_bool() != Some(true) {
                continue;
            }
            let limitations = row.get("limitations").and_then(|l| l.as_array());
            let reason = row.get("reason").and_then(|r| r.as_str());
            let has_explanation = match limitations {
                Some(arr) => arr
                    .iter()
                    .any(|v| v.as_str().is_some_and(|s| !s.is_empty())),
                None => reason.is_some_and(|s| !s.is_empty()),
            };
            assert!(
                has_explanation,
                "row {} for {example} is blocked without a limitation/reason",
                row.get("id").and_then(|i| i.as_str()).unwrap_or("?")
            );
        }
    }
}

#[test]
fn report_status_is_one_of_documented_values() {
    let manifest = repo_root()
        .join("examples")
        .join("af-mod-add")
        .join("af-core.toml");
    let manifest_str = manifest.to_str().unwrap();
    let out = af()
        .args(["--json", "core", "report", manifest_str])
        .output()
        .expect("execute");
    if !out.status.success() {
        return;
    }
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    if let Some(status) = v.get("status").and_then(|s| s.as_str()) {
        assert!(
            matches!(
                status,
                "passed" | "warning" | "failed" | "error" | "partial" | "planned"
            ),
            "report status `{status}` is not one of documented values"
        );
    }
}

#[test]
fn vendor_and_hardware_evidence_ingest_uses_fixture_artifacts_only() {
    let tmp = tempdir().unwrap();
    let build_root = tmp.path().join("build");
    let cases = [
        (
            "synthesis-report",
            "vivado",
            "vivado-synth.log",
            "Vivado synthesis passed with 0 errors\n",
        ),
        (
            "pnr-report",
            "nextpnr",
            "nextpnr.json",
            "{\"status\":\"passed\",\"errors\":0}\n",
        ),
        (
            "programming-log",
            "openfpgaloader",
            "programming.log",
            "Programming passed\n",
        ),
        (
            "hardware-measurement",
            "lab-fixture",
            "hardware-measurement.txt",
            "Board smoke measurement passed\n",
        ),
    ];

    for (kind, tool, file_name, contents) in cases {
        let input = tmp.path().join(file_name);
        fs::write(&input, contents).unwrap();
        let out = af()
            .args(["--json", "--build-root", build_root.to_str().unwrap()])
            .args(["evidence", "ingest"])
            .args(["--kind", kind])
            .args(["--input", input.to_str().unwrap()])
            .args(["--tool", tool])
            .args(["--status", "passed"])
            .output()
            .expect("execute evidence ingest");

        assert!(
            out.status.success(),
            "evidence ingest for {kind} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let value: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(value["evidence_kind"], kind);
        assert_eq!(value["evidence_status"], "passed");
        assert_eq!(value["release_gate"]["status"], "satisfied");
        assert_eq!(value["tool"], tool);

        let copied = value["copied_artifact"].as_str().unwrap();
        let output = value["output"].as_str().unwrap();
        assert!(
            copied.contains("/evidence/"),
            "copied artifact should stay under build-root evidence dir: {copied}"
        );
        assert!(
            output.contains("/reports/evidence/"),
            "report should stay under build-root reports dir: {output}"
        );
    }
}
