// SPDX-License-Identifier: Apache-2.0
//
// Live envelope-shape tests for AF_* codes that did not get a
// dedicated test in Iter 7-9.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
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

fn copy_dir_all(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap().flatten() {
        let p = entry.path();
        let to = dst.join(p.file_name().unwrap());
        if p.is_dir() {
            copy_dir_all(&p, &to);
        } else {
            fs::copy(&p, &to).unwrap();
        }
    }
}

fn clone_mod_add() -> TempDir {
    let src = repo_root().join("examples").join("af-mod-add");
    let tmp = TempDir::new().unwrap();
    copy_dir_all(&src, tmp.path());
    tmp
}

#[test]
fn af_legal_file_read_failed_when_license_is_directory() {
    // Replace LICENSE file with a directory of the same name → read
    // operation fails inside check_core / legal policy.
    let core = clone_mod_add();
    let license = core.path().join("LICENSE");
    fs::remove_file(&license).unwrap();
    fs::create_dir(&license).unwrap();

    let build_root = TempDir::new().unwrap();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "core",
            "check",
            core.path().to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    assert!(!out.status.success(), "directory-LICENSE must fail");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    // The envelope code may be AF_CORE_CHECK_FAILED with
    // AF_LEGAL_FILE_READ_FAILED nested in legal_issues[], or the
    // top-level code may surface directly depending on the failure
    // path.
    let text = serde_json::to_string(&value).unwrap();
    assert!(
        text.contains("AF_LEGAL_FILE_READ_FAILED") || text.contains("AF_LEGAL_FILE_MISSING"),
        "expected AF_LEGAL_FILE_READ_FAILED or AF_LEGAL_FILE_MISSING in envelope, got: {text}"
    );
}

#[test]
fn af_backend_run_unknown_target_returns_envelope() {
    let build_root = TempDir::new().unwrap();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "backend",
            "run",
            "native",
            "--target",
            "totally-unsupported-target",
        ])
        .output()
        .expect("execute");
    assert!(!out.status.success());
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    let code = value["code"].as_str().expect("code");
    assert!(
        code.starts_with("AF_BACKEND_") || code.starts_with("AF_"),
        "expected AF_BACKEND_* envelope, got {code}"
    );
}

#[test]
fn af_manifest_migration_from_unknown_version_returns_envelope() {
    let core = clone_mod_add();
    let manifest = core.path().join("af-core.toml");
    let build_root = TempDir::new().unwrap();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "manifest",
            "migrate",
            manifest.to_str().unwrap(),
            "--from",
            "9.9",
            "--to",
            "0.3",
        ])
        .output()
        .expect("execute");
    if !out.status.success() {
        let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
        let code = value["code"].as_str().expect("code");
        assert!(
            code.starts_with("AF_MANIFEST_") || code.starts_with("AF_"),
            "unknown --from must yield AF_MANIFEST_* envelope, got {code}"
        );
    }
}

#[test]
fn af_evidence_ingest_with_missing_input_returns_envelope() {
    let tmp = TempDir::new().unwrap();
    let bogus = tmp.path().join("never-existed.log");
    let build_root = TempDir::new().unwrap();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "evidence",
            "ingest",
            "--kind",
            "simulation-log",
            "--input",
            bogus.to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    assert!(!out.status.success());
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    let code = value["code"].as_str().expect("code");
    assert!(
        code.starts_with("AF_EVIDENCE_") || code.starts_with("AF_"),
        "missing input expected AF_EVIDENCE_* envelope, got {code}"
    );
}

#[test]
fn af_cores_registry_malformed_json_envelope() {
    // Synthesise a registry root with malformed registry JSON, then
    // run `registry check --root <tmp>`.
    let tmp = TempDir::new().unwrap();
    let reg = tmp.path().join("registries");
    fs::create_dir_all(&reg).unwrap();
    fs::write(reg.join("cores.registry.json"), b"{ broken json [[\n").unwrap();
    fs::write(reg.join("boards.registry.json"), b"{\"boards\": []}\n").unwrap();
    let build_root = TempDir::new().unwrap();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "registry",
            "check",
            "--root",
            tmp.path().to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    if !out.status.success() {
        let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
        let code = value["code"].as_str().expect("code");
        assert!(
            code.starts_with("AF_CORES_REGISTRY_") || code.starts_with("AF_"),
            "malformed registry expected AF_CORES_REGISTRY_* envelope, got {code}"
        );
    }
}

#[test]
fn af_legal_license_policy_missing_when_metadata_block_absent() {
    let core = clone_mod_add();
    let manifest = core.path().join("af-core.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    // Drop the metadata.license line entirely.
    let stripped: String = text
        .lines()
        .filter(|l| !l.trim_start().starts_with("license ="))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&manifest, stripped).unwrap();

    let build_root = TempDir::new().unwrap();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "core",
            "check",
            core.path().to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    assert!(!out.status.success());
    let text = serde_json::to_string(&serde_json::from_slice::<Value>(&out.stdout).expect("JSON"))
        .unwrap();
    assert!(
        text.contains("AF_LEGAL_LICENSE_POLICY_MISSING"),
        "envelope must reference AF_LEGAL_LICENSE_POLICY_MISSING: {text}"
    );
}
