// SPDX-License-Identifier: Apache-2.0
//
// Per-contract architecture checks: CDC, backend variants, verification
// gates, constructor metadata.

mod common;

use af_architecture::check_architecture;
use common::clone_example;
use std::fs;

#[test]
fn af_mod_add_has_no_cdc_contract_issues() {
    // af-mod-add declares a single clock domain so there is no CDC
    // contract obligation. The resource-contract check intentionally
    // fires on this example (RTL contains `ram`/`fifo` markers without
    // declared resource contracts in the manifest) — that is exercised
    // in `resource_contract_marker_in_rtl_is_flagged` below.
    let tmp = clone_example("af-mod-add");
    let report = check_architecture(tmp.path()).expect("architecture check runs");

    let cdc: Vec<&str> = report
        .issues
        .iter()
        .filter(|i| i.code == "AF_CDC_CONTRACT_MISSING")
        .map(|i| i.code.as_str())
        .collect();
    assert!(
        cdc.is_empty(),
        "single-clock manifest must not raise CDC issues: {cdc:?}"
    );
}

#[test]
fn resource_contract_marker_in_rtl_is_flagged() {
    let tmp = clone_example("af-mod-add");
    let report = check_architecture(tmp.path()).expect("architecture check runs");
    let codes: Vec<&str> = report.issues.iter().map(|i| i.code.as_str()).collect();
    assert!(
        codes.contains(&"AF_RESOURCE_CONTRACT_MISSING"),
        "ram/fifo/dsp markers in RTL without manifest resource contracts must raise AF_RESOURCE_CONTRACT_MISSING; got {codes:?}"
    );
}

#[test]
fn missing_verification_evidence_is_flagged() {
    // Edit the manifest to point at an evidence path that does not exist
    // on disk. The verification-gates check must emit
    // AF_VERIFICATION_EVIDENCE_MISSING.
    let tmp = clone_example("af-mod-add");
    let manifest = tmp.path().join("af-core.toml");
    let original = fs::read_to_string(&manifest).unwrap();
    let augmented = format!(
        "{original}\n[[verification_required]]\nkind = \"simulation\"\nevidence = \"reports/never-existed.json\"\n"
    );
    fs::write(&manifest, augmented).unwrap();

    let report = check_architecture(tmp.path()).expect("architecture check runs");
    let codes: Vec<&str> = report.issues.iter().map(|i| i.code.as_str()).collect();
    assert!(
        codes.contains(&"AF_VERIFICATION_EVIDENCE_MISSING"),
        "missing evidence path must raise AF_VERIFICATION_EVIDENCE_MISSING; got {codes:?}"
    );
}

#[test]
fn verification_gate_without_evidence_path_emits_planned_warning() {
    let tmp = clone_example("af-mod-add");
    let manifest = tmp.path().join("af-core.toml");
    let original = fs::read_to_string(&manifest).unwrap();
    let augmented = format!("{original}\n[[verification_required]]\nkind = \"formal-occupancy\"\n");
    fs::write(&manifest, augmented).unwrap();

    let report = check_architecture(tmp.path()).expect("architecture check runs");
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("AF_VERIFICATION_EVIDENCE_PLANNED")),
        "evidence-less gate must produce AF_VERIFICATION_EVIDENCE_PLANNED warning; got {:?}",
        report.warnings
    );
}

#[test]
fn report_serializes_to_json_round_trip() {
    let tmp = clone_example("af-mod-add");
    let report = check_architecture(tmp.path()).expect("architecture check runs");
    let json = serde_json::to_string(&report).expect("serialize");
    let back: serde_json::Value = serde_json::from_str(&json).expect("parse");
    let kind = back["status"].as_str().expect("status string");
    assert!(
        matches!(kind, "passed" | "warning" | "failed"),
        "status must be one of passed/warning/failed: {kind}"
    );
}
