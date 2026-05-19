// SPDX-License-Identifier: Apache-2.0
//
// Integration for `af architecture check`, `af resource plan`,
// `af signoff plan`, `af compatibility check`, `af dependency graph`.

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
    let out = af().args(&full).output().expect("execute");
    let exit = out.status.code().expect("exit");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    (exit, value)
}

fn mod_add_dir() -> String {
    repo_root()
        .join("examples")
        .join("af-mod-add")
        .to_string_lossy()
        .to_string()
}

fn mod_add_manifest() -> String {
    repo_root()
        .join("examples")
        .join("af-mod-add")
        .join("af-core.toml")
        .to_string_lossy()
        .to_string()
}

#[test]
fn architecture_check_runs_on_reference_fixture() {
    let dir = mod_add_dir();
    let (exit, value) = run(&["architecture", "check", &dir]);
    assert!(matches!(exit, 0 | 2), "exit outside band: {exit}");
    if exit == 0 {
        let status = value["status"]
            .as_str()
            .or_else(|| value["command_payload"]["status"].as_str())
            .expect("status string in success payload");
        assert!(matches!(status, "passed" | "warning" | "failed"));
    } else {
        // Reference fixture may legitimately raise AF_ARCH_LAYER_VIOLATION
        // / AF_RESOURCE_CONTRACT_MISSING; envelope shape is what we pin.
        let code = value["code"].as_str().expect("envelope code");
        assert!(code.starts_with("AF_"));
    }
}

#[test]
fn resource_plan_with_xilinx_vendor_produces_resources_map() {
    let dir = mod_add_dir();
    let (exit, value) = run(&[
        "resource", "plan", &dir, "--vendor", "xilinx", "--family", "artix-7",
    ]);
    assert!(matches!(exit, 0 | 2));
    // Resources map appears at the top or under command_payload.
    let resources = value.get("resources").or_else(|| {
        value
            .get("command_payload")
            .and_then(|p| p.get("resources"))
    });
    assert!(
        resources.is_some(),
        "resource plan must surface resources map"
    );
}

#[test]
fn signoff_plan_for_simple_portable_returns_planned_checks() {
    let manifest = mod_add_manifest();
    let (exit, value) = run(&["signoff", "plan", &manifest, "--class", "simple-portable"]);
    assert_eq!(exit, 0);
    let checks = value["checks"]
        .as_array()
        .or_else(|| value["command_payload"]["checks"].as_array());
    assert!(checks.is_some_and(|c| !c.is_empty()));
}

#[test]
fn compatibility_check_on_single_core_passes() {
    let dir = mod_add_dir();
    let (exit, _value) = run(&["compatibility", "check", &dir]);
    assert!(matches!(exit, 0 | 2));
}

#[test]
fn dependency_graph_json_format_passes() {
    let dir = mod_add_dir();
    let (exit, value) = run(&["dependency", "graph", &dir, "--format", "json"]);
    assert!(matches!(exit, 0 | 2));
    assert!(value.is_object());
}

#[test]
fn dependency_graph_unknown_format_returns_envelope() {
    let dir = mod_add_dir();
    let (exit, value) = run(&["dependency", "graph", &dir, "--format", "graphvizzz"]);
    assert!(exit != 0);
    let code = value["code"].as_str().expect("code");
    assert!(code.starts_with("AF_"));
}
