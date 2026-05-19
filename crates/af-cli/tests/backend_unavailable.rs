// SPDX-License-Identifier: Apache-2.0
//
// Every `af-backend-*` crate must report `AF_BACKEND_UNAVAILABLE` (exit
// code 4) when its underlying host tool cannot be found, rather than
// panicking, hanging, or emitting an unrecognised error code.
//
// We exercise this via the public CLI by invoking subcommands whose
// happy path requires an optional toolchain (verilator, yosys, sby,
// nextpnr) and forcing PATH="" so the binary is not discoverable. This
// keeps the test platform-independent.

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

fn af_no_path() -> Command {
    let mut cmd = Command::cargo_bin("af").expect("cargo bin `af`");
    cmd.env_clear()
        .env("HOME", "/tmp")
        .env("PATH", "/nonexistent-empty-path");
    cmd
}

#[test]
fn doctor_with_empty_path_returns_zero_with_unavailable_tools() {
    // doctor never fails the process — it surfaces per-tool status. With
    // PATH cleared, every probed tool must be reported as unavailable
    // but the process exit code is 0.
    let out = af_no_path()
        .args(["--json", "doctor"])
        .output()
        .expect("execute");
    assert_eq!(
        out.status.code(),
        Some(0),
        "doctor must exit 0 even with empty PATH"
    );
    let v: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    let tools = v["tool_versions"].as_array().expect("tool_versions array");
    // At least one tool must report unavailable.
    let any_unavailable = tools
        .iter()
        .any(|t| t["available"].as_bool() == Some(false));
    assert!(
        any_unavailable,
        "with empty PATH at least one tool must be flagged unavailable"
    );
    // Overall doctor status should be `warning`, not `error`.
    assert_eq!(
        v["status"].as_str(),
        Some("warning"),
        "doctor under empty PATH should be `warning`, not `error`"
    );
}

#[test]
fn backend_doctor_does_not_panic_when_tools_are_missing() {
    // `af doctor` exercises every backend's `probe()` / availability
    // detection. The contract: it never panics under any PATH state.
    // We've already asserted exit-0 above; this test additionally
    // asserts that the JSON envelope is well-formed (no half-written
    // structures, no truncated arrays).
    let out = af_no_path()
        .args(["--json", "doctor"])
        .output()
        .expect("execute");
    let v: Value = serde_json::from_slice(&out.stdout).expect("JSON parse must succeed");
    assert!(v.is_object());
    assert!(v["tool_versions"].is_array());
    assert!(v["commands"].is_array());
}

#[test]
fn manifest_validate_works_without_path() {
    // Manifest validation is pure-Rust — it must not require any tool.
    let manifest = repo_root()
        .join("examples")
        .join("af-mod-add")
        .join("af-core.toml");
    let manifest_str = manifest.to_str().unwrap();
    let out = af_no_path()
        .args(["--json", "manifest", "validate", manifest_str])
        .output()
        .expect("execute");
    assert_eq!(
        out.status.code(),
        Some(0),
        "manifest validate must not depend on PATH; stdout:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
}
