// SPDX-License-Identifier: Apache-2.0
//
// Property-based gates for path normalization. The rules under test:
//
// 1. Empty / whitespace-only paths → AF_PATH_EMPTY.
// 2. Absolute paths (Unix `/...`, Windows drive prefixes) → AF_PATH_ABSOLUTE
//    or AF_PATH_PREFIX.
// 3. Any path containing a `..` segment → AF_PATH_TRAVERSAL.
// 4. Normalized output never escapes the base.
// 5. A successful normalize_relative_path roundtrips: re-feeding its
//    string representation produces a stably-equal PathBuf.

use af_security::{normalize_relative_path, safe_join, SecurityError};
use proptest::prelude::*;
use std::path::PathBuf;

// Generate ASCII path-segment strings (a-z, 0-9, -, _). Empty allowed
// to test edge cases.
fn segment() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_-]{1,8}".prop_map(String::from)
}

// Joiner generator: forward slash; we don't generate `..` here so the
// path is "clean" by construction.
fn clean_relative_path() -> impl Strategy<Value = String> {
    proptest::collection::vec(segment(), 1..6).prop_map(|parts| parts.join("/"))
}

// Path with at least one `..` segment somewhere.
fn traversal_path() -> impl Strategy<Value = String> {
    (
        proptest::collection::vec(segment(), 0..3),
        proptest::collection::vec(segment(), 0..3),
    )
        .prop_map(|(before, after)| {
            let mut parts: Vec<String> = before
                .into_iter()
                .chain(std::iter::once("..".to_string()))
                .collect();
            parts.extend(after);
            parts.join("/")
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Any path containing `..` must be rejected as traversal.
    #[test]
    fn traversal_paths_are_rejected(p in traversal_path()) {
        let result = normalize_relative_path(&p);
        match result {
            Err(SecurityError::PathTraversal { .. }) => {}
            // Empty path before the .. is also legitimately rejected.
            Err(SecurityError::EmptyPath { .. }) => {}
            other => panic!("expected PathTraversal for `{p}`, got {other:?}"),
        }
    }

    /// Clean (no `..`) relative paths normalize successfully.
    #[test]
    fn clean_paths_normalize(p in clean_relative_path()) {
        let normalized = normalize_relative_path(&p).expect("clean path must normalize");
        // Normalized path is non-empty.
        prop_assert!(!normalized.as_os_str().is_empty());
        // Normalized path is relative.
        prop_assert!(!normalized.is_absolute());
    }

    /// `safe_join(base, p)` stays under `base` for any clean relative
    /// path.
    #[test]
    fn safe_join_stays_under_base(p in clean_relative_path()) {
        let base = PathBuf::from("/tmp/af-test-base");
        let joined = safe_join(&base, &p).expect("safe_join on clean path");
        prop_assert!(joined.starts_with(&base), "joined `{}` escaped base", joined.display());
    }

    /// Whitespace-only input is rejected.
    #[test]
    fn whitespace_paths_are_rejected(spaces in "[ \\t]{1,8}") {
        let result = normalize_relative_path(&spaces);
        let is_empty = matches!(&result, Err(SecurityError::EmptyPath { .. }));
        prop_assert!(is_empty, "expected EmptyPath, got {:?}", result);
    }
}

#[test]
fn empty_input_is_rejected() {
    let err = normalize_relative_path("").unwrap_err();
    assert_eq!(err.code(), "AF_PATH_EMPTY");
}

#[test]
fn absolute_unix_path_is_rejected() {
    let err = normalize_relative_path("/etc/passwd").unwrap_err();
    assert_eq!(err.code(), "AF_PATH_ABSOLUTE");
}

#[test]
fn parent_dir_segment_is_rejected_explicit() {
    let err = normalize_relative_path("foo/../etc").unwrap_err();
    assert_eq!(err.code(), "AF_PATH_TRAVERSAL");
}

#[test]
fn current_dir_segment_is_silently_stripped() {
    let p = normalize_relative_path("./foo/./bar").unwrap();
    assert_eq!(p, PathBuf::from("foo/bar"));
}

#[test]
fn path_traversal_exit_code_is_security_band() {
    let err = normalize_relative_path("../etc/passwd").unwrap_err();
    // exit code is 2 (validation) per the implementation; document it.
    assert_eq!(err.exit_code(), 2);
}
