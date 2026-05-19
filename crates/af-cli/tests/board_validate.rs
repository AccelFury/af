// SPDX-License-Identifier: Apache-2.0
//
// `af board check` integration. The clap subcommand is named `check`
// (not `validate`); we exercise it on the in-tree board registry and
// on synthetic malformed fixtures.

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
fn board_list_against_in_repo_data_returns_known_boards() {
    let build_root = TempDir::new().unwrap();
    let out = af()
        .current_dir(repo_root())
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "board",
            "list",
        ])
        .output()
        .expect("execute");
    assert!(
        out.status.success(),
        "board list must succeed; stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    let boards = value["boards"]
        .as_array()
        .or_else(|| value.as_array())
        .or_else(|| value["command_payload"]["boards"].as_array())
        .expect("board list payload");
    assert!(!boards.is_empty(), "board list must return ≥1 entry");
}

#[test]
fn board_check_on_existing_profile_passes() {
    let build_root = TempDir::new().unwrap();
    // Pick a known board profile path.
    let candidates = [
        "boards/gowin/sipeed_tang_nano_20k/board.toml",
        "boards/lattice/orangecrab_ecp5/board.toml",
    ];
    let profile = candidates
        .iter()
        .map(|p| repo_root().join(p))
        .find(|p| p.is_file());
    let Some(profile) = profile else {
        eprintln!("no board.toml fixture found; skipping");
        return;
    };
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "board",
            "check",
            profile.to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    // Status may be passed or partial depending on the profile's
    // declared evidence; what we assert is a documented exit band.
    let exit = out.status.code().expect("exit");
    assert!(
        matches!(exit, 0 | 2),
        "board check exit code outside band: {exit}"
    );
    let _value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
}

#[test]
fn board_check_on_malformed_profile_emits_envelope() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("broken.toml");
    fs::write(&path, b"this is not = valid toml [[\n").unwrap();
    let build_root = TempDir::new().unwrap();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "board",
            "check",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    assert!(!out.status.success());
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    let code = value["code"].as_str().expect("code");
    assert!(
        code.starts_with("AF_BOARD_") || code.starts_with("AF_MANIFEST_"),
        "malformed board profile expected AF_BOARD_* or AF_MANIFEST_*, got {code}"
    );
}
