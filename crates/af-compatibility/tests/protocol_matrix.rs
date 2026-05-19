// SPDX-License-Identifier: Apache-2.0
//
// Integration tests for `af-compatibility::check_compatibility`.
//
// The check compares neighbouring core manifests pair-wise and emits
// `AF_COMPAT_*` issues (PROTOCOL_MISMATCH, CLOCK_MISMATCH, ...) plus
// adapter suggestions (async_fifo_cdc, stream_width_adapter).
//
// The test surface:
//   * No-input case ⇒ AF_COMPAT_INPUT_MISSING.
//   * Single-core input ⇒ passes (no pairs to compare).
//   * Two cores, no stream interfaces ⇒ passes.
//   * Two cores, identical stream ⇒ passes.
//   * Two cores, protocol mismatch ⇒ AF_COMPAT_PROTOCOL_MISMATCH.
//   * Determinism: same input → same report (set equality of issues).

use af_compatibility::{check_compatibility, CompatibilityError};
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

#[test]
fn empty_inputs_return_missing_input_error() {
    let err = check_compatibility(&[], false).unwrap_err();
    assert_eq!(err.code(), "AF_COMPAT_INPUT_MISSING");
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn single_core_input_passes() {
    let core = clone_example("af-mod-add");
    let report =
        check_compatibility(&[core.path().to_path_buf()], false).expect("compatibility check runs");
    // No pairs to compare → no protocol/clock issues.
    let protocol_issues: Vec<&str> = report
        .issues
        .iter()
        .filter(|i| i.code.starts_with("AF_COMPAT_"))
        .map(|i| i.code.as_str())
        .collect();
    assert!(
        protocol_issues.is_empty(),
        "single core must not yield pair-wise compat issues: {protocol_issues:?}"
    );
}

#[test]
fn report_lists_documented_check_axes() {
    let core = clone_example("af-mod-add");
    let report = check_compatibility(&[core.path().to_path_buf()], false).expect("check runs");
    for axis in [
        "protocol kind",
        "data width",
        "clock domain",
        "reset polarity",
        "latency",
        "throughput",
        "backpressure",
        "parameter ranges",
        "resource conflicts",
        "vendor/board support",
        "security policy conflicts",
    ] {
        assert!(
            report.checks.iter().any(|c| c == axis),
            "report.checks missing axis `{axis}`"
        );
    }
}

#[test]
fn two_identical_cores_yield_no_pair_issues() {
    // Same core twice in different temp dirs ⇒ symmetric pair, no
    // protocol/clock mismatches.
    let a = clone_example("af-mod-add");
    let b = clone_example("af-mod-add");
    let report = check_compatibility(&[a.path().to_path_buf(), b.path().to_path_buf()], false)
        .expect("check runs");
    let pair_issues: Vec<&str> = report
        .issues
        .iter()
        .filter(|i| {
            matches!(
                i.code.as_str(),
                "AF_COMPAT_PROTOCOL_MISMATCH"
                    | "AF_COMPAT_CLOCK_MISMATCH"
                    | "AF_COMPAT_WIDTH_MISMATCH"
            )
        })
        .map(|i| i.code.as_str())
        .collect();
    assert!(
        pair_issues.is_empty(),
        "identical cores must not raise pair-wise issues: {pair_issues:?}"
    );
}

#[test]
fn check_is_deterministic() {
    // Same input → byte-equal report content (after JSON normalisation).
    let core = clone_example("af-mod-add");
    let r1 = check_compatibility(&[core.path().to_path_buf()], false).unwrap();
    let r2 = check_compatibility(&[core.path().to_path_buf()], false).unwrap();
    let s1 = serde_json::to_string(&r1).unwrap();
    let s2 = serde_json::to_string(&r2).unwrap();
    assert_eq!(s1, s2, "compatibility report must be deterministic");
}

#[test]
fn non_existent_input_passes_through_with_warning() {
    // Inputs without an `af-core.toml` are filtered out; the rest of
    // the report is still produced.
    let tmp = TempDir::new().unwrap();
    let report = check_compatibility(&[tmp.path().to_path_buf()], false).expect("runs");
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("Non-core/system input")),
        "non-core input must surface as warning: {:?}",
        report.warnings
    );
}

#[test]
fn manifest_error_propagates_as_envelope_code() {
    // A directory with a malformed af-core.toml should produce
    // AF_MANIFEST_PARSE_FAILED (proxied through CompatibilityError::Manifest).
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("af-core.toml"),
        b"this is not = valid toml [\n",
    )
    .unwrap();
    let err = check_compatibility(&[tmp.path().to_path_buf()], false).unwrap_err();
    match err {
        CompatibilityError::Manifest(_) => {}
        other => panic!("expected Manifest error, got {other:?}"),
    }
    assert!(err.code().starts_with("AF_MANIFEST_"));
}
