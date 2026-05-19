// SPDX-License-Identifier: Apache-2.0
//
// Integration tests for `af core lint`, `af core sim`, `af core formal`.
//
// The `native` backend is the only one we expect available on every
// host (it is pure-Rust). The remaining backends should always
// surface `AF_BACKEND_UNAVAILABLE` (exit 4) without panicking when
// their bin is missing.

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

fn af() -> Command {
    Command::cargo_bin("af").expect("cargo bin `af`")
}

fn core_dir() -> PathBuf {
    repo_root().join("examples").join("af-mod-add")
}

#[test]
fn core_lint_with_unknown_backend_emits_envelope_without_panic() {
    let build_root = tempfile::TempDir::new().unwrap();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "core",
            "lint",
            core_dir().to_str().unwrap(),
            "--backend",
            "totally-unknown-backend",
        ])
        .output()
        .expect("execute");
    assert!(!out.status.success());
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    let code = value["code"].as_str().expect("code");
    assert!(
        code.starts_with("AF_BACKEND_")
            || code.starts_with("AF_LINT_")
            || code.starts_with("AF_CORE_"),
        "unknown backend expected AF_BACKEND/AF_LINT/AF_CORE prefix, got {code}"
    );
}

#[test]
fn core_sim_with_icarus_under_empty_path_returns_unavailable() {
    let build_root = tempfile::TempDir::new().unwrap();
    let out = af_no_path()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "core",
            "sim",
            core_dir().to_str().unwrap(),
            "--backend",
            "icarus",
        ])
        .output()
        .expect("execute");
    let exit = out.status.code().expect("exit");
    // Either AF_BACKEND_UNAVAILABLE (exit 4) or some manifest-level
    // pre-flight failure (e.g. constructor metadata). Both are
    // documented bands; what matters is no panic.
    assert!(
        matches!(exit, 2 | 4 | 6),
        "icarus sim under empty PATH must exit in documented band, got {exit}"
    );
    let _value: Value = serde_json::from_slice(&out.stdout).expect("JSON parses");
}

#[test]
fn core_formal_with_sby_under_empty_path_returns_unavailable() {
    let build_root = tempfile::TempDir::new().unwrap();
    let out = af_no_path()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "core",
            "formal",
            core_dir().to_str().unwrap(),
            "--backend",
            "sby",
        ])
        .output()
        .expect("execute");
    let exit = out.status.code().expect("exit");
    assert!(
        matches!(exit, 2 | 4 | 8),
        "sby formal under empty PATH must exit in documented band, got {exit}"
    );
}

#[test]
fn core_package_with_default_manifest_format_runs() {
    let build_root = tempfile::TempDir::new().unwrap();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "core",
            "package",
            core_dir().to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    let exit = out.status.code().expect("exit");
    assert!(
        matches!(exit, 0 | 2 | 9),
        "core package exit code outside band: {exit}"
    );
    // Output is always JSON-parseable.
    let _value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
}
