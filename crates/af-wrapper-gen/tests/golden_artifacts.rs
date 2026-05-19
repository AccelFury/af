// SPDX-License-Identifier: Apache-2.0
//
// Insta-snapshots for wrapper generators. Locks the byte-level content
// of FuseSoC, LiteX, and IP-XACT artifacts against the af-mod-add
// reference manifest. A drift in any string here is a public-contract
// change — review with `cargo insta review`.
//
// Why snapshots and not goldens-on-disk? insta has structured diffs and
// auto-accept tooling; we already depend on it in af-backend-fusesoc
// and af-report. The behaviour under `cargo insta test --check` is to
// fail without auto-accepting, which is the CI gate we want.

mod common;

use af_manifest::CoreManifest;
use af_wrapper_gen::{generate_ipxact_skeleton, generate_wrapper, WrapperTarget};
use common::clone_example;

#[test]
fn fusesoc_artifact_matches_snapshot() {
    let project = clone_example("af-mod-add");
    let build_root = tempfile::TempDir::new().unwrap();
    let report = generate_wrapper(
        project.path(),
        build_root.path(),
        WrapperTarget::FuseSoc,
        None,
    )
    .expect("fusesoc wrapper");
    assert_eq!(report.artifacts.len(), 1);
    let content = std::fs::read_to_string(&report.artifacts[0]).unwrap();
    insta::assert_snapshot!("fusesoc_af_mod_add", content);
}

#[test]
fn litex_artifact_matches_snapshot() {
    let project = clone_example("af-mod-add");
    let build_root = tempfile::TempDir::new().unwrap();
    let report = generate_wrapper(
        project.path(),
        build_root.path(),
        WrapperTarget::LiteX,
        Some("digilent_arty_a7"),
    )
    .expect("litex wrapper");
    let content = std::fs::read_to_string(&report.artifacts[0]).unwrap();
    insta::assert_snapshot!("litex_af_mod_add_arty_a7", content);
}

#[test]
fn ipxact_skeleton_matches_snapshot() {
    let project = clone_example("af-mod-add");
    let manifest = CoreManifest::from_path(project.path().join("af-core.toml")).unwrap();
    let skel = generate_ipxact_skeleton(&manifest, None);
    insta::assert_snapshot!("ipxact_af_mod_add", skel.content);
}
