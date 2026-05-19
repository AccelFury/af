// SPDX-License-Identifier: Apache-2.0
//
// Stable exit-code contract.
//
// `docs/cli-reference.md` documents the following non-zero exit codes:
//
//   0  success
//   2  validation
//   3  RTL or backend logic
//   4  backend unavailable (host tool not installed)
//   6  simulation
//   7  lint
//   8  formal
//   9  build
//  10  flash
//  11  security
//  12  artifact missing
//
// This test pins exit codes for paths we can reach reliably without any
// optional toolchain. Reusing a code for an unrelated failure is a public
// contract regression — this file exists to make that regression visible
// in CI.

use assert_cmd::Command;
use std::fs;

fn af() -> Command {
    Command::cargo_bin("af").expect("cargo bin `af` builds")
}

#[test]
fn doctor_returns_0_on_healthy_host() {
    // doctor never fails the process; it reports per-tool status.
    let out = af().args(["--json", "doctor"]).output().expect("execute");
    assert_eq!(out.status.code(), Some(0), "doctor must always exit 0");
}

#[test]
fn manifest_validate_missing_file_uses_validation_exit_code() {
    let tmp = tempfile::TempDir::new().unwrap();
    let bogus = tmp.path().join("does-not-exist.toml");
    let out = af()
        .args(["--json", "manifest", "validate"])
        .arg(&bogus)
        .output()
        .expect("execute");
    let exit = out.status.code().expect("process did not return code");
    // Manifest read errors map to validation domain (exit 2) per cli-reference.
    assert!(
        exit == 2,
        "expected exit code 2 (validation) for missing manifest, got {exit}\nstdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn manifest_validate_malformed_toml_uses_validation_exit_code() {
    let tmp = tempfile::TempDir::new().unwrap();
    let p = tmp.path().join("af-core.toml");
    fs::write(&p, "[invalid syntax\n").unwrap();
    let out = af()
        .args(["--json", "manifest", "validate"])
        .arg(&p)
        .output()
        .expect("execute");
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn manifest_validate_existing_v03_returns_zero() {
    // examples/af-mod-add/af-core.toml is a v0.3 reference manifest.
    let manifest = repo_root()
        .join("examples")
        .join("af-mod-add")
        .join("af-core.toml");
    assert!(
        manifest.exists(),
        "reference manifest absent: {}",
        manifest.display()
    );
    let out = af()
        .args(["--json", "manifest", "validate"])
        .arg(&manifest)
        .output()
        .expect("execute");
    assert_eq!(
        out.status.code(),
        Some(0),
        "reference manifest must validate; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn clap_argv_error_uses_exit_code_2() {
    let out = af()
        .args(["--unknown-global-flag", "doctor"])
        .output()
        .expect("execute");
    assert_eq!(out.status.code(), Some(2));
}

// `af report <missing>` returns 0 with `warnings[]` populated; that is the
// documented behaviour (artefact discovery may legitimately produce a
// near-empty report). Not exercised here — see `errors_contract.rs`.

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}
