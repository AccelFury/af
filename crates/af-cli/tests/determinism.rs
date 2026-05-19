// SPDX-License-Identifier: Apache-2.0
//
// `af --json` invocations are required to be deterministic for the same
// input. The CLI may include timestamps and command durations in human
// output; the JSON tree must not.
//
// We don't enforce byte-for-byte equality of every command (some still
// contain `duration_ms` or per-run paths). What we do enforce:
//
// 1. `manifest validate <example>` produces byte-equal JSON on n=3 runs.
// 2. `doctor --json` produces a JSON value whose `schema_version` /
//    `kind` / `tool_versions` topology is stable across runs.

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
    Command::cargo_bin("af").expect("cargo bin `af` builds")
}

fn run_json(args: &[&str]) -> Value {
    let out = af().args(args).output().expect("execute");
    assert!(
        out.status.success(),
        "command {args:?} must succeed; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("stdout must be JSON")
}

#[test]
fn manifest_validate_is_byte_deterministic() {
    let manifest = repo_root()
        .join("examples")
        .join("af-mod-add")
        .join("af-core.toml");
    let manifest_str = manifest.to_str().unwrap();
    let args = ["--json", "manifest", "validate", manifest_str];

    let mut bytes_runs: Vec<Vec<u8>> = Vec::new();
    for _ in 0..3 {
        let out = af().args(args).output().expect("execute");
        assert!(out.status.success(), "manifest validate must succeed");
        bytes_runs.push(out.stdout);
    }
    let first = &bytes_runs[0];
    for (i, run) in bytes_runs.iter().enumerate().skip(1) {
        assert_eq!(
            first.len(),
            run.len(),
            "manifest validate run #{i} has different byte length"
        );
        assert_eq!(
            first, run,
            "manifest validate run #{i} differs byte-for-byte"
        );
    }
}

#[test]
fn doctor_topology_is_stable_across_runs() {
    let runs: Vec<Value> = (0..2).map(|_| run_json(&["--json", "doctor"])).collect();
    // Hard contract: `schema_version` and `kind` are pinned values.
    for v in &runs {
        assert_eq!(v["schema_version"].as_str(), Some("0.1"));
        assert_eq!(v["kind"].as_str(), Some("accelfury.report"));
    }
    // Soft contract: the set of probed tool names is identical across
    // runs (versions might vary if a new tool was installed mid-run,
    // but the inventory is fixed in code).
    let names = |v: &Value| -> std::collections::BTreeSet<String> {
        v["tool_versions"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|t| t["tool"].as_str().map(String::from))
            .collect()
    };
    assert_eq!(
        names(&runs[0]),
        names(&runs[1]),
        "doctor tool inventory drifted between runs"
    );
}
