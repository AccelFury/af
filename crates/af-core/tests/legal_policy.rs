// SPDX-License-Identifier: Apache-2.0
//
// `check_core` enforces legal-policy invariants on every reusable core:
//
//   AF_LEGAL_LICENSE_POLICY_MISMATCH    metadata.license != approved policy
//   AF_LEGAL_LICENSE_POLICY_MISSING     metadata.license absent
//   AF_LEGAL_FILE_MISSING               LICENSE / COMMERCIAL-LICENSE.md absent
//   AF_LEGAL_FILE_READ_FAILED           file present but unreadable
//   AF_LEGAL_PLACEHOLDER_TEXT           "placeholder"/"tbd"/etc inside legal text
//   AF_LEGAL_COMMERCIAL_BOUNDARY_INCOMPLETE
//                                        COMMERCIAL-LICENSE.md missing boundary
//                                        text (paid license / closed-source
//                                        trigger / support+warranty)
//
// We test each fail-closed branch via a TempDir copy of the reference
// `examples/af-mod-add/` core, mutating only the offending legal file.

use af_core::{check_core, CoreError};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
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

fn clone_example(name: &str) -> TempDir {
    let src = repo_root().join("examples").join(name);
    let tmp = TempDir::new().unwrap();
    copy_dir_all(&src, tmp.path());
    tmp
}

fn issues_of(err: &CoreError) -> Vec<String> {
    match err {
        CoreError::CheckFailed { report } => {
            report.legal_issues.iter().map(|i| i.code.clone()).collect()
        }
        _ => Vec::new(),
    }
}

#[test]
fn reference_fixture_passes_legal_policy() {
    let core = clone_example("af-mod-add");
    let report = check_core(core.path()).expect("af-mod-add must pass core check");
    assert_eq!(report.status, "passed");
    assert!(
        report.legal_issues.is_empty(),
        "reference fixture must not raise legal issues: {:?}",
        report.legal_issues
    );
}

#[test]
fn missing_license_file_is_flagged() {
    let core = clone_example("af-mod-add");
    fs::remove_file(core.path().join("LICENSE")).unwrap();
    let err = check_core(core.path()).unwrap_err();
    let codes = issues_of(&err);
    assert!(
        codes.contains(&"AF_LEGAL_FILE_MISSING".to_string()),
        "missing LICENSE must raise AF_LEGAL_FILE_MISSING: {codes:?}"
    );
}

#[test]
fn missing_commercial_license_file_is_flagged() {
    let core = clone_example("af-mod-add");
    fs::remove_file(core.path().join("COMMERCIAL-LICENSE.md")).unwrap();
    let err = check_core(core.path()).unwrap_err();
    let codes = issues_of(&err);
    assert!(
        codes.contains(&"AF_LEGAL_FILE_MISSING".to_string()),
        "missing COMMERCIAL-LICENSE.md must raise AF_LEGAL_FILE_MISSING: {codes:?}"
    );
}

#[test]
fn wrong_metadata_license_is_flagged() {
    let core = clone_example("af-mod-add");
    let manifest = core.path().join("af-core.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    let mutated = text.replace(
        "license = \"AccelFury Source Available License v1.0\"",
        "license = \"MIT\"",
    );
    fs::write(&manifest, mutated).unwrap();

    let err = check_core(core.path()).unwrap_err();
    let codes = issues_of(&err);
    assert!(
        codes.contains(&"AF_LEGAL_LICENSE_POLICY_MISMATCH".to_string()),
        "wrong [metadata].license must raise AF_LEGAL_LICENSE_POLICY_MISMATCH: {codes:?}"
    );
}

#[test]
fn missing_metadata_license_is_flagged() {
    let core = clone_example("af-mod-add");
    let manifest = core.path().join("af-core.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    // Strip the license line entirely.
    let stripped: String = text
        .lines()
        .filter(|line| !line.trim_start().starts_with("license ="))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&manifest, stripped).unwrap();

    let err = check_core(core.path()).unwrap_err();
    let codes = issues_of(&err);
    assert!(
        codes.contains(&"AF_LEGAL_LICENSE_POLICY_MISSING".to_string()),
        "absent [metadata].license must raise AF_LEGAL_LICENSE_POLICY_MISSING: {codes:?}"
    );
}

#[test]
fn placeholder_text_in_license_is_flagged() {
    let core = clone_example("af-mod-add");
    fs::write(
        core.path().join("LICENSE"),
        b"This is a PLACEHOLDER legal text. TBD before release.\n",
    )
    .unwrap();
    let err = check_core(core.path()).unwrap_err();
    let codes = issues_of(&err);
    assert!(
        codes.contains(&"AF_LEGAL_PLACEHOLDER_TEXT".to_string()),
        "placeholder text in LICENSE must raise AF_LEGAL_PLACEHOLDER_TEXT: {codes:?}"
    );
}

#[test]
fn commercial_license_missing_boundary_is_flagged() {
    let core = clone_example("af-mod-add");
    // Replace COMMERCIAL-LICENSE.md with a text that lacks the required
    // boundary keywords (paid commercial / closed-source / support
    // boundary).
    fs::write(
        core.path().join("COMMERCIAL-LICENSE.md"),
        b"# Commercial\n\nThis IP is available.\n",
    )
    .unwrap();
    let err = check_core(core.path()).unwrap_err();
    let codes = issues_of(&err);
    assert!(
        codes.contains(&"AF_LEGAL_COMMERCIAL_BOUNDARY_INCOMPLETE".to_string()),
        "COMMERCIAL-LICENSE.md without boundary keywords must raise AF_LEGAL_COMMERCIAL_BOUNDARY_INCOMPLETE: {codes:?}"
    );
}

#[test]
fn check_core_envelope_code_is_af_core_check_failed() {
    // Any legal-policy failure routes through `CoreError::CheckFailed`.
    let core = clone_example("af-mod-add");
    fs::remove_file(core.path().join("LICENSE")).unwrap();
    let err = check_core(core.path()).unwrap_err();
    assert_eq!(err.code(), "AF_CORE_CHECK_FAILED");
    assert_eq!(err.exit_code(), 2);
}
