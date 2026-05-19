// SPDX-License-Identifier: Apache-2.0
//
// Property-based fuzz of `check_core` legal-policy permutations.
//
// We mutate a TempDir clone of af-mod-add along three axes:
//   * keep / remove LICENSE
//   * keep / remove COMMERCIAL-LICENSE.md
//   * approved / wrong / missing metadata.license
//
// Invariants:
//   - Clean reference always passes.
//   - Removing either legal file always raises AF_LEGAL_FILE_MISSING.
//   - Setting a wrong license string raises AF_LEGAL_LICENSE_POLICY_MISMATCH.

use af_core::{check_core, CoreError};
use proptest::prelude::*;
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

fn clone_mod_add() -> TempDir {
    let src = repo_root().join("examples").join("af-mod-add");
    let tmp = TempDir::new().unwrap();
    copy_dir_all(&src, tmp.path());
    tmp
}

fn legal_codes(err: &CoreError) -> Vec<String> {
    match err {
        CoreError::CheckFailed { report } => {
            report.legal_issues.iter().map(|i| i.code.clone()).collect()
        }
        _ => Vec::new(),
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]

    /// For any permutation of `remove_license / remove_commercial`, the
    /// resulting envelope must surface AF_LEGAL_FILE_MISSING when at
    /// least one legal file is absent.
    #[test]
    fn legal_file_missing_is_flagged(remove_license: bool, remove_commercial: bool) {
        let core = clone_mod_add();
        if remove_license {
            fs::remove_file(core.path().join("LICENSE")).unwrap();
        }
        if remove_commercial {
            fs::remove_file(core.path().join("COMMERCIAL-LICENSE.md")).unwrap();
        }
        let result = check_core(core.path());
        if !remove_license && !remove_commercial {
            // Reference state — must pass.
            prop_assert!(result.is_ok(), "clean fixture must pass: {result:?}");
        } else {
            let err = match result {
                Ok(_) => panic!("missing legal file must fail"),
                Err(e) => e,
            };
            let codes = legal_codes(&err);
            prop_assert!(
                codes.iter().any(|c| c == "AF_LEGAL_FILE_MISSING"),
                "expected AF_LEGAL_FILE_MISSING, got {codes:?}"
            );
        }
    }

    /// Arbitrary non-approved license string ⇒ AF_LEGAL_LICENSE_POLICY_MISMATCH.
    #[test]
    fn wrong_metadata_license_is_flagged(license in "[a-zA-Z0-9 ]{3,20}") {
        // Skip if random string accidentally equals the approved one.
        if license.contains("AccelFury Source Available License v1.0") {
            return Ok(());
        }
        let core = clone_mod_add();
        let manifest = core.path().join("af-core.toml");
        let text = fs::read_to_string(&manifest).unwrap();
        let mutated = text.replace(
            "license = \"AccelFury Source Available License v1.0\"",
            &format!("license = \"{license}\""),
        );
        fs::write(&manifest, mutated).unwrap();

        let err = check_core(core.path()).unwrap_err();
        let codes = legal_codes(&err);
        prop_assert!(
            codes.iter().any(|c| c == "AF_LEGAL_LICENSE_POLICY_MISMATCH"),
            "wrong license `{license}` must raise mismatch; got {codes:?}"
        );
    }
}
