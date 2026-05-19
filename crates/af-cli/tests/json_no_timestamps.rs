// SPDX-License-Identifier: Apache-2.0
//
// Active assertion: CLI JSON outputs must not include ISO-8601-style
// timestamps or Unix-epoch numeric timestamps that would break
// reproducibility / snapshot stability. The walker scans every string
// leaf in the JSON tree.

use assert_cmd::Command;
use regex::Regex;
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

fn run_json(args: &[&str]) -> Value {
    let build_root = tempfile::TempDir::new().unwrap();
    let mut full = vec![
        "--json".to_string(),
        "--build-root".to_string(),
        build_root.path().to_str().unwrap().to_string(),
    ];
    full.extend(args.iter().map(|s| s.to_string()));
    let out = af()
        .current_dir(repo_root())
        .args(&full)
        .output()
        .expect("execute");
    assert!(out.status.success(), "command {args:?} must succeed");
    serde_json::from_slice(&out.stdout).expect("JSON")
}

fn walk_strings<F: FnMut(&str, &str)>(v: &Value, path: &str, f: &mut F) {
    match v {
        Value::String(s) => f(path, s),
        Value::Array(arr) => {
            for (i, x) in arr.iter().enumerate() {
                walk_strings(x, &format!("{path}[{i}]"), f);
            }
        }
        Value::Object(map) => {
            for (k, val) in map {
                let p = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                walk_strings(val, &p, f);
            }
        }
        _ => {}
    }
}

fn assert_no_timestamps(v: &Value, command: &str) {
    // ISO-8601: 2024-01-15T13:45:00 (with optional Z, ±hh:mm).
    let iso = Regex::new(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}").unwrap();
    let mut offenders: Vec<(String, String)> = Vec::new();
    walk_strings(v, "", &mut |path, s| {
        if iso.is_match(s) {
            offenders.push((path.to_string(), s.to_string()));
        }
    });
    assert!(
        offenders.is_empty(),
        "{command} JSON leaks ISO-8601 timestamps:\n{:#?}",
        offenders
    );
}

#[test]
fn doctor_json_has_no_iso8601_timestamps() {
    let v = run_json(&["doctor"]);
    assert_no_timestamps(&v, "doctor");
}

#[test]
fn manifest_validate_json_has_no_iso8601_timestamps() {
    let manifest = repo_root()
        .join("examples")
        .join("af-mod-add")
        .join("af-core.toml");
    let v = run_json(&["manifest", "validate", manifest.to_str().unwrap()]);
    assert_no_timestamps(&v, "manifest validate");
}

#[test]
fn core_check_json_has_no_iso8601_timestamps() {
    let core = repo_root().join("examples").join("af-mod-add");
    let v = run_json(&["core", "check", core.to_str().unwrap()]);
    assert_no_timestamps(&v, "core check");
}
