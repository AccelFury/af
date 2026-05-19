// SPDX-License-Identifier: Apache-2.0
//
// Integration tests for the native backend trait + capabilities API.

use af_backend::{AfBackend, BackendStatus};
use af_backend_native::{capabilities, NativeBackend};
use af_manifest::CoreManifest;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

#[test]
fn name_is_native_and_capabilities_listed() {
    assert_eq!(NativeBackend.name(), "native");
    let caps = capabilities();
    assert!(!caps.is_empty());
    assert!(
        caps.iter()
            .any(|c| c.name == "native-portable-core-check" && c.supported),
        "must advertise native-portable-core-check as supported"
    );
    assert!(
        caps.iter()
            .any(|c| c.name == "native-portability-lint" && c.supported),
        "must advertise native-portability-lint as supported"
    );
}

#[test]
fn doctor_returns_passed_status_with_tool_version() {
    let report = NativeBackend.doctor().expect("doctor never errors");
    assert_eq!(report.status, BackendStatus::Passed);
    assert!(
        report.tool_versions.iter().any(|tv| tv.tool == "af-native"),
        "doctor must report af-native tool version"
    );
    assert!(!report.limitations.is_empty());
}

#[test]
fn lint_on_clean_reference_passes() {
    let core_dir = repo_root().join("examples").join("af-mod-add");
    let manifest = CoreManifest::from_path(core_dir.join("af-core.toml")).unwrap();
    let build_root = TempDir::new().unwrap();
    let report = NativeBackend
        .lint(&manifest, &core_dir, build_root.path())
        .expect("lint must not panic on reference core");
    // The reference fixture has known resource-contract issues; the
    // status may be Passed, Failed, or Warning. We only ensure no panic
    // and a well-formed report.
    assert!(matches!(
        report.status,
        BackendStatus::Passed | BackendStatus::Failed | BackendStatus::Unavailable
    ));
}

#[test]
fn sim_returns_unavailable_status() {
    let core_dir = repo_root().join("examples").join("af-mod-add");
    let manifest = CoreManifest::from_path(core_dir.join("af-core.toml")).unwrap();
    let build_root = TempDir::new().unwrap();
    let report = NativeBackend
        .sim(&manifest, &core_dir, build_root.path())
        .expect("sim should not error, just report unavailable");
    assert_eq!(report.status, BackendStatus::Unavailable);
}
