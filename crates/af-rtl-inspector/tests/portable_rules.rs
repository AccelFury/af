// SPDX-License-Identifier: Apache-2.0
//
// Integration tests for `inspect_core` portable-rule detection.
//
// Each rule has a positive case (clean RTL, no issue raised) and a
// negative case (marker injected, code surfaces). We use a TempDir
// clone of af-mod-add as the clean baseline and overwrite individual
// files (or add new ones into rtl/common/) to trip each detector.

use af_manifest::CoreManifest;
use af_rtl_inspector::inspect_core;
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

/// Append `snippet` to the af-reset-sync RTL file (a Verilog-2001
/// source that the portable-policy walker DOES traverse). Injecting
/// inside `module ... endmodule` keeps the file structurally valid.
fn inject_marker_in_rtl(project: &Path, snippet: &str) {
    let path = project.join("rtl/af_reset_sync.v");
    let text = fs::read_to_string(&path).unwrap();
    // Insert the snippet just before the final `endmodule`.
    let injected = if let Some(idx) = text.rfind("endmodule") {
        let (head, tail) = text.split_at(idx);
        format!("{head}\n// injected marker\n{snippet}\n{tail}")
    } else {
        format!("{text}\n{snippet}\n")
    };
    fs::write(&path, injected).unwrap();
}

fn issues_of_clean_example() -> Vec<String> {
    let tmp = clone_example("af-reset-sync");
    let manifest = CoreManifest::from_path(tmp.path().join("af-core.toml")).unwrap();
    let report = inspect_core(tmp.path(), &manifest).expect("inspect");
    report.issues.into_iter().map(|i| i.code).collect()
}

fn issues_after_injection(snippet: &str) -> Vec<String> {
    let tmp = clone_example("af-reset-sync");
    inject_marker_in_rtl(tmp.path(), snippet);
    let manifest = CoreManifest::from_path(tmp.path().join("af-core.toml")).unwrap();
    let report = inspect_core(tmp.path(), &manifest).expect("inspect");
    report.issues.into_iter().map(|i| i.code).collect()
}

#[test]
fn clean_example_does_not_raise_vendor_or_clock_marker() {
    let codes = issues_of_clean_example();
    assert!(
        !codes
            .iter()
            .any(|c| c == "AF_PORTABLE_VENDOR_OR_CLOCK_MARKER"),
        "clean af-mod-add must not raise vendor/clock marker: {codes:?}"
    );
}

#[test]
fn ddr_marker_in_common_layer_raises_hard_phy_block() {
    // Real instance declaration (not a comment) so strip_comments
    // does not eat the token.
    let codes = issues_after_injection("  wire ddr3_clk; assign ddr3_clk = 1'b0;");
    assert!(
        codes.iter().any(|c| c == "AF_PORTABLE_HARD_PHY_BLOCK"),
        "DDR marker must raise AF_PORTABLE_HARD_PHY_BLOCK: {codes:?}"
    );
}

#[test]
fn mmcm_marker_in_common_layer_raises_vendor_or_clock_marker() {
    let codes = issues_after_injection("  MMCM_ADV mmcm_inst (...);");
    assert!(
        codes
            .iter()
            .any(|c| c == "AF_PORTABLE_VENDOR_OR_CLOCK_MARKER"),
        "MMCM_ADV must raise AF_PORTABLE_VENDOR_OR_CLOCK_MARKER: {codes:?}"
    );
}

#[test]
fn pll_clkwiz_raises_vendor_or_clock_marker() {
    let codes = issues_after_injection("  clk_wiz_0 clkgen (...);");
    assert!(codes
        .iter()
        .any(|c| c == "AF_PORTABLE_VENDOR_OR_CLOCK_MARKER"));
}

#[test]
fn axi_only_marker_is_flagged_in_common_layer() {
    // Real AXI-stream signal names trip the AXI-only check.
    let codes = issues_after_injection("  wire tvalid; wire tready;");
    assert!(
        codes.iter().any(|c| c == "AF_PORTABLE_AXI_ONLY_MARKER"),
        "AXI-only marker must raise AF_PORTABLE_AXI_ONLY_MARKER: {codes:?}"
    );
}

#[test]
fn encrypted_netlist_pragma_is_flagged() {
    // Real preprocessor pragma, not a comment — strip_comments leaves
    // `pragma` directives in the source body.
    let codes = issues_after_injection("`pragma protect begin_protected");
    assert!(
        codes.iter().any(|c| c == "AF_PORTABLE_ENCRYPTED_NETLIST"),
        "pragma protect must raise AF_PORTABLE_ENCRYPTED_NETLIST: {codes:?}"
    );
}

#[test]
fn implicit_reset_via_initial_block_is_flagged() {
    // af-reset-sync ships its `initial` inside a `synthesis
    // translate_off` guard. We have to strip the guard first so the
    // unguarded-initial detector fires.
    let tmp = clone_example("af-reset-sync");
    let rtl = tmp.path().join("rtl/af_reset_sync.v");
    let text = fs::read_to_string(&rtl).unwrap();
    let unsafe_text = text
        .replace("// synthesis translate_off", "")
        .replace("// synthesis translate_on", "");
    fs::write(&rtl, unsafe_text).unwrap();

    let manifest = CoreManifest::from_path(tmp.path().join("af-core.toml")).unwrap();
    let report = inspect_core(tmp.path(), &manifest).expect("inspect");
    let codes: Vec<String> = report.issues.into_iter().map(|i| i.code).collect();
    assert!(
        codes.iter().any(|c| c == "AF_PORTABLE_IMPLICIT_RESET"),
        "unguarded initial must raise AF_PORTABLE_IMPLICIT_RESET: {codes:?}"
    );
}

#[test]
fn multiple_markers_all_surface() {
    let codes =
        issues_after_injection("  wire mmcm_clk;\n  wire tvalid;\n`pragma protect begin_protected");
    let expected = [
        "AF_PORTABLE_VENDOR_OR_CLOCK_MARKER",
        "AF_PORTABLE_AXI_ONLY_MARKER",
        "AF_PORTABLE_ENCRYPTED_NETLIST",
    ];
    for code in expected {
        assert!(
            codes.iter().any(|c| c == code),
            "missing `{code}` when multiple markers co-occur: {codes:?}"
        );
    }
}

#[test]
fn inspect_report_records_scanned_files() {
    let tmp = clone_example("af-mod-add");
    let manifest = CoreManifest::from_path(tmp.path().join("af-core.toml")).unwrap();
    let report = inspect_core(tmp.path(), &manifest).expect("inspect");
    assert!(
        !report.scanned_files.is_empty(),
        "inspector must record scanned source files"
    );
}

#[test]
fn inspect_report_has_at_least_one_check_axis() {
    let tmp = clone_example("af-mod-add");
    let manifest = CoreManifest::from_path(tmp.path().join("af-core.toml")).unwrap();
    let report = inspect_core(tmp.path(), &manifest).expect("inspect");
    assert!(
        !report.checks.is_empty(),
        "inspector must record at least one check axis"
    );
}
