// SPDX-License-Identifier: Apache-2.0
//
// Live envelope-shape tests for AF_PORTABLE_* codes via
// `af core check --json` against af-reset-sync (the in-tree
// `verilog-2001` reference that the portable-policy walker exercises).
//
// The walker writes its issues into the `inspection.issues[]` array of
// the CoreCheckReport, which the CLI propagates through `details`.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
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

fn copy_dir_all(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap().flatten() {
        let p = entry.path();
        let to = dst.join(p.file_name().unwrap());
        if p.is_dir() {
            copy_dir_all(&p, &to);
        } else {
            fs::copy(&p, &to).unwrap();
        }
    }
}

fn clone_reset_sync() -> TempDir {
    let src = repo_root().join("examples").join("af-reset-sync");
    let tmp = TempDir::new().unwrap();
    copy_dir_all(&src, tmp.path());
    tmp
}

/// Inject snippet just before the final `endmodule` of af_reset_sync.v.
fn inject(project: &Path, snippet: &str) {
    let p = project.join("rtl/af_reset_sync.v");
    let text = fs::read_to_string(&p).unwrap();
    let injected = if let Some(idx) = text.rfind("endmodule") {
        let (head, tail) = text.split_at(idx);
        format!("{head}\n{snippet}\n{tail}")
    } else {
        format!("{text}\n{snippet}\n")
    };
    fs::write(&p, injected).unwrap();
}

fn run_core_check(core: &Path) -> (i32, Value) {
    let build_root = TempDir::new().unwrap();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "core",
            "check",
            core.to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    let exit = out.status.code().expect("exit");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    (exit, value)
}

fn portable_codes(value: &Value) -> Vec<String> {
    value
        .get("details")
        .and_then(|d| d.get("inspection"))
        .and_then(|i| i.get("issues"))
        .and_then(|arr| arr.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|i| i["code"].as_str().map(String::from))
                .filter(|c| c.starts_with("AF_PORTABLE_"))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn clean_reset_sync_passes_core_check() {
    let core = clone_reset_sync();
    let (exit, value) = run_core_check(core.path());
    assert_eq!(
        exit,
        0,
        "clean af-reset-sync must pass core check; envelope: {}",
        serde_json::to_string_pretty(&value).unwrap()
    );
}

#[test]
fn ddr_marker_surfaces_in_envelope() {
    let core = clone_reset_sync();
    inject(core.path(), "  wire ddr3_clk; assign ddr3_clk = 1'b0;");
    let (exit, value) = run_core_check(core.path());
    assert_eq!(exit, 2);
    assert_eq!(value["code"].as_str(), Some("AF_CORE_CHECK_FAILED"));
    let codes = portable_codes(&value);
    assert!(
        codes.contains(&"AF_PORTABLE_HARD_PHY_BLOCK".to_string()),
        "envelope must carry AF_PORTABLE_HARD_PHY_BLOCK; got {codes:?}"
    );
}

#[test]
fn vendor_clock_marker_surfaces_in_envelope() {
    let core = clone_reset_sync();
    inject(core.path(), "  wire mmcm_lock;");
    let (exit, value) = run_core_check(core.path());
    assert_eq!(exit, 2);
    let codes = portable_codes(&value);
    assert!(
        codes.contains(&"AF_PORTABLE_VENDOR_OR_CLOCK_MARKER".to_string()),
        "envelope must carry AF_PORTABLE_VENDOR_OR_CLOCK_MARKER; got {codes:?}"
    );
}

#[test]
fn axi_only_marker_surfaces_in_envelope() {
    let core = clone_reset_sync();
    inject(core.path(), "  wire tvalid; wire tready;");
    let (exit, value) = run_core_check(core.path());
    assert_eq!(exit, 2);
    let codes = portable_codes(&value);
    assert!(
        codes.contains(&"AF_PORTABLE_AXI_ONLY_MARKER".to_string()),
        "envelope must carry AF_PORTABLE_AXI_ONLY_MARKER; got {codes:?}"
    );
}

#[test]
fn encrypted_netlist_pragma_surfaces_in_envelope() {
    let core = clone_reset_sync();
    inject(core.path(), "`pragma protect begin_protected");
    let (exit, value) = run_core_check(core.path());
    assert_eq!(exit, 2);
    let codes = portable_codes(&value);
    assert!(
        codes.contains(&"AF_PORTABLE_ENCRYPTED_NETLIST".to_string()),
        "envelope must carry AF_PORTABLE_ENCRYPTED_NETLIST; got {codes:?}"
    );
}
