// SPDX-License-Identifier: Apache-2.0
//
// Pins the JSON envelope shape for success and error outputs.
//
// Success path: `--json` writes a JSON value to stdout (object or array,
//   depending on subcommand). It must be well-formed JSON.
//
// Error path: `--json` writes an `ErrorPayload` object with fields
//   {code, message, hint, exit_code, details?}; `code` matches `AF_*`;
//   `exit_code` is the same integer as the process exit code.

use assert_cmd::Command;
use serde_json::Value;

fn af() -> Command {
    Command::cargo_bin("af").expect("cargo bin `af` builds")
}

#[test]
fn doctor_json_payload_is_object_with_documented_keys() {
    let out = af().args(["--json", "doctor"]).output().expect("execute");
    assert!(out.status.success(), "doctor must succeed");
    let payload: Value = serde_json::from_slice(&out.stdout).expect("doctor stdout must be JSON");
    let obj = payload.as_object().expect("doctor returns an object");
    // Schema contract — these keys are present in current builds; the
    // test gates against unintended renames.
    for required in ["status", "tool_versions", "schema_version", "kind"] {
        assert!(
            obj.contains_key(required),
            "doctor JSON missing required key {required}"
        );
    }
    assert_eq!(obj["kind"].as_str(), Some("accelfury.report"));
    assert_eq!(obj["schema_version"].as_str(), Some("0.1"));
}

#[test]
fn error_envelope_has_required_keys() {
    // Trigger any deterministic error path (missing manifest).
    let tmp = tempfile::TempDir::new().unwrap();
    let bogus = tmp.path().join("nope.toml");
    let out = af()
        .args(["--json", "manifest", "validate"])
        .arg(&bogus)
        .output()
        .expect("execute");
    assert!(!out.status.success());
    let payload: Value =
        serde_json::from_slice(&out.stdout).expect("--json error must produce JSON");
    let obj = payload.as_object().expect("error payload is object");

    for required in ["code", "message", "hint", "exit_code"] {
        assert!(
            obj.contains_key(required),
            "error envelope missing required key {required}: {payload}"
        );
    }
    let code = obj["code"].as_str().expect("code is string");
    assert!(code.starts_with("AF_"), "code must start with AF_: {code}");
    let msg = obj["message"].as_str().expect("message is string");
    assert!(!msg.is_empty());
    let hint = obj["hint"].as_str().expect("hint is string");
    assert!(!hint.is_empty());
    let exit_code = obj["exit_code"].as_i64().expect("exit_code is integer");
    assert_eq!(
        exit_code as i32,
        out.status.code().expect("process exit code"),
        "envelope exit_code must match process exit code"
    );
    // details is optional and may be absent.
}

#[test]
fn error_envelope_is_single_object_not_wrapped() {
    let tmp = tempfile::TempDir::new().unwrap();
    let bogus = tmp.path().join("nope.toml");
    let out = af()
        .args(["--json", "manifest", "validate"])
        .arg(&bogus)
        .output()
        .expect("execute");
    let stdout = String::from_utf8(out.stdout).unwrap();
    // First non-whitespace char must be `{` — confirms we don't emit
    // an array or a `{ "error": {...} }` wrapper. Stable contract.
    let trimmed = stdout.trim_start();
    assert!(
        trimmed.starts_with('{'),
        "error envelope must be a top-level JSON object, got:\n{stdout}"
    );
}

// AfReport.schema_version pinning is enforced separately via
// `crates/af-report/tests/schema_snapshot.rs` (schemars snapshot).
