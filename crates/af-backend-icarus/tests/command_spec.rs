// SPDX-License-Identifier: Apache-2.0
//
// `icarus_lint_command` / `icarus_sim_compile_command` argv composition.

use af_backend_icarus::{icarus_lint_command, icarus_sim_compile_command};
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
fn lint_program_is_iverilog_with_top_module_flag() {
    let (core_dir, manifest) = mod_add();
    let spec = icarus_lint_command(&manifest, &core_dir);
    assert_eq!(spec.program, "iverilog");
    assert_eq!(spec.cwd.as_deref(), Some(core_dir.as_path()));
    // `-s <top>` pair must be present, and `<top>` immediately follows.
    let s_idx = spec
        .args
        .iter()
        .position(|a| a == "-s")
        .expect("-s flag missing");
    assert_eq!(spec.args.get(s_idx + 1), Some(&manifest.rtl.top));
}

#[test]
fn lint_args_include_wall_and_tnull() {
    let (core_dir, manifest) = mod_add();
    let spec = icarus_lint_command(&manifest, &core_dir);
    assert!(spec.args.iter().any(|a| a == "-Wall"));
    assert!(spec.args.iter().any(|a| a == "-tnull"));
}

#[test]
fn lint_args_carry_every_source_file_and_include_dir() {
    let (core_dir, manifest) = mod_add();
    let spec = icarus_lint_command(&manifest, &core_dir);
    for source in &manifest.sources.files {
        assert!(
            spec.args.iter().any(|a| a == source),
            "source `{source}` missing from lint args: {:?}",
            spec.args
        );
    }
    for dir in &manifest.sources.include_dirs {
        let needle = format!("-I{dir}");
        assert!(
            spec.args.iter().any(|a| a == &needle),
            "include dir `{dir}` missing as `-I{dir}` argv element"
        );
    }
}

#[test]
fn sim_compile_output_is_in_argv_as_separate_arg() {
    let (core_dir, manifest) = mod_add();
    let out = core_dir.join("build/sim.vvp");
    let spec = icarus_sim_compile_command(&manifest, &core_dir, &out);
    let o_idx = spec.args.iter().position(|a| a == "-o").expect("-o flag");
    assert_eq!(
        spec.args.get(o_idx + 1).map(String::as_str),
        Some(out.display().to_string().as_str())
    );
}

#[test]
fn sim_compile_dedupes_sources_when_overlap_with_testbench() {
    let (core_dir, manifest) = mod_add();
    let out = core_dir.join("build/sim.vvp");
    let spec = icarus_sim_compile_command(&manifest, &core_dir, &out);
    // Each source must appear at most once in argv.
    let mut counts = std::collections::BTreeMap::<&String, usize>::new();
    for a in &spec.args {
        *counts.entry(a).or_default() += 1;
    }
    for (a, n) in &counts {
        if a.ends_with(".sv") || a.ends_with(".v") {
            assert_eq!(
                *n, 1,
                "source `{a}` appears {n} times in argv; must be deduplicated"
            );
        }
    }
}
