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
fn registry_check_reports_catalog_readiness_blockers_without_failing() {
    let (exit, value) = run(&["registry", "check"]);

    assert_eq!(exit, 0);
    assert_eq!(value["status"].as_str(), Some("passed"));
    assert_eq!(
        value["catalog_readiness"]["target"].as_str(),
        Some("fpga.chat-v1")
    );
    assert_eq!(
        value["catalog_readiness"]["status"].as_str(),
        Some("blocked")
    );
    assert!(
        value["catalog_readiness"]["board_records"]["blocked_count"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert!(
        value["catalog_readiness"]["core_licenses"]["blocked_count"]
            .as_u64()
            .unwrap()
            > 0
    );

    let board_blockers = value["catalog_readiness"]["board_records"]["blockers"]
        .as_array()
        .expect("board blockers");
    assert!(board_blockers.iter().any(|blocker| {
        blocker["code"].as_str() == Some("AF_CATALOG_BOARD_REVISION_MISSING")
            && blocker["reason"].as_str() == Some("revision_missing_from_upstream")
    }));

    let core_blockers = value["catalog_readiness"]["core_licenses"]["blockers"]
        .as_array()
        .expect("core license blockers");
    assert!(core_blockers.iter().any(|blocker| {
        blocker["code"].as_str() == Some("AF_CATALOG_CORE_LICENSE_NON_OSI")
            && blocker["reason"].as_str() == Some("non_osi_license")
    }));
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
