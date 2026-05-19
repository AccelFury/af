// SPDX-License-Identifier: Apache-2.0
//
// `export_constructor_package` writes a fixed set of catalog files and
// surfaces AF_CONSTRUCTOR_EXPORT_INCOMPLETE when no manifest is present.

mod common;

use af_constructor_export::export_constructor_package;
use common::clone_example;
use tempfile::TempDir;

#[test]
fn export_from_core_dir_emits_files_under_output() {
    let core = clone_example("af-mod-add");
    let out = TempDir::new().unwrap();
    let report = export_constructor_package(core.path(), out.path(), false).expect("export runs");
    assert_eq!(report.status, "passed");
    assert!(!report.files.is_empty(), "must emit at least one file");
    for file in &report.files {
        assert!(file.is_file(), "{} must exist", file.display());
        assert!(
            file.starts_with(out.path()),
            "every file must live under output dir: {}",
            file.display()
        );
    }
    assert!(
        report.warnings.is_empty(),
        "core-with-manifest must not warn"
    );
}

#[test]
fn export_without_manifest_surfaces_incomplete_warning() {
    let empty = TempDir::new().unwrap();
    let out = TempDir::new().unwrap();
    let report = export_constructor_package(empty.path(), out.path(), false).expect("export runs");
    assert_eq!(report.status, "warning");
    assert!(report
        .warnings
        .iter()
        .any(|w| w.contains("AF_CONSTRUCTOR_EXPORT_INCOMPLETE")));
}

#[test]
fn export_files_are_valid_json() {
    let core = clone_example("af-mod-add");
    let out = TempDir::new().unwrap();
    let report = export_constructor_package(core.path(), out.path(), true).expect("export runs");
    for file in &report.files {
        let text = std::fs::read_to_string(file).unwrap();
        let _: serde_json::Value =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    }
}

#[test]
fn missing_output_parent_is_created() {
    let core = clone_example("af-mod-add");
    let out_parent = TempDir::new().unwrap();
    let nested_out = out_parent.path().join("deep/nest");
    let report = export_constructor_package(core.path(), &nested_out, false).expect("export runs");
    assert!(nested_out.is_dir(), "nested output dir must be created");
    assert!(!report.files.is_empty());
}
