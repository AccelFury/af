// SPDX-License-Identifier: Apache-2.0
//
// Integration tests for `af flash` and top-level `af report`.
//
// `af flash` is detect-only on MVP-2: under any PATH state it must
// surface a documented envelope without panicking. Top-level
// `af report` consumes either a `.af-build/...` directory or a saved
// AfReport JSON file and re-emits it.

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

fn af_no_path() -> Command {
    let mut cmd = Command::cargo_bin("af").expect("cargo bin `af`");
    cmd.env_clear()
        .env("HOME", "/tmp")
        .env("PATH", "/nonexistent");
    cmd
}

#[test]
fn flash_with_missing_build_dir_returns_envelope_without_panic() {
    let tmp = tempfile::TempDir::new().unwrap();
    let bogus = tmp.path().join("never-existed");
    let out = af()
        .args([
            "--json",
            "--build-root",
            tmp.path().to_str().unwrap(),
            "flash",
            bogus.to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    assert!(!out.status.success(), "flash on missing dir must fail");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON envelope");
    let code = value["code"].as_str().expect("code");
    assert!(
        code.starts_with("AF_"),
        "envelope must start with AF_, got {code}"
    );
}

#[test]
fn flash_under_empty_path_returns_envelope_without_panic() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dir = tmp.path().join("some-build-dir");
    std::fs::create_dir_all(&dir).unwrap();
    let out = af_no_path()
        .args([
            "--json",
            "--build-root",
            tmp.path().to_str().unwrap(),
            "flash",
            dir.to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    let exit = out.status.code().expect("exit");
    assert!(
        matches!(exit, 2 | 4 | 10),
        "flash exit code outside band: {exit}"
    );
    let _value: Value = serde_json::from_slice(&out.stdout).expect("JSON parses");
}

#[test]
fn report_on_missing_input_emits_warning_not_error() {
    // Pinned in Iter 7: `af report <missing>` is a warning-only path,
    // not an envelope error.
    let tmp = tempfile::TempDir::new().unwrap();
    let bogus = tmp.path().join("never-existed.json");
    let out = af()
        .args([
            "--json",
            "--build-root",
            tmp.path().to_str().unwrap(),
            "report",
            bogus.to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    assert_eq!(
        out.status.code(),
        Some(0),
        "report missing-input must exit 0"
    );
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    // Warnings appear in `report.warnings[]` (the embedded AfReport).
    let warnings = value
        .get("report")
        .and_then(|r| r.get("warnings"))
        .and_then(|w| w.as_array())
        .or_else(|| value.get("warnings").and_then(|w| w.as_array()));
    assert!(
        warnings.is_some_and(|w| !w.is_empty()),
        "report on missing input must surface non-empty warnings[]"
    );
}

#[test]
fn report_on_in_tree_examples_passes() {
    // Run report against a fresh build dir; warning-only is acceptable.
    let tmp = tempfile::TempDir::new().unwrap();
    let core = repo_root().join("examples").join("af-mod-add");
    // First populate build root via `core check`.
    let _ = af()
        .args([
            "--json",
            "--build-root",
            tmp.path().to_str().unwrap(),
            "core",
            "check",
            core.to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    // Then run report pointing to the build dir.
    let out = af()
        .args([
            "--json",
            "--build-root",
            tmp.path().to_str().unwrap(),
            "report",
            tmp.path().to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    assert_eq!(
        out.status.code(),
        Some(0),
        "report on populated build dir must exit 0"
    );
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert!(value.is_object());
}
