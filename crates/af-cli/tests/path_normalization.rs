// SPDX-License-Identifier: Apache-2.0
//
// JSON outputs must use forward-slash path separators regardless of
// platform. On Linux this is the system default; on Windows the
// invariant catches PathBuf::display() / Path::to_string_lossy()
// emitting backslashes that downstream parsers (`af-error-explainer`,
// `gh issue create --body-file`, third-party agents) cannot consume
// reliably.
//
// The test walks every string leaf in the JSON tree and asserts:
//
//   * No leaf contains an unescaped backslash adjacent to a path
//     component (heuristic: a backslash followed by an alphanumeric or
//     forward slash).
//
// On Linux the test is essentially a smoke gate; on Windows CI it
// becomes a real regression catch.

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

/// Heuristic: any backslash followed by a letter/digit or `/` likely
/// indicates a Windows-style path separator leaking into JSON output.
fn looks_like_pathy_backslash(s: &str) -> bool {
    let bytes = s.as_bytes();
    for i in 0..bytes.len().saturating_sub(1) {
        if bytes[i] == b'\\' {
            let next = bytes[i + 1];
            if next.is_ascii_alphanumeric() || next == b'/' || next == b'.' || next == b'_' {
                // Allow JSON escape sequences (\", \\, \n, \r, \t, \u).
                // Those will not appear as raw bytes in the parsed
                // string; if we see a real `\` it really is a backslash.
                return true;
            }
        }
    }
    false
}

fn walk_strings<F: FnMut(&str, &str)>(value: &Value, path: &str, f: &mut F) {
    match value {
        Value::String(s) => f(path, s),
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                walk_strings(v, &format!("{path}[{i}]"), f);
            }
        }
        Value::Object(map) => {
            for (k, v) in map {
                let p = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                walk_strings(v, &p, f);
            }
        }
        _ => {}
    }
}

fn assert_no_backslash_paths(value: &Value, command: &str) {
    let mut offenders: Vec<(String, String)> = Vec::new();
    walk_strings(value, "", &mut |path, s| {
        if looks_like_pathy_backslash(s) {
            offenders.push((path.to_string(), s.to_string()));
        }
    });
    assert!(
        offenders.is_empty(),
        "{command} emitted JSON strings with Windows-style backslash paths:\n{:#?}",
        offenders
    );
}

fn run_json(args: &[&str]) -> Value {
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
    assert!(
        out.status.success(),
        "command {args:?} must succeed:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    serde_json::from_slice(&out.stdout).expect("JSON output")
}

#[test]
fn doctor_json_uses_forward_slash_paths() {
    let v = run_json(&["doctor"]);
    assert_no_backslash_paths(&v, "doctor");
}

#[test]
fn manifest_validate_json_uses_forward_slash_paths() {
    let manifest = repo_root()
        .join("examples")
        .join("af-mod-add")
        .join("af-core.toml");
    let v = run_json(&["manifest", "validate", manifest.to_str().unwrap()]);
    assert_no_backslash_paths(&v, "manifest validate");
}

#[test]
fn core_check_json_uses_forward_slash_paths() {
    let core = repo_root().join("examples").join("af-mod-add");
    let v = run_json(&["core", "check", core.to_str().unwrap()]);
    assert_no_backslash_paths(&v, "core check");
}

#[test]
fn build_json_uses_forward_slash_paths() {
    let build_root = tempfile::TempDir::new().unwrap();
    let core = repo_root().join("examples").join("af-mod-add");
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "build",
            core.to_str().unwrap(),
            "--board",
            "digilent_arty_a7",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_no_backslash_paths(&v, "build");
}

#[test]
fn heuristic_recognises_known_bad_inputs() {
    // Self-test: confirm the heuristic actually fires on Windows-like
    // strings. Guards against the test silently passing on every
    // input.
    assert!(looks_like_pathy_backslash("C:\\Users\\me\\file.txt"));
    assert!(looks_like_pathy_backslash("crates\\af-cli\\src"));
    assert!(!looks_like_pathy_backslash("crates/af-cli/src"));
    assert!(!looks_like_pathy_backslash("plain string with no slashes"));
}
