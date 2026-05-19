// SPDX-License-Identifier: Apache-2.0
//
// Insta-based snapshots of stable JSON-output topology. We snapshot
// SHAPE, not exact values: paths, tool versions, command durations,
// commit SHA, and host-specific environment vary across machines, so
// we redact them and snapshot the structural skeleton.
//
// This catches:
//   * accidental key renames (e.g. `tool_versions` → `tools`),
//   * silently-added top-level keys,
//   * removed required keys,
//   * type changes (string → object, array → object).
//
// Snapshots live under `crates/af-cli/tests/snapshots/`. Review with
// `cargo insta review` on first run; CI uses `cargo insta test
// --check` to refuse auto-accept.

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

/// Replace every leaf in the JSON tree with a placeholder describing
/// its type, so the snapshot is a TOPOLOGY skeleton (key names + value
/// types only).
fn shape_of(v: &Value) -> Value {
    match v {
        Value::Null => Value::String("<null>".into()),
        Value::Bool(_) => Value::String("<bool>".into()),
        Value::Number(_) => Value::String("<number>".into()),
        Value::String(_) => Value::String("<string>".into()),
        Value::Array(items) => {
            // Snapshot the type of the first element (or empty marker)
            // so an empty array vs. an array of strings is observable.
            if items.is_empty() {
                Value::Array(vec![])
            } else {
                Value::Array(vec![shape_of(&items[0])])
            }
        }
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, val) in map {
                out.insert(k.clone(), shape_of(val));
            }
            Value::Object(out)
        }
    }
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
fn doctor_json_topology_matches_snapshot() {
    let value = run_json(&["doctor"]);
    let topology = shape_of(&value);
    insta::assert_json_snapshot!("doctor_topology", topology);
}

#[test]
fn manifest_validate_topology_matches_snapshot() {
    let manifest = repo_root()
        .join("examples")
        .join("af-mod-add")
        .join("af-core.toml");
    let value = run_json(&["manifest", "validate", manifest.to_str().unwrap()]);
    let topology = shape_of(&value);
    insta::assert_json_snapshot!("manifest_validate_topology", topology);
}

#[test]
fn core_check_topology_matches_snapshot() {
    let core = repo_root().join("examples").join("af-mod-add");
    let value = run_json(&["core", "check", core.to_str().unwrap()]);
    let topology = shape_of(&value);
    insta::assert_json_snapshot!("core_check_topology", topology);
}
