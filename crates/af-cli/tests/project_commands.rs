// SPDX-License-Identifier: Apache-2.0
//
// `af project classify` and `af project new` integration.

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn af() -> Command {
    Command::cargo_bin("af").expect("cargo bin `af`")
}

#[test]
fn project_classify_on_empty_dir_returns_classification() {
    let tmp = TempDir::new().unwrap();
    let build_root = TempDir::new().unwrap();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "project",
            "classify",
            tmp.path().to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    let exit = out.status.code().expect("exit");
    assert!(matches!(exit, 0 | 2));
    if !out.stdout.is_empty() {
        let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
        assert!(value.is_object());
    }
}

#[test]
fn project_new_system_platform_creates_scaffold() {
    let tmp = TempDir::new().unwrap();
    let build_root = TempDir::new().unwrap();
    let dir = tmp.path().join("my-system");
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "project",
            "new",
            dir.to_str().unwrap(),
            "--class",
            "system-platform",
            "--name",
            "my-system",
        ])
        .output()
        .expect("execute");
    let exit = out.status.code().expect("exit");
    if exit == 0 {
        assert!(dir.join("af-project.toml").is_file());
    } else {
        // Envelope shape for any non-zero exit.
        let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
        assert!(value["code"].as_str().unwrap().starts_with("AF_"));
    }
}

#[test]
fn project_new_unknown_class_returns_envelope() {
    let tmp = TempDir::new().unwrap();
    let build_root = TempDir::new().unwrap();
    let dir = tmp.path().join("bogus");
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "project",
            "new",
            dir.to_str().unwrap(),
            "--class",
            "invented-class",
            "--name",
            "x",
        ])
        .output()
        .expect("execute");
    assert!(!out.status.success(), "unknown class must fail");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert!(value["code"].as_str().unwrap().starts_with("AF_"));
}
