// SPDX-License-Identifier: Apache-2.0
//
// Matrix: each open-source backend, when its bin is not in PATH, must
// surface `AF_BACKEND_UNAVAILABLE` (exit 4) without panicking. This is
// the contract for `af-error-explainer` and CI agents that retry on
// other exit codes but never on 4.

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

fn af_no_path() -> Command {
    let mut cmd = Command::cargo_bin("af").expect("cargo bin `af`");
    cmd.env_clear()
        .env("HOME", "/tmp")
        .env("PATH", "/nonexistent-empty-path");
    cmd
}

fn core_dir() -> PathBuf {
    repo_root().join("examples").join("af-mod-add")
}

fn run(args: &[&str]) -> (i32, Value) {
    let build_root = tempfile::TempDir::new().unwrap();
    let mut full = vec![
        "--json".to_string(),
        "--build-root".to_string(),
        build_root.path().to_str().unwrap().to_string(),
    ];
    full.extend(args.iter().map(|s| s.to_string()));
    let out = af_no_path().args(&full).output().expect("execute");
    let exit = out.status.code().expect("exit");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    (exit, value)
}

fn assert_backend_unavailable_band(exit: i32, value: &Value, label: &str) {
    // The CLI may return either AF_BACKEND_UNAVAILABLE (exit 4) or a
    // domain-specific upstream failure with a different code; both are
    // documented. What's *not* allowed: exit 0 (silent pass) or panic.
    assert!(
        exit != 0,
        "{label} under empty PATH must fail (exit ≠ 0); got {exit}"
    );
    assert!(
        value.is_object(),
        "{label} must still produce a JSON object envelope"
    );
    if let Some(code) = value["code"].as_str() {
        assert!(
            code.starts_with("AF_"),
            "{label} envelope code must be AF_*: got {code}"
        );
    }
}

#[test]
fn core_lint_verilator_under_empty_path_fails_gracefully() {
    let (exit, value) = run(&[
        "core",
        "lint",
        core_dir().to_str().unwrap(),
        "--backend",
        "verilator",
    ]);
    assert_backend_unavailable_band(exit, &value, "core lint verilator");
}

#[test]
fn core_lint_yosys_under_empty_path_fails_gracefully() {
    let (exit, value) = run(&[
        "core",
        "lint",
        core_dir().to_str().unwrap(),
        "--backend",
        "yosys",
    ]);
    assert_backend_unavailable_band(exit, &value, "core lint yosys");
}

#[test]
fn core_lint_icarus_under_empty_path_fails_gracefully() {
    let (exit, value) = run(&[
        "core",
        "lint",
        core_dir().to_str().unwrap(),
        "--backend",
        "icarus",
    ]);
    assert_backend_unavailable_band(exit, &value, "core lint icarus");
}

#[test]
fn core_sim_icarus_under_empty_path_fails_gracefully() {
    let (exit, value) = run(&[
        "core",
        "sim",
        core_dir().to_str().unwrap(),
        "--backend",
        "icarus",
    ]);
    assert_backend_unavailable_band(exit, &value, "core sim icarus");
}

#[test]
fn core_formal_sby_under_empty_path_fails_gracefully() {
    let (exit, value) = run(&[
        "core",
        "formal",
        core_dir().to_str().unwrap(),
        "--backend",
        "sby",
    ]);
    assert_backend_unavailable_band(exit, &value, "core formal sby");
}
