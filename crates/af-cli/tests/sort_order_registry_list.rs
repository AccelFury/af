// SPDX-License-Identifier: Apache-2.0
//
// `af core registry list --json` and `af board list --json` must
// produce byte-identical bytes across multiple runs (BTreeMap order).

use assert_cmd::Command;
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

fn run_in(br: &std::path::Path, args: &[&str]) -> Vec<u8> {
    let mut full = vec![
        "--json".to_string(),
        "--build-root".to_string(),
        br.to_str().unwrap().to_string(),
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

#[test]
fn core_registry_list_byte_identical_across_3_runs() {
    let br = TempDir::new().unwrap();
    let a = run_in(br.path(), &["core", "registry", "list"]);
    let b = run_in(br.path(), &["core", "registry", "list"]);
    let c = run_in(br.path(), &["core", "registry", "list"]);
    assert_eq!(a, b);
    assert_eq!(b, c);
}

#[test]
fn core_registry_list_with_filter_byte_identical() {
    let br = TempDir::new().unwrap();
    let a = run_in(br.path(), &["core", "registry", "list", "--priority", "P0"]);
    let b = run_in(br.path(), &["core", "registry", "list", "--priority", "P0"]);
    assert_eq!(a, b);
}

#[test]
fn board_list_byte_identical_across_3_runs() {
    let br = TempDir::new().unwrap();
    let a = run_in(br.path(), &["board", "list"]);
    let b = run_in(br.path(), &["board", "list"]);
    let c = run_in(br.path(), &["board", "list"]);
    assert_eq!(a, b);
    assert_eq!(b, c);
}
