// SPDX-License-Identifier: Apache-2.0
//
// `scaffold_project` and `scaffold_backend` emit deterministic file
// trees with the AccelFury generated-by stamp. The tests cover both
// supported project classes, the unsupported-class path, the
// duplicate-write path (AF_FILE_EXISTS), and the per-vendor constraint
// file naming.

use af_complexity::ProjectClass;
use af_template::{scaffold_backend, scaffold_project, TemplateError, GENERATED_BY_MARKER};
use tempfile::TempDir;

#[test]
fn scaffold_system_platform_emits_project_toml_and_dirs() {
    let tmp = TempDir::new().unwrap();
    let report = scaffold_project(tmp.path(), ProjectClass::SystemPlatform, Some("my-system"))
        .expect("scaffold runs");
    assert_eq!(report.status, "passed");
    assert!(tmp.path().join("af-project.toml").is_file());
    assert!(tmp.path().join("cores").is_dir());
    assert!(tmp.path().join("platforms").is_dir());
    assert!(tmp.path().join("constraints").is_dir());
    assert!(tmp.path().join("security").is_dir());
    let toml_text = std::fs::read_to_string(tmp.path().join("af-project.toml")).unwrap();
    assert!(
        toml_text.contains(GENERATED_BY_MARKER),
        "project toml must carry the marker"
    );
    assert!(toml_text.contains("name = \"my-system\""));
    assert!(toml_text.contains("class = \"system-platform\""));
}

#[test]
fn scaffold_product_stack_emits_product_toml() {
    let tmp = TempDir::new().unwrap();
    let report = scaffold_project(tmp.path(), ProjectClass::ProductStack, Some("my-stack"))
        .expect("scaffold runs");
    assert_eq!(report.status, "passed");
    assert!(tmp.path().join("af-product.toml").is_file());
    assert!(tmp.path().join("packages").is_dir());
    assert!(tmp.path().join("constructor_catalog").is_dir());
}

#[test]
fn scaffold_rejects_simple_portable() {
    let tmp = TempDir::new().unwrap();
    let err = scaffold_project(tmp.path(), ProjectClass::SimplePortable, None).unwrap_err();
    match err {
        TemplateError::UnsupportedClass { project_class } => {
            assert_eq!(project_class, ProjectClass::SimplePortable);
        }
        other => panic!("expected UnsupportedClass, got {other:?}"),
    }
}

#[test]
fn scaffold_refuses_to_overwrite_existing_file() {
    let tmp = TempDir::new().unwrap();
    scaffold_project(tmp.path(), ProjectClass::SystemPlatform, None).expect("first scaffold");
    let err = scaffold_project(tmp.path(), ProjectClass::SystemPlatform, None).unwrap_err();
    assert_eq!(err.code(), "AF_FILE_EXISTS");
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn unsupported_class_exit_code_is_validation() {
    let tmp = TempDir::new().unwrap();
    let err = scaffold_project(tmp.path(), ProjectClass::SimplePortable, None).unwrap_err();
    assert_eq!(err.code(), "AF_TEMPLATE_CLASS_UNSUPPORTED");
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn scaffold_backend_creates_per_vendor_constraint_files() {
    let cases = &[
        ("xilinx", "constraints.xdc"),
        ("intel", "constraints.sdc"),
        ("gowin", "constraints.cst"),
        ("lattice", "constraints.lpf"),
        ("unknown_vendor", "constraints.sdc"),
    ];
    for (vendor, expected) in cases {
        let tmp = TempDir::new().unwrap();
        let report =
            scaffold_backend(tmp.path(), vendor, "family-X").expect("scaffold_backend runs");
        assert_eq!(report.status, "passed");
        let constraint = tmp
            .path()
            .join("vendor")
            .join(vendor)
            .join("constraints")
            .join(expected);
        assert!(
            constraint.is_file(),
            "expected constraint file at {} for vendor {vendor}",
            constraint.display()
        );
    }
}

#[test]
fn scaffold_backend_creates_documented_subdirs() {
    let tmp = TempDir::new().unwrap();
    scaffold_backend(tmp.path(), "xilinx", "artix-7").expect("scaffold_backend runs");
    for sub in ["ram", "fifo", "dsp", "clock", "constraints", "tests"] {
        assert!(
            tmp.path().join("vendor").join("xilinx").join(sub).is_dir(),
            "missing vendor/xilinx/{sub}"
        );
    }
}

#[test]
fn every_emitted_file_carries_the_generated_by_marker() {
    let tmp = TempDir::new().unwrap();
    let report =
        scaffold_project(tmp.path(), ProjectClass::SystemPlatform, None).expect("scaffold runs");
    for artifact in &report.artifacts {
        let text = std::fs::read_to_string(artifact).unwrap();
        assert!(
            text.contains(GENERATED_BY_MARKER),
            "{} missing generated-by marker",
            artifact.display()
        );
    }
}
