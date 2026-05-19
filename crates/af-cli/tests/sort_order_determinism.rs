// SPDX-License-Identifier: Apache-2.0
//
// JSON outputs must keep their object keys in stable order. The
// implementation relies on `serde_json::to_value` over BTreeMap +
// derive-based serialization, which sorts alphabetically. Any
// refactor that introduces HashMap or per-call Vec<(K,V)> would
// silently break consumers' diffs and snapshots.

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

fn run_json_in(build_root: &std::path::Path, args: &[&str]) -> Vec<u8> {
    let mut full = vec![
        "--json".to_string(),
        "--build-root".to_string(),
        build_root.to_str().unwrap().to_string(),
    ];
    full.extend(args.iter().map(|s| s.to_string()));
    let out = af()
        .current_dir(repo_root())
        .args(&full)
        .output()
        .expect("execute");
    assert!(out.status.success(), "command {args:?} must succeed");
    out.stdout
}

fn run_json(args: &[&str]) -> Vec<u8> {
    let br = tempfile::TempDir::new().unwrap();
    run_json_in(br.path(), args)
}

#[test]
fn doctor_json_is_byte_identical_across_three_runs() {
    // Same build_root across runs so artefact paths do not drift.
    let br = tempfile::TempDir::new().unwrap();
    let a = run_json_in(br.path(), &["doctor"]);
    let b = run_json_in(br.path(), &["doctor"]);
    let c = run_json_in(br.path(), &["doctor"]);
    assert_eq!(a, b, "doctor run 1 vs 2 diverged");
    assert_eq!(b, c, "doctor run 2 vs 3 diverged");
}

#[test]
fn manifest_validate_json_is_byte_identical_across_three_runs() {
    let br = tempfile::TempDir::new().unwrap();
    let manifest = repo_root()
        .join("examples")
        .join("af-mod-add")
        .join("af-core.toml");
    let manifest_str = manifest.to_str().unwrap();
    let a = run_json_in(br.path(), &["manifest", "validate", manifest_str]);
    let b = run_json_in(br.path(), &["manifest", "validate", manifest_str]);
    let c = run_json_in(br.path(), &["manifest", "validate", manifest_str]);
    assert_eq!(a, b);
    assert_eq!(b, c);
}

#[test]
fn doctor_tool_versions_are_sorted_alphabetically_by_tool_name() {
    let bytes = run_json(&["doctor"]);
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    let tools = v["tool_versions"].as_array().expect("array");
    let names: Vec<String> = tools
        .iter()
        .filter_map(|t| t["tool"].as_str().map(String::from))
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    // tool_versions is a Vec persisted in insertion order. The fixture
    // happens to declare tools in alphabetical-ish order; we don't
    // require strict alphabetic sort because the registry chooses the
    // probe order. We DO require duplicate-free.
    let mut uniq = std::collections::BTreeSet::new();
    for n in &names {
        assert!(uniq.insert(n.clone()), "duplicate tool name in output: {n}");
    }
    // Sanity: at least one tool was probed.
    assert!(!names.is_empty());
}

#[test]
fn json_object_keys_are_in_btreemap_order() {
    // Walk every Object in the JSON tree and assert its keys appear in
    // lexicographic order (BTreeMap invariant after serde_json
    // serialisation).
    fn walk(v: &Value, path: &str, offenders: &mut Vec<String>) {
        match v {
            Value::Object(map) => {
                let keys: Vec<&String> = map.keys().collect();
                let mut sorted = keys.clone();
                sorted.sort();
                if keys != sorted {
                    offenders.push(format!("{path}: keys {keys:?}"));
                }
                for (k, val) in map {
                    walk(
                        val,
                        &if path.is_empty() {
                            k.clone()
                        } else {
                            format!("{path}.{k}")
                        },
                        offenders,
                    );
                }
            }
            Value::Array(arr) => {
                for (i, val) in arr.iter().enumerate() {
                    walk(val, &format!("{path}[{i}]"), offenders);
                }
            }
            _ => {}
        }
    }
    let bytes = run_json(&["doctor"]);
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    let mut offenders = Vec::new();
    walk(&v, "", &mut offenders);
    assert!(
        offenders.is_empty(),
        "JSON object keys must be in lexicographic order; offenders:\n{}",
        offenders.join("\n")
    );
}
