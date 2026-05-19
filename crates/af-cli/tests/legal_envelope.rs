// SPDX-License-Identifier: Apache-2.0
//
// Live envelope-shape tests for AF_LEGAL_* codes via
// `af core check --json`. Each test mutates a TempDir clone of
// `examples/af-mod-add` to violate one legal-policy rule and asserts
// the resulting JSON envelope.
//
// The CLI surfaces legal-policy failures through
// `CoreError::CheckFailed { report }`. The CliError envelope has
// `code: "AF_CORE_CHECK_FAILED"`, exit code 2, and `details` carrying
// the full `CoreCheckReport` including `legal_issues[]`.

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

fn run_core_check(core: &Path) -> (i32, Value) {
    let build_root = TempDir::new().unwrap();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "core",
            "check",
            core.to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    let exit = out.status.code().expect("exit code");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    (exit, value)
}

fn legal_codes(value: &Value) -> Vec<String> {
    value
        .get("details")
        .and_then(|d| d.get("legal_issues"))
        .and_then(|li| li.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|i| i["code"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn missing_license_file_surfaces_in_envelope() {
    let core = clone_mod_add();
    fs::remove_file(core.path().join("LICENSE")).unwrap();
    let (exit, value) = run_core_check(core.path());
    assert_eq!(exit, 2);
    assert_eq!(value["code"].as_str(), Some("AF_CORE_CHECK_FAILED"));
    let codes = legal_codes(&value);
    assert!(
        codes.contains(&"AF_LEGAL_FILE_MISSING".to_string()),
        "envelope must carry AF_LEGAL_FILE_MISSING; got {codes:?}"
    );
}

#[test]
fn placeholder_license_text_surfaces_in_envelope() {
    let core = clone_mod_add();
    fs::write(
        core.path().join("LICENSE"),
        b"This is a PLACEHOLDER. TBD before release.\n",
    )
    .unwrap();
    let (exit, value) = run_core_check(core.path());
    assert_eq!(exit, 2);
    assert_eq!(value["code"].as_str(), Some("AF_CORE_CHECK_FAILED"));
    let codes = legal_codes(&value);
    assert!(
        codes.contains(&"AF_LEGAL_PLACEHOLDER_TEXT".to_string()),
        "envelope must carry AF_LEGAL_PLACEHOLDER_TEXT; got {codes:?}"
    );
}

#[test]
fn wrong_metadata_license_surfaces_in_envelope() {
    let core = clone_mod_add();
    let manifest = core.path().join("af-core.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    let mutated = text.replace(
        "license = \"AccelFury Source Available License v1.0\"",
        "license = \"MIT\"",
    );
    fs::write(&manifest, mutated).unwrap();
    let (exit, value) = run_core_check(core.path());
    assert_eq!(exit, 2);
    let codes = legal_codes(&value);
    assert!(
        codes.contains(&"AF_LEGAL_LICENSE_POLICY_MISMATCH".to_string()),
        "envelope must carry AF_LEGAL_LICENSE_POLICY_MISMATCH; got {codes:?}"
    );
}

#[test]
fn commercial_boundary_incomplete_surfaces_in_envelope() {
    let core = clone_mod_add();
    fs::write(
        core.path().join("COMMERCIAL-LICENSE.md"),
        b"# Commercial\n\nThis IP is available.\n",
    )
    .unwrap();
    let (exit, value) = run_core_check(core.path());
    assert_eq!(exit, 2);
    let codes = legal_codes(&value);
    assert!(
        codes.contains(&"AF_LEGAL_COMMERCIAL_BOUNDARY_INCOMPLETE".to_string()),
        "envelope must carry AF_LEGAL_COMMERCIAL_BOUNDARY_INCOMPLETE; got {codes:?}"
    );
}
