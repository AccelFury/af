// SPDX-License-Identifier: Apache-2.0
//
// `af tooling check / plan / ensure` integration. Tests the policy
// gates that protect against unintended network/system installs.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

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
fn tooling_check_against_in_repo_manifest_succeeds() {
    let build_root = TempDir::new().unwrap();
    let out = af()
        .current_dir(repo_root())
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "tooling",
            "check",
        ])
        .output()
        .expect("execute");
    assert_eq!(
        out.status.code(),
        Some(0),
        "tooling check must succeed; stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert!(value.is_object());
}

#[test]
fn tooling_plan_with_oss_profile_runs() {
    let build_root = TempDir::new().unwrap();
    let out = af()
        .current_dir(repo_root())
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "tooling",
            "plan",
            "--profile",
            "oss",
        ])
        .output()
        .expect("execute");
    let exit = out.status.code().expect("exit");
    assert!(matches!(exit, 0 | 2));
}

#[test]
fn tooling_ensure_without_yes_and_network_returns_confirmation_required() {
    let build_root = TempDir::new().unwrap();
    let out = af()
        .current_dir(repo_root())
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "tooling",
            "ensure",
            "--tools",
            "yosys",
        ])
        .output()
        .expect("execute");
    // Without --yes / --allow-network, the policy must block.
    let exit = out.status.code().expect("exit");
    if exit == 0 {
        // Some hosts may have yosys already installed; allow that.
        return;
    }
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    let code = value["code"].as_str().expect("code");
    assert!(
        code.starts_with("AF_TOOLING_") || code.starts_with("AF_"),
        "ensure without --yes expected AF_TOOLING_* envelope, got {code}"
    );
}

#[test]
fn tooling_check_with_malformed_manifest_returns_envelope() {
    let tmp = TempDir::new().unwrap();
    let manifest_path = tmp.path().join("af-toolchain.toml");
    fs::write(&manifest_path, b"not = valid toml [[\n").unwrap();
    let build_root = TempDir::new().unwrap();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "tooling",
            "check",
            "--toolchain-manifest",
            manifest_path.to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    assert!(!out.status.success(), "malformed manifest must fail");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    let code = value["code"].as_str().expect("code");
    assert!(code.starts_with("AF_TOOLING_") || code.starts_with("AF_"));
}

#[test]
fn tooling_check_with_missing_manifest_returns_envelope() {
    let tmp = TempDir::new().unwrap();
    let manifest_path = tmp.path().join("no-such.toml");
    let build_root = TempDir::new().unwrap();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "tooling",
            "check",
            "--toolchain-manifest",
            manifest_path.to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    assert!(!out.status.success(), "missing manifest must fail");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert!(value["code"].as_str().unwrap().starts_with("AF_"));
}

#[test]
fn tooling_plan_with_unknown_tool_returns_envelope() {
    let build_root = TempDir::new().unwrap();
    let out = af()
        .current_dir(repo_root())
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "tooling",
            "plan",
            "--tools",
            "totally-fake-toolname",
        ])
        .output()
        .expect("execute");
    let exit = out.status.code().expect("exit");
    if exit != 0 {
        let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
        assert!(value["code"].as_str().unwrap().starts_with("AF_"));
    }
}
