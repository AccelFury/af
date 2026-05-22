// SPDX-License-Identifier: Apache-2.0
//
// `verilator_lint_command` / `verilator_smoke_command` argv composition.

use af_backend_verilator::{verilator_lint_command, verilator_smoke_command};
use af_manifest::CoreManifest;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn mod_add() -> (PathBuf, CoreManifest) {
    let core_dir = repo_root().join("examples").join("af-mod-add");
    let manifest = CoreManifest::from_path(core_dir.join("af-core.toml")).expect("manifest");
    (core_dir, manifest)
}

#[test]
fn lint_program_is_verilator_with_lint_only_and_top_module() {
    let (core_dir, manifest) = mod_add();
    let spec = verilator_lint_command(&manifest, &core_dir);
    assert_eq!(spec.program, "verilator");
    assert_eq!(spec.cwd.as_deref(), Some(core_dir.as_path()));
    assert!(spec.args.iter().any(|a| a == "--lint-only"));
    let idx = spec
        .args
        .iter()
        .position(|a| a == "--top-module")
        .expect("--top-module flag");
    assert_eq!(spec.args.get(idx + 1), Some(&manifest.rtl.top));
}

#[test]
fn lint_args_include_every_source_and_include_dir() {
    let (core_dir, manifest) = mod_add();
    let spec = verilator_lint_command(&manifest, &core_dir);
    for source in &manifest.sources.files {
        assert!(
            spec.args.iter().any(|a| a == source),
            "source `{source}` missing from verilator lint argv"
        );
    }
    for dir in &manifest.sources.include_dirs {
        let needle = format!("-I{dir}");
        assert!(
            spec.args.iter().any(|a| a == &needle),
            "include dir `-I{dir}` missing in argv"
        );
    }
}

#[test]
fn smoke_command_picks_testbench_top_when_present() {
    let (core_dir, manifest) = mod_add();
    let spec = verilator_smoke_command(&manifest, &core_dir);
    let idx = spec
        .args
        .iter()
        .position(|a| a == "--top-module")
        .expect("--top-module");
    let chosen = spec.args.get(idx + 1).cloned().unwrap_or_default();
    let expected = manifest
        .testbenches
        .first()
        .map(|tb| tb.top.clone())
        .unwrap_or_else(|| manifest.rtl.top.clone());
    assert_eq!(chosen, expected);
}

#[test]
fn smoke_command_prefers_verilator_specific_testbench() {
    let (core_dir, mut manifest) = mod_add();
    manifest.testbenches[0].backend = Some("icarus".to_string());
    let mut verilator_tb = manifest.testbenches[0].clone();
    verilator_tb.name = "verilator_smoke".to_string();
    verilator_tb.backend = Some("verilator".to_string());
    verilator_tb.top = "tb_for_verilator".to_string();
    verilator_tb.sources = vec!["tb/verilator_only.sv".to_string()];
    manifest.testbenches.push(verilator_tb);

    let spec = verilator_smoke_command(&manifest, &core_dir);

    let idx = spec
        .args
        .iter()
        .position(|a| a == "--top-module")
        .expect("--top-module");
    assert_eq!(
        spec.args.get(idx + 1).map(String::as_str),
        Some("tb_for_verilator")
    );
    assert!(
        spec.args.iter().any(|arg| arg == "tb/verilator_only.sv"),
        "selected Verilator testbench source missing from argv: {:?}",
        spec.args
    );
}

#[test]
fn no_shell_metacharacters_can_smuggle_into_argv() {
    let (core_dir, manifest) = mod_add();
    let spec = verilator_lint_command(&manifest, &core_dir);
    for a in &spec.args {
        assert!(!a.contains('\0'), "null byte in argv: {a}");
    }
    // Program must not be a shell.
    assert_eq!(spec.program, "verilator");
}
