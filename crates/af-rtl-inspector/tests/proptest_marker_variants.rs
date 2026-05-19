// SPDX-License-Identifier: Apache-2.0
//
// Property-based fuzz: any UPPER/lower/mIxEd case variant of a known
// vendor marker triggers AF_PORTABLE_VENDOR_OR_CLOCK_MARKER through
// inspect_core on a Verilog-2001 fixture.

use af_manifest::CoreManifest;
use af_rtl_inspector::inspect_core;
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

fn clone_reset_sync() -> TempDir {
    let src = repo_root().join("examples").join("af-reset-sync");
    let tmp = TempDir::new().unwrap();
    copy_dir_all(&src, tmp.path());
    tmp
}

fn inject_in_rtl(project: &Path, snippet: &str) {
    let path = project.join("rtl/af_reset_sync.v");
    let text = fs::read_to_string(&path).unwrap();
    let injected = if let Some(idx) = text.rfind("endmodule") {
        let (head, tail) = text.split_at(idx);
        format!("{head}\n{snippet}\n{tail}")
    } else {
        format!("{text}\n{snippet}\n")
    };
    fs::write(&path, injected).unwrap();
}

fn issues_after(snippet: &str) -> Vec<String> {
    let tmp = clone_reset_sync();
    inject_in_rtl(tmp.path(), snippet);
    let manifest = CoreManifest::from_path(tmp.path().join("af-core.toml")).unwrap();
    let report = inspect_core(tmp.path(), &manifest).expect("inspect");
    report.issues.into_iter().map(|i| i.code).collect()
}

fn case_variant(s: &str, mask: &[bool]) -> String {
    let mut out = String::with_capacity(s.len());
    for (i, c) in s.chars().enumerate() {
        if mask.get(i).copied().unwrap_or(false) {
            out.extend(c.to_uppercase());
        } else {
            out.extend(c.to_lowercase());
        }
    }
    out
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    /// Any case variant of `mmcm` (a known vendor marker) triggers
    /// AF_PORTABLE_VENDOR_OR_CLOCK_MARKER. The matcher lowercases the
    /// source text first, so case-insensitivity is the contract.
    #[test]
    fn mmcm_case_variants_all_trigger(mask in proptest::collection::vec(any::<bool>(), 4..5)) {
        let marker = case_variant("mmcm", &mask);
        let snippet = format!("  wire {marker}_clk;");
        let codes = issues_after(&snippet);
        prop_assert!(
            codes.iter().any(|c| c == "AF_PORTABLE_VENDOR_OR_CLOCK_MARKER"),
            "marker `{marker}` should trigger AF_PORTABLE_VENDOR_OR_CLOCK_MARKER; got {codes:?}"
        );
    }

    /// Case variants of `ddr3` trigger AF_PORTABLE_HARD_PHY_BLOCK.
    #[test]
    fn ddr3_case_variants_all_trigger(mask in proptest::collection::vec(any::<bool>(), 4..5)) {
        let marker = case_variant("ddr3", &mask);
        let snippet = format!("  wire {marker}_clk;");
        let codes = issues_after(&snippet);
        prop_assert!(
            codes.iter().any(|c| c == "AF_PORTABLE_HARD_PHY_BLOCK"),
            "marker `{marker}` should trigger AF_PORTABLE_HARD_PHY_BLOCK; got {codes:?}"
        );
    }
}

#[test]
fn random_unrelated_keyword_does_not_trigger() {
    // Sanity: a freshly-coined identifier should not collide with any
    // marker in the inspector tables.
    let codes = issues_after("  wire unrelated_random_signal_zzz;");
    let triggered: Vec<&str> = codes
        .iter()
        .filter(|c| c.starts_with("AF_PORTABLE_"))
        .map(|c| c.as_str())
        .collect();
    assert!(
        triggered.is_empty(),
        "neutral identifier must not trip any AF_PORTABLE_* rule: {triggered:?}"
    );
}
