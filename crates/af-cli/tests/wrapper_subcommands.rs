// SPDX-License-Identifier: Apache-2.0
//
// `af wrapper generate --target <fusesoc|litex|ipxact>` integration.

use assert_cmd::Command;
use serde_json::Value;
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

fn run_wrapper(target: &str, board: Option<&str>) -> (i32, Value, PathBuf) {
    let build_root = TempDir::new().unwrap();
    let br_path = build_root.path().to_path_buf();
    let core = repo_root().join("examples").join("af-mod-add");
    let mut args = vec![
        "--json".to_string(),
        "--build-root".to_string(),
        br_path.to_str().unwrap().to_string(),
        "wrapper".to_string(),
        "generate".to_string(),
        core.to_str().unwrap().to_string(),
        "--target".to_string(),
        target.to_string(),
    ];
    if let Some(b) = board {
        args.push("--board".to_string());
        args.push(b.to_string());
    }
    let out = af().args(&args).output().expect("execute");
    let exit = out.status.code().expect("exit");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    // Leak build_root by forgetting the guard so the caller can inspect
    // artifacts on the filesystem.
    let leak = build_root.keep();
    (exit, value, leak)
}

#[test]
fn wrapper_fusesoc_emits_core_file_under_build_root() {
    let (exit, value, br) = run_wrapper("fusesoc", None);
    assert_eq!(exit, 0);
    assert_eq!(value["status"].as_str(), Some("passed"));
    // <build-root>/fusesoc/<sanitised>.core exists.
    let fusesoc = br.join("fusesoc");
    assert!(fusesoc.is_dir(), "fusesoc dir must exist");
    let entries: Vec<_> = std::fs::read_dir(&fusesoc).unwrap().flatten().collect();
    assert!(
        entries
            .iter()
            .any(|e| e.path().extension().and_then(|x| x.to_str()) == Some("core")),
        "fusesoc/ must contain a .core file"
    );
    let _ = std::fs::remove_dir_all(&br);
}

#[test]
fn wrapper_litex_emits_python_skeleton() {
    let (exit, value, br) = run_wrapper("litex", Some("digilent_arty_a7"));
    assert_eq!(exit, 0);
    assert_eq!(value["status"].as_str(), Some("passed"));
    let litex = br.join("litex");
    assert!(litex.is_dir());
    let py: Vec<_> = std::fs::read_dir(&litex)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("py"))
        .collect();
    assert!(!py.is_empty(), "litex/ must contain a .py skeleton");
    let _ = std::fs::remove_dir_all(&br);
}

#[test]
fn wrapper_ipxact_emits_xml_with_1685_2022_namespace() {
    let (exit, value, br) = run_wrapper("ipxact", None);
    assert_eq!(exit, 0);
    assert_eq!(value["status"].as_str(), Some("passed"));
    let ipxact = br.join("ipxact");
    let xml_files: Vec<_> = std::fs::read_dir(&ipxact)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("xml"))
        .collect();
    assert_eq!(xml_files.len(), 1, "exactly one .xml expected");
    let text = std::fs::read_to_string(xml_files[0].path()).unwrap();
    assert!(text.contains("IPXACT/1685-2022"));
    assert!(text.contains("<ipxact:component"));
    let _ = std::fs::remove_dir_all(&br);
}

#[test]
fn wrapper_unknown_target_emits_envelope() {
    let build_root = TempDir::new().unwrap();
    let core = repo_root().join("examples").join("af-mod-add");
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "wrapper",
            "generate",
            core.to_str().unwrap(),
            "--target",
            "unknown-target",
        ])
        .output()
        .expect("execute");
    assert!(!out.status.success());
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert_eq!(
        value["code"].as_str(),
        Some("AF_WRAPPER_TARGET_UNSUPPORTED")
    );
    assert_eq!(value["exit_code"].as_i64(), Some(2));
}
