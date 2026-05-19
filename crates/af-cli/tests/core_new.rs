// SPDX-License-Identifier: Apache-2.0
//
// `af core new` scaffolds a new core directory. Validates --class,
// --language enum values and refuses to overwrite existing files.

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn af() -> Command {
    Command::cargo_bin("af").expect("cargo bin `af`")
}

fn new_args(dir: &std::path::Path, name: &str, class: &str, language: &str) -> Vec<String> {
    vec![
        "--json".to_string(),
        "core".to_string(),
        "new".to_string(),
        dir.to_str().unwrap().to_string(),
        "--name".to_string(),
        name.to_string(),
        "--class".to_string(),
        class.to_string(),
        "--language".to_string(),
        language.to_string(),
    ]
}

#[test]
fn core_new_creates_manifest_and_rtl_tree() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("af-test-core");
    let args = new_args(&dir, "af-test-core", "simple-portable", "verilog-2001");
    let out = af().args(&args).output().expect("execute");
    assert!(
        out.status.success(),
        "core new must succeed; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert_eq!(value["status"].as_str(), Some("passed"));
    assert!(dir.join("af-core.toml").is_file());
}

#[test]
fn core_new_with_unknown_class_returns_envelope() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("c");
    let args = new_args(&dir, "c", "platinum-elite-class", "verilog-2001");
    let out = af().args(&args).output().expect("execute");
    assert!(!out.status.success(), "unknown --class must fail");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    let code = value["code"].as_str().expect("code");
    assert!(code.starts_with("AF_"));
}

#[test]
fn core_new_with_unknown_language_returns_envelope() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("c");
    let args = new_args(&dir, "c", "simple-portable", "esperanto-hdl");
    let out = af().args(&args).output().expect("execute");
    assert!(!out.status.success(), "unknown --language must fail");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert!(value["code"].as_str().unwrap().starts_with("AF_"));
}

#[test]
fn core_new_refuses_to_overwrite_existing_manifest() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("c");
    let args = new_args(&dir, "c", "simple-portable", "verilog-2001");
    let _ = af().args(&args).output().expect("first run");
    assert!(dir.join("af-core.toml").is_file());
    let out = af().args(&args).output().expect("second run");
    assert!(!out.status.success(), "second create must refuse overwrite");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert!(value["code"].as_str().unwrap().starts_with("AF_"));
}

#[test]
fn core_new_in_nested_path_creates_parent_dirs() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("nested/deep/path/core");
    let args = new_args(&dir, "core", "simple-portable", "verilog-2001");
    let out = af().args(&args).output().expect("execute");
    assert!(
        out.status.success(),
        "nested path scaffold must succeed; stdout:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(dir.join("af-core.toml").is_file());
}
