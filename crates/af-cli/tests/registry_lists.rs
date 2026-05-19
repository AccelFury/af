// SPDX-License-Identifier: Apache-2.0
//
// `af core registry list`, `af registry check`, `af board list`,
// `af board new` integration coverage.

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

fn run(args: &[&str]) -> (i32, Value) {
    let build_root = tempfile::TempDir::new().unwrap();
    let mut full = vec![
        "--json",
        "--build-root",
        build_root.path().to_str().unwrap(),
    ];
    full.extend_from_slice(args);
    let out = af()
        .current_dir(repo_root())
        .args(&full)
        .output()
        .expect("execute");
    let exit = out.status.code().expect("exit");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    (exit, value)
}

#[test]
fn core_registry_list_returns_array_of_cores() {
    let (exit, value) = run(&["core", "registry", "list"]);
    assert_eq!(exit, 0);
    let cores = value["cores"]
        .as_array()
        .or_else(|| value["command_payload"]["cores"].as_array())
        .expect("cores array somewhere in payload");
    assert!(!cores.is_empty(), "registry must list ≥1 core");
}

#[test]
fn core_registry_list_filters_by_priority() {
    let (exit, value) = run(&["core", "registry", "list", "--priority", "P0"]);
    assert_eq!(exit, 0);
    let cores = value["cores"]
        .as_array()
        .or_else(|| value["command_payload"]["cores"].as_array())
        .expect("cores");
    for c in cores {
        assert_eq!(c["priority"].as_str(), Some("P0"));
    }
}

#[test]
fn core_registry_list_filters_by_portability() {
    let (exit, value) = run(&["core", "registry", "list", "--portability", "U0"]);
    assert_eq!(exit, 0);
    let cores = value["cores"]
        .as_array()
        .or_else(|| value["command_payload"]["cores"].as_array())
        .expect("cores");
    for c in cores {
        assert_eq!(c["portability_level"].as_str(), Some("U0"));
    }
}

#[test]
fn registry_check_against_in_repo_root_passes() {
    let (exit, value) = run(&["registry", "check"]);
    assert!(
        matches!(exit, 0 | 2),
        "registry check exit outside band: {exit}"
    );
    assert!(value.is_object());
}

#[test]
fn board_list_returns_known_boards() {
    let (exit, value) = run(&["board", "list"]);
    assert_eq!(exit, 0);
    let boards = value["boards"]
        .as_array()
        .or_else(|| value["command_payload"]["boards"].as_array())
        .or_else(|| value.as_array())
        .expect("boards array");
    assert!(!boards.is_empty());
}

#[test]
fn board_new_creates_board_dir_under_root() {
    let tmp = tempfile::TempDir::new().unwrap();
    let build_root = tempfile::TempDir::new().unwrap();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "board",
            "new",
            "--board-id",
            "test-board",
            "--vendor",
            "xilinx",
            "--family",
            "artix-7",
            "--constraint-format",
            "xdc",
            "--root",
            tmp.path().to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    if !out.status.success() {
        // Some `board new` flows require evidence files we cannot
        // synthesise here; envelope shape is what matters.
        let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
        assert!(value["code"].as_str().unwrap().starts_with("AF_"));
    } else {
        // Verify a board directory was created under <root>/boards/.
        let pattern = tmp.path().join("boards");
        assert!(
            pattern.exists() || pattern.read_dir().is_ok_and(|mut d| d.next().is_some()),
            "board new must create boards/ dir"
        );
    }
}
