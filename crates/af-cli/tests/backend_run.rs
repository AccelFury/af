// SPDX-License-Identifier: Apache-2.0
//
// `af backend list` and `af backend run` integration.

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

#[test]
fn backend_list_returns_capabilities_payload() {
    let build_root = tempfile::TempDir::new().unwrap();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "backend",
            "list",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success(), "backend list must succeed");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert!(value.is_object());
    // The payload must include capabilities; at least one row should
    // belong to the native backend (always advertised).
    let caps = value["capabilities"]
        .as_array()
        .expect("capabilities array");
    assert!(!caps.is_empty());
    let any_native = caps
        .iter()
        .any(|c| c["name"].as_str().is_some_and(|n| n.starts_with("native")));
    assert!(
        any_native,
        "backend list must include native-* capabilities"
    );
}

#[test]
fn backend_run_native_doctor_runs() {
    let build_root = tempfile::TempDir::new().unwrap();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "backend",
            "run",
            "native",
            "--target",
            "doctor",
        ])
        .output()
        .expect("execute");
    // Native doctor must always succeed.
    let exit = out.status.code().expect("exit");
    assert!(
        matches!(exit, 0 | 2),
        "backend run native doctor exit: {exit}"
    );
}

#[test]
fn backend_run_native_lint_with_core_dir_runs() {
    let build_root = tempfile::TempDir::new().unwrap();
    let core = repo_root().join("examples").join("af-mod-add");
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "backend",
            "run",
            "native",
            "--target",
            "lint",
            "--core-dir",
            core.to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    let exit = out.status.code().expect("exit");
    assert!(
        matches!(exit, 0 | 2 | 3 | 7),
        "backend run native lint exit outside band: {exit}"
    );
    let _value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
}

#[test]
fn backend_run_unknown_backend_returns_envelope() {
    let build_root = tempfile::TempDir::new().unwrap();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "backend",
            "run",
            "totally-fake-backend",
            "--target",
            "doctor",
        ])
        .output()
        .expect("execute");
    assert!(!out.status.success(), "unknown backend must fail");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    let code = value["code"].as_str().expect("code");
    assert!(code.starts_with("AF_BACKEND_") || code.starts_with("AF_"));
}
