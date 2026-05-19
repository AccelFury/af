// SPDX-License-Identifier: Apache-2.0
//
// `create_signoff_plan` walks a `ProjectClass` and emits the required
// check rows. The mapping is part of the public contract — anything
// that adds or removes a check id is a SemVer surface change.

mod common;

use af_complexity::ProjectClass;
use af_signoff::{create_signoff_plan, SignoffCheck};
use common::clone_example;

fn ids(checks: &[SignoffCheck]) -> Vec<&str> {
    checks.iter().map(|c| c.id.as_str()).collect()
}

#[test]
fn simple_portable_has_three_planned_checks() {
    let tmp = clone_example("af-mod-add");
    let report = create_signoff_plan(
        tmp.path().join("af-core.toml"),
        Some(ProjectClass::SimplePortable),
        None,
    )
    .expect("signoff plan");
    let got = ids(&report.checks);
    assert_eq!(
        got,
        vec!["manifest-check", "native-portable-lint", "smoke-sim"],
        "simple-portable plan drifted"
    );
    assert!(report.checks.iter().all(|c| c.required));
    assert!(report.checks.iter().all(|c| c.status == "planned"));
}

#[test]
fn composite_portable_adds_dependency_and_compatibility() {
    let tmp = clone_example("af-mod-add");
    let report = create_signoff_plan(
        tmp.path().join("af-core.toml"),
        Some(ProjectClass::CompositePortable),
        None,
    )
    .expect("signoff plan");
    let got = ids(&report.checks);
    assert!(got.contains(&"dependency-check"));
    assert!(got.contains(&"compatibility-check"));
}

#[test]
fn complex_vendor_aware_adds_formal_and_backend_equivalence() {
    let tmp = clone_example("af-mod-add");
    let report = create_signoff_plan(
        tmp.path().join("af-core.toml"),
        Some(ProjectClass::ComplexVendorAware),
        None,
    )
    .expect("signoff plan");
    let got = ids(&report.checks);
    for required in [
        "formal-targets",
        "backend-equivalence",
        "cdc-rdc-plan",
        "timing-plan",
        "resource-plan",
        "constructor-export",
    ] {
        assert!(got.contains(&required), "missing {required}: {got:?}");
    }
}

#[test]
fn system_platform_drops_unit_checks_for_platform_ones() {
    let tmp = clone_example("af-mod-add");
    let report = create_signoff_plan(
        tmp.path().join("af-core.toml"),
        Some(ProjectClass::SystemPlatform),
        None,
    )
    .expect("signoff plan");
    let got = ids(&report.checks);
    assert!(got.contains(&"platform-constraints"));
    assert!(got.contains(&"security-production-flow"));
    assert!(
        !got.contains(&"manifest-check"),
        "system-platform must not require unit-level manifest-check"
    );
}

#[test]
fn product_stack_focuses_on_release_artifacts() {
    let tmp = clone_example("af-mod-add");
    let report = create_signoff_plan(
        tmp.path().join("af-core.toml"),
        Some(ProjectClass::ProductStack),
        None,
    )
    .expect("signoff plan");
    let got = ids(&report.checks);
    for required in [
        "catalog-export",
        "version-matrix",
        "compatibility-matrix",
        "release-reports",
        "known-limitations",
    ] {
        assert!(got.contains(&required), "missing {required}: {got:?}");
    }
}

#[test]
fn board_request_on_core_level_class_emits_warning() {
    let tmp = clone_example("af-mod-add");
    let report = create_signoff_plan(
        tmp.path().join("af-core.toml"),
        Some(ProjectClass::SimplePortable),
        Some("digilent_arty_a7".to_string()),
    )
    .expect("signoff plan");
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("Board-specific signoff requested")),
        "board on core-level class must warn: {:?}",
        report.warnings
    );
}

#[test]
fn signoff_check_kinds_are_documented_categories() {
    let tmp = clone_example("af-mod-add");
    let report = create_signoff_plan(
        tmp.path().join("af-core.toml"),
        Some(ProjectClass::ComplexVendorAware),
        None,
    )
    .expect("signoff plan");
    for c in &report.checks {
        assert!(
            matches!(
                c.kind.as_str(),
                "verification" | "security" | "implementation" | "product"
            ),
            "unknown check kind for {}: {}",
            c.id,
            c.kind
        );
    }
}
