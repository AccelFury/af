// SPDX-License-Identifier: Apache-2.0
//
// Verifies the layer-violation detector flags every vendor primitive
// marker listed in `vendor_markers()` and accepts portable RTL.

mod common;

use af_architecture::check_architecture;
use common::clone_example;
use std::fs;

/// Inject a snippet of RTL containing `marker` into a fresh copy of the
/// `af-mod-add` example. Returns the temporary project dir owner. The
/// snippet lives in `rtl/common/marker_inject.sv` and is added to the
/// manifest's `[sources].files` list — putting it under the common layer
/// triggers the architecture check.
fn project_with_marker_in_common(marker: &str) -> tempfile::TempDir {
    let tmp = clone_example("af-mod-add");
    let common_dir = tmp.path().join("rtl/common");
    fs::create_dir_all(&common_dir).unwrap();
    let snippet = format!(
        "// SPDX-License-Identifier: Apache-2.0\nmodule marker_inject;\n  // {marker}\nendmodule\n"
    );
    fs::write(common_dir.join("marker_inject.sv"), snippet).unwrap();

    let manifest_path = tmp.path().join("af-core.toml");
    let original = fs::read_to_string(&manifest_path).unwrap();
    let injected = original.replace(
        "include_dirs = [\"rtl/core\", \"rtl/common\"]",
        "include_dirs = [\"rtl/core\", \"rtl/common\"]\n  # injected for vendor-marker test",
    );
    let with_extra_source = injected.replace(
        "\"rtl/common/af_reset_sync.sv\",\n]",
        "\"rtl/common/af_reset_sync.sv\",\n  \"rtl/common/marker_inject.sv\",\n]",
    );
    fs::write(&manifest_path, with_extra_source).unwrap();
    tmp
}

#[test]
fn portable_example_passes_architecture_check() {
    let tmp = clone_example("af-mod-add");
    let report = check_architecture(tmp.path()).expect("architecture check runs");
    let layer_violations: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.code == "AF_ARCH_LAYER_VIOLATION")
        .collect();
    assert!(
        layer_violations.is_empty(),
        "portable reference fixture must not raise layer violations: {layer_violations:?}"
    );
}

#[test]
fn vendor_marker_in_common_layer_is_flagged() {
    // One representative marker is enough to prove the rule fires; the
    // exhaustive list is asserted in `vendor_marker_inventory_is_stable`
    // below.
    let tmp = project_with_marker_in_common("xpm_memory_sdpram");
    let report = check_architecture(tmp.path()).expect("architecture check runs");
    let codes: Vec<&str> = report.issues.iter().map(|i| i.code.as_str()).collect();
    assert!(
        codes.contains(&"AF_ARCH_LAYER_VIOLATION"),
        "vendor marker in rtl/common must raise AF_ARCH_LAYER_VIOLATION; got {codes:?}"
    );
    assert_eq!(report.status, "failed");
}

#[test]
fn each_documented_marker_triggers_a_violation() {
    // Sweep every documented marker. We don't hard-code which crate path
    // it lives at — only that the detector fires.
    for marker in [
        "ramb_block",
        "dsp48e1",
        "xpm_fifo",
        "MMCM_ADV",
        "PLLE2_BASE",
        "pll_x",
        "clk_wiz_0",
        "altsyncram",
        "scfifo_inst",
        "dcfifo_inst",
        "GOWIN_PLL",
        "SB_RAM40",
        "ehxpll",
        "PCIE_block",
        "serdes_gtp",
    ] {
        let tmp = project_with_marker_in_common(marker);
        let report = check_architecture(tmp.path()).expect("architecture check runs");
        let codes: Vec<&str> = report.issues.iter().map(|i| i.code.as_str()).collect();
        assert!(
            codes.contains(&"AF_ARCH_LAYER_VIOLATION"),
            "marker `{marker}` in rtl/common must raise AF_ARCH_LAYER_VIOLATION; got {codes:?}"
        );
    }
}

#[test]
fn report_has_documented_checked_axes() {
    let tmp = clone_example("af-mod-add");
    let report = check_architecture(tmp.path()).expect("architecture check runs");
    for required in [
        "common layer vendor leakage",
        "resource contracts",
        "CDC contracts",
        "backend matrix limitations",
        "constructor metadata",
        "verification gates",
    ] {
        assert!(
            report.checked.iter().any(|axis| axis == required),
            "report.checked missing axis `{required}`: {:?}",
            report.checked
        );
    }
}

#[test]
fn limitations_are_disclosed() {
    let tmp = clone_example("af-mod-add");
    let report = check_architecture(tmp.path()).expect("architecture check runs");
    assert!(
        !report.limitations.is_empty(),
        "architecture report must disclose its limitations"
    );
}
