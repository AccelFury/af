// SPDX-License-Identifier: Apache-2.0
//
// `af ci render / doctor / validate / improve / add-board / run-local`
// envelope coverage.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn af() -> Command {
    Command::cargo_bin("af").expect("cargo bin `af`")
}

fn run(args: &[&str]) -> (i32, Value) {
    let build_root = TempDir::new().unwrap();
    let mut full = vec![
        "--json",
        "--build-root",
        build_root.path().to_str().unwrap(),
    ];
    full.extend_from_slice(args);
    let out = af().args(&full).output().expect("execute");
    let exit = out.status.code().expect("exit");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    (exit, value)
}

#[test]
fn ci_render_with_missing_config_returns_envelope() {
    let tmp = TempDir::new().unwrap();
    let cfg = tmp.path().join("no-such-config.toml");
    let out_path = tmp.path().join("ci.yml");
    let (exit, value) = run(&[
        "ci",
        "render",
        "--config",
        cfg.to_str().unwrap(),
        "--output",
        out_path.to_str().unwrap(),
        "--dry-run",
    ]);
    assert!(exit != 0, "missing config must fail");
    assert!(value["code"].as_str().unwrap().starts_with("AF_"));
}

#[test]
fn ci_doctor_on_fresh_repo_returns_status() {
    let tmp = TempDir::new().unwrap();
    let (exit, value) = run(&["ci", "doctor", "--repo", tmp.path().to_str().unwrap()]);
    // Doctor on an empty repo may flag missing workflow but should
    // never panic.
    // ci doctor may return exit 3 (logic) when no workflow is present.
    assert!(
        matches!(exit, 0 | 2 | 3),
        "ci doctor exit outside band: {exit}"
    );
    assert!(value.is_object());
}

#[test]
fn ci_validate_on_repo_without_config_returns_envelope() {
    let tmp = TempDir::new().unwrap();
    let (exit, value) = run(&[
        "ci",
        "validate",
        "--repo",
        tmp.path().to_str().unwrap(),
        "--config",
        "af-ci.toml",
    ]);
    if exit != 0 {
        assert!(value["code"].as_str().unwrap().starts_with("AF_"));
    }
}

#[test]
fn ci_improve_on_missing_workflow_returns_envelope() {
    let tmp = TempDir::new().unwrap();
    let workflow = tmp.path().join(".github/workflows/never.yml");
    let (exit, value) = run(&[
        "ci",
        "improve",
        "--repo",
        tmp.path().to_str().unwrap(),
        "--workflow",
        workflow.to_str().unwrap(),
        "--dry-run",
    ]);
    if exit != 0 {
        assert!(value["code"].as_str().unwrap().starts_with("AF_"));
    }
}

#[test]
fn ci_add_board_with_minimal_args_runs() {
    let tmp = TempDir::new().unwrap();
    let constraints = tmp.path().join("constraints.xdc");
    fs::write(&constraints, b"# placeholder\n").unwrap();
    let (exit, value) = run(&[
        "ci",
        "add-board",
        "--repo",
        tmp.path().to_str().unwrap(),
        "--name",
        "test-board",
        "--family",
        "ice40",
        "--top",
        "top",
        "--device",
        "ice40lp8k-cm225",
        "--constraints",
        constraints.to_str().unwrap(),
        "--dry-run",
    ]);
    // Either succeeds or returns a documented envelope. Crucially:
    // must not panic. Exit 3 happens when target workflow/config is
    // missing in the fresh tempdir.
    assert!(
        matches!(exit, 0 | 2 | 3),
        "ci add-board exit outside band: {exit}"
    );
    let _ = value;
}

#[test]
fn ci_run_local_with_missing_profile_returns_envelope() {
    let tmp = TempDir::new().unwrap();
    let (exit, value) = run(&[
        "ci",
        "run-local",
        "--repo",
        tmp.path().to_str().unwrap(),
        "--profile",
        "totally-fake-profile",
        "--dry-run",
    ]);
    if exit != 0 {
        assert!(value["code"].as_str().unwrap().starts_with("AF_"));
    }
}
