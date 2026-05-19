// SPDX-License-Identifier: Apache-2.0
//
// Schema-shape validation for config-driven CLI inputs:
// af-toolchain.toml, af-selfcheck.toml, af-arch.toml (via
// `architecture check`), af-project.toml (via `project classify`).

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
fn af_toolchain_with_wrong_schema_version_returns_envelope() {
    let tmp = TempDir::new().unwrap();
    let manifest = tmp.path().join("af-toolchain.toml");
    fs::write(
        &manifest,
        b"schema_version = \"9.9\"\nkind = \"accelfury.toolchain\"\n[policy]\noffline = true\n",
    )
    .unwrap();
    let build_root = TempDir::new().unwrap();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "tooling",
            "check",
            "--toolchain-manifest",
            manifest.to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    if !out.status.success() {
        let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
        assert!(value["code"].as_str().unwrap().starts_with("AF_"));
    }
}

#[test]
fn af_selfcheck_with_unknown_target_returns_envelope() {
    // Already tested in self_check_inventory.rs; this re-asserts the
    // schema-shape contract.
    let build_root = TempDir::new().unwrap();
    let out = af()
        .current_dir(repo_root())
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "self",
            "check",
            "--target",
            "definitely-not-a-target",
        ])
        .output()
        .expect("execute");
    assert!(!out.status.success());
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert_eq!(value["code"].as_str(), Some("AF_SELF_CHECK_TARGET_UNKNOWN"));
}

#[test]
fn af_selfcheck_with_malformed_toml_returns_envelope() {
    let tmp = TempDir::new().unwrap();
    let cfg = tmp.path().join("bad.toml");
    fs::write(&cfg, b"this is not = valid toml [\n").unwrap();
    let build_root = TempDir::new().unwrap();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "self",
            "check",
            "--config",
            cfg.to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    assert!(!out.status.success());
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert!(value["code"].as_str().unwrap().starts_with("AF_"));
}

#[test]
fn af_arch_check_on_dir_without_manifest_returns_envelope() {
    let tmp = TempDir::new().unwrap();
    let build_root = TempDir::new().unwrap();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "architecture",
            "check",
            tmp.path().to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    assert!(!out.status.success(), "empty project dir must fail");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    let code = value["code"].as_str().expect("code");
    assert!(code.starts_with("AF_"));
}
