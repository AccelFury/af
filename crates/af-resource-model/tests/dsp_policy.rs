// SPDX-License-Identifier: Apache-2.0
//
// `strongest_policy` collapses a mix of declared backend_policy values
// onto a single string:
//
//     require_vendor > prefer_vendor > portable
//
// We exercise this transitively via `plan_resources`. Each test case
// declares two DSP entries with conflicting policies and asserts the
// reported aggregate policy.

mod common;

use af_resource_model::plan_resources;
use common::{append_to_manifest, clone_example};

fn dsp_fragment(name: &str, count: u32, policy: &str) -> String {
    format!(
        "\n[[resources.dsp]]\nname = \"{name}\"\nkind = \"mac\"\ncount = {count}\nbackend_policy = \"{policy}\"\n"
    )
}

fn dsp_policy(tmp: &std::path::Path) -> String {
    let report = plan_resources(tmp, None, None, None).expect("plan_resources runs");
    report
        .resources
        .get("dsp")
        .map(|r| r.policy.clone())
        .unwrap_or_else(|| panic!("dsp row missing: {:?}", report.resources))
}

#[test]
fn require_vendor_dominates_any_combination() {
    let tmp = clone_example("af-mod-add");
    append_to_manifest(tmp.path(), &dsp_fragment("dsp0", 4, "portable"));
    append_to_manifest(tmp.path(), &dsp_fragment("dsp1", 4, "require_vendor"));
    assert_eq!(dsp_policy(tmp.path()), "require_vendor");
}

#[test]
fn prefer_vendor_beats_portable() {
    let tmp = clone_example("af-mod-add");
    append_to_manifest(tmp.path(), &dsp_fragment("dsp0", 4, "portable"));
    append_to_manifest(tmp.path(), &dsp_fragment("dsp1", 4, "prefer_vendor"));
    assert_eq!(dsp_policy(tmp.path()), "prefer_vendor");
}

#[test]
fn all_portable_collapses_to_portable() {
    let tmp = clone_example("af-mod-add");
    append_to_manifest(tmp.path(), &dsp_fragment("dsp0", 4, "portable"));
    append_to_manifest(tmp.path(), &dsp_fragment("dsp1", 4, "portable"));
    assert_eq!(dsp_policy(tmp.path()), "portable");
}

#[test]
fn require_vendor_on_generic_target_raises_backend_required() {
    let tmp = clone_example("af-mod-add");
    append_to_manifest(tmp.path(), &dsp_fragment("dsp0", 4, "require_vendor"));
    let report = plan_resources(tmp.path(), None, None, None).expect("plan_resources runs");
    assert!(
        report
            .risks
            .iter()
            .any(|r| r.contains("AF_BACKEND_REQUIRED")),
        "require_vendor on generic target must add AF_BACKEND_REQUIRED risk; got {:?}",
        report.risks
    );
}

#[test]
fn dsp_estimate_sums_counts() {
    let tmp = clone_example("af-mod-add");
    append_to_manifest(tmp.path(), &dsp_fragment("dsp0", 5, "portable"));
    append_to_manifest(tmp.path(), &dsp_fragment("dsp1", 7, "portable"));
    let report = plan_resources(tmp.path(), None, None, None).expect("plan_resources runs");
    let dsp = report.resources.get("dsp").expect("dsp row");
    assert_eq!(dsp.estimated, 12, "dsp counts must sum");
}
