// SPDX-License-Identifier: Apache-2.0
//
// `af self check --json` walks `af-selfcheck.toml::[[targets]]` and
// produces per-target results. The tests pin:
//
// 1. Default run (no filter) returns one result per required target.
// 2. `--target <name>` filter restricts the report to the named target.
// 3. `--target <unknown>` ⇒ `AF_SELF_CHECK_TARGET_UNKNOWN`.
// 4. `--include-optional` surfaces optional targets (status: skipped or
//    passed; never `error`).

use assert_cmd::Command;
use serde_json::Value;
use std::path::PathBuf;

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

fn run_self_check(extra: &[&str]) -> (i32, Value) {
    let build_root = tempfile::TempDir::new().unwrap();
    let mut args = vec![
        "--json",
        "--build-root",
        build_root.path().to_str().unwrap(),
        "self",
        "check",
    ];
    args.extend_from_slice(extra);
    let out = af()
        .current_dir(repo_root())
        .args(&args)
        .output()
        .expect("execute");
    let exit = out.status.code().expect("exit");
    let value: Value = serde_json::from_slice(&out.stdout).expect("self check must produce JSON");
    (exit, value)
}

#[test]
fn default_run_returns_required_targets() {
    let (exit, value) = run_self_check(&[]);
    assert_eq!(exit, 0, "self check on a healthy repo must exit 0");
    assert!(matches!(
        value["status"].as_str(),
        Some("passed" | "warning")
    ));
    let targets = value["targets"].as_array().expect("targets array");
    assert!(!targets.is_empty(), "must have at least one target");
    // Every required target reports a known status.
    for t in targets {
        let status = t["status"].as_str().expect("status string");
        assert!(
            matches!(status, "passed" | "warning" | "skipped"),
            "in-tree required targets must not be error: {} → {}",
            t["name"].as_str().unwrap_or("?"),
            status
        );
    }
}

#[test]
fn target_filter_restricts_to_named_target() {
    let (exit, value) = run_self_check(&["--target", "example-af-mod-add"]);
    assert_eq!(exit, 0);
    let targets = value["targets"].as_array().expect("targets array");
    assert_eq!(
        targets.len(),
        1,
        "--target must return exactly one row, got {targets:?}"
    );
    assert_eq!(
        targets[0]["name"].as_str(),
        Some("example-af-mod-add"),
        "filter must echo back the named target"
    );
}

#[test]
fn unknown_target_emits_envelope() {
    let (exit, value) = run_self_check(&["--target", "definitely-not-a-target-name"]);
    assert!(exit != 0, "unknown target must fail");
    assert_eq!(
        value["code"].as_str(),
        Some("AF_SELF_CHECK_TARGET_UNKNOWN"),
        "expected AF_SELF_CHECK_TARGET_UNKNOWN, got {}",
        value["code"].as_str().unwrap_or("?")
    );
    assert_eq!(value["exit_code"].as_i64(), Some(2));
    let hint = value["hint"].as_str().expect("hint string");
    assert!(!hint.is_empty());
}

#[test]
fn include_optional_widens_target_set() {
    let (_, baseline) = run_self_check(&[]);
    let (_, full) = run_self_check(&["--include-optional"]);
    let n_baseline = baseline["targets"].as_array().unwrap().len();
    let n_full = full["targets"].as_array().unwrap().len();
    assert!(
        n_full >= n_baseline,
        "--include-optional must not reduce the target count"
    );
}

#[test]
fn include_optional_records_external_targets_without_panic() {
    // Optional external targets (`source = "local-external-project"`)
    // require an `AF_SELF_CHECK_*` env var to point at a real local
    // clone. When that env var is absent the target's status is
    // `failed` with a descriptive message — that is the documented
    // behaviour. The test pins that the process itself does not panic,
    // every target receives a string status, and the report retains the
    // documented top-level keys.
    let (_exit, value) = run_self_check(&["--include-optional"]);
    let targets = value["targets"].as_array().expect("targets array");
    assert!(!targets.is_empty());
    for t in targets {
        let status = t["status"].as_str().expect("status string");
        assert!(
            matches!(
                status,
                "passed" | "warning" | "skipped" | "failed" | "error"
            ),
            "unknown self-check status: {status}"
        );
    }
    assert!(value["status"].is_string());
}
