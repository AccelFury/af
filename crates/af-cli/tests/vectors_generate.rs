// SPDX-License-Identifier: Apache-2.0
//
// `af vectors generate` produces basic + random vectors with a metadata
// hash. Tests use explicit `--basic-out`/`--random-out`/`--svh-out` so
// the command never writes to the repo tree.

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn af() -> Command {
    Command::cargo_bin("af").expect("cargo bin `af`")
}

fn run_generate(args: &[&str]) -> (i32, Value) {
    let build_root = TempDir::new().unwrap();
    let mut full = vec![
        "--json",
        "--build-root",
        build_root.path().to_str().unwrap(),
        "vectors",
        "generate",
    ];
    full.extend_from_slice(args);
    let out = af().args(&full).output().expect("execute");
    let exit = out.status.code().expect("exit");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    (exit, value)
}

#[test]
fn generate_with_explicit_paths_emits_status_passed() {
    let tmp = TempDir::new().unwrap();
    let basic = tmp.path().join("basic.json");
    let random = tmp.path().join("random.json");
    let svh = tmp.path().join("random.svh");
    let (exit, value) = run_generate(&[
        "--basic-out",
        basic.to_str().unwrap(),
        "--random-out",
        random.to_str().unwrap(),
        "--svh-out",
        svh.to_str().unwrap(),
        "--count",
        "16",
        "--seed",
        "0xCAFEBABEDEADBEEF",
    ]);
    assert_eq!(exit, 0);
    assert_eq!(value["status"].as_str(), Some("passed"));
    assert!(basic.is_file());
    assert!(random.is_file());
    assert!(svh.is_file());
}

#[test]
fn generate_invalid_seed_emits_envelope() {
    let tmp = TempDir::new().unwrap();
    let basic = tmp.path().join("basic.json");
    let random = tmp.path().join("random.json");
    let svh = tmp.path().join("random.svh");
    let (exit, value) = run_generate(&[
        "--basic-out",
        basic.to_str().unwrap(),
        "--random-out",
        random.to_str().unwrap(),
        "--svh-out",
        svh.to_str().unwrap(),
        "--count",
        "4",
        "--seed",
        "not-a-hex-seed",
    ]);
    assert!(exit != 0, "invalid seed must fail");
    let code = value["code"].as_str().expect("code");
    assert!(
        code.starts_with("AF_"),
        "invalid seed envelope must use AF_* code, got {code}"
    );
}

#[test]
fn generate_same_seed_yields_byte_identical_files() {
    let a = TempDir::new().unwrap();
    let b = TempDir::new().unwrap();
    for dir in [&a, &b] {
        let basic = dir.path().join("basic.json");
        let random = dir.path().join("random.json");
        let svh = dir.path().join("random.svh");
        let (exit, _) = run_generate(&[
            "--basic-out",
            basic.to_str().unwrap(),
            "--random-out",
            random.to_str().unwrap(),
            "--svh-out",
            svh.to_str().unwrap(),
            "--count",
            "8",
            "--seed",
            "0xDEADBEEFCAFEBABE",
        ]);
        assert_eq!(exit, 0);
    }
    for name in ["basic.json", "random.json", "random.svh"] {
        let ya = std::fs::read(a.path().join(name)).unwrap();
        let yb = std::fs::read(b.path().join(name)).unwrap();
        assert_eq!(ya, yb, "{name} must be byte-identical across runs");
    }
}
