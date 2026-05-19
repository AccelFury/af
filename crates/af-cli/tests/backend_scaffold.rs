// SPDX-License-Identifier: Apache-2.0
//
// `af backend scaffold <core> --vendor <v> --family <f>` integration.
// Creates `<core>/vendor/<v>/` with required subdirs and the
// vendor-appropriate constraint file (.xdc/.sdc/.cst/.lpf).

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

fn clone_mod_add() -> TempDir {
    let src = repo_root().join("examples").join("af-mod-add");
    let tmp = TempDir::new().unwrap();
    copy_dir_all(&src, tmp.path());
    tmp
}

fn run_scaffold(vendor: &str, family: &str) -> (TempDir, i32, Value) {
    let core = clone_mod_add();
    let build_root = TempDir::new().unwrap();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "backend",
            "scaffold",
            core.path().to_str().unwrap(),
            "--vendor",
            vendor,
            "--family",
            family,
        ])
        .output()
        .expect("execute");
    let exit = out.status.code().expect("exit");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    (core, exit, value)
}

#[test]
fn scaffold_xilinx_creates_xdc_constraint_file() {
    let (core, exit, value) = run_scaffold("xilinx", "artix-7");
    assert_eq!(exit, 0, "scaffold must succeed; payload: {value}");
    assert!(core
        .path()
        .join("vendor/xilinx/constraints/constraints.xdc")
        .is_file());
}

#[test]
fn scaffold_intel_creates_sdc_constraint_file() {
    let (core, exit, _) = run_scaffold("intel", "cyclone-iv");
    assert_eq!(exit, 0);
    assert!(core
        .path()
        .join("vendor/intel/constraints/constraints.sdc")
        .is_file());
}

#[test]
fn scaffold_gowin_creates_cst_constraint_file() {
    let (core, exit, _) = run_scaffold("gowin", "gw1n");
    assert_eq!(exit, 0);
    assert!(core
        .path()
        .join("vendor/gowin/constraints/constraints.cst")
        .is_file());
}

#[test]
fn scaffold_lattice_creates_lpf_constraint_file() {
    let (core, exit, _) = run_scaffold("lattice", "ecp5");
    assert_eq!(exit, 0);
    assert!(core
        .path()
        .join("vendor/lattice/constraints/constraints.lpf")
        .is_file());
}

#[test]
fn scaffold_creates_all_documented_subdirs() {
    let (core, exit, _) = run_scaffold("xilinx", "artix-7");
    assert_eq!(exit, 0);
    for sub in ["ram", "fifo", "dsp", "clock", "constraints", "tests"] {
        assert!(
            core.path().join("vendor/xilinx").join(sub).is_dir(),
            "vendor/xilinx/{sub} must exist"
        );
    }
}
