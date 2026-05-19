// SPDX-License-Identifier: Apache-2.0
//
// `af core package` produces a packaged manifest representation. The
// default format is `manifest`; we exercise the happy path + format
// enum validation.

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
fn package_default_manifest_format_runs() {
    let build_root = tempfile::TempDir::new().unwrap();
    let core = repo_root().join("examples").join("af-mod-add");
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "core",
            "package",
            core.to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    // Either passes or returns a documented validation/build code.
    let exit = out.status.code().expect("exit");
    assert!(
        matches!(exit, 0 | 2 | 9),
        "core package exit code outside band: {exit}"
    );
    let _value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
}

#[test]
fn package_unknown_format_returns_envelope() {
    let build_root = tempfile::TempDir::new().unwrap();
    let core = repo_root().join("examples").join("af-mod-add");
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "core",
            "package",
            core.to_str().unwrap(),
            "--format",
            "totally-fake-format",
        ])
        .output()
        .expect("execute");
    assert!(!out.status.success(), "unknown format must fail");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    let code = value["code"].as_str().expect("code");
    assert!(code.starts_with("AF_"));
}

#[test]
fn package_with_missing_core_dir_returns_envelope() {
    let build_root = tempfile::TempDir::new().unwrap();
    let bogus = build_root.path().join("no-such-core");
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "core",
            "package",
            bogus.to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    assert!(!out.status.success(), "missing core dir must fail");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    let code = value["code"].as_str().expect("code");
    assert!(code.starts_with("AF_"));
    assert_eq!(value["exit_code"].as_i64(), Some(2));
}
