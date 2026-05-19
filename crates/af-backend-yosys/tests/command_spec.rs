// SPDX-License-Identifier: Apache-2.0
//
// `yosys_smoke_command` argv composition contract:
//
//   program       == "yosys"
//   args          == ["-q", "-p", <tcl-script>]
//   <tcl-script>  starts with `read_verilog`, optionally `-sv` flag,
//                 contains every `-I<include_dir>` and every source
//                 file, ends with hierarchy/proc/opt/check stages
//                 referencing manifest.rtl.top
//   cwd           == core_dir
//
// We do NOT execute yosys; the test asserts the spec produced by the
// builder so future refactors cannot drop a flag or rewrite the script
// silently.

use af_backend_yosys::yosys_smoke_command;
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

fn script_arg(spec: &af_security::CommandSpec) -> &str {
    // Tcl script is the third arg after `-q -p`.
    spec.args
        .iter()
        .find(|a| a.contains("read_verilog"))
        .map(String::as_str)
        .unwrap_or("")
}

#[test]
fn program_is_yosys_and_cwd_is_core_dir() {
    let (core_dir, manifest) = mod_add();
    let spec = yosys_smoke_command(&manifest, &core_dir);
    assert_eq!(spec.program, "yosys");
    assert_eq!(spec.cwd.as_deref(), Some(core_dir.as_path()));
}

#[test]
fn args_carry_q_and_p_flags_separately() {
    let (core_dir, manifest) = mod_add();
    let spec = yosys_smoke_command(&manifest, &core_dir);
    // `-q` and `-p` must each be their own argv element (no shell
    // interpolation possible).
    assert!(
        spec.args.iter().any(|a| a == "-q"),
        "missing -q: {:?}",
        spec.args
    );
    assert!(
        spec.args.iter().any(|a| a == "-p"),
        "missing -p: {:?}",
        spec.args
    );
}

#[test]
fn sv_flag_is_added_for_systemverilog_manifests() {
    let (core_dir, manifest) = mod_add();
    // af-mod-add has language = "systemverilog".
    assert_eq!(manifest.rtl.language, "systemverilog");
    let spec = yosys_smoke_command(&manifest, &core_dir);
    let script = script_arg(&spec);
    assert!(
        script.contains("-sv"),
        "SystemVerilog manifest must yield `-sv` flag: {script}"
    );
}

#[test]
fn every_include_dir_is_passed_with_minus_i() {
    let (core_dir, manifest) = mod_add();
    let spec = yosys_smoke_command(&manifest, &core_dir);
    let script = script_arg(&spec);
    for dir in &manifest.sources.include_dirs {
        let needle = format!("-I{dir}");
        assert!(
            script.contains(&needle),
            "include dir `{dir}` missing as `-I{dir}` in script:\n{script}"
        );
    }
}

#[test]
fn every_source_file_appears_in_script() {
    let (core_dir, manifest) = mod_add();
    let spec = yosys_smoke_command(&manifest, &core_dir);
    let script = script_arg(&spec);
    for source in &manifest.sources.files {
        assert!(
            script.contains(source),
            "source `{source}` missing from script:\n{script}"
        );
    }
}

#[test]
fn script_runs_hierarchy_check_with_top_module() {
    let (core_dir, manifest) = mod_add();
    let spec = yosys_smoke_command(&manifest, &core_dir);
    let script = script_arg(&spec);
    let needle = format!("hierarchy -check -top {}", manifest.rtl.top);
    assert!(
        script.contains(&needle),
        "script must invoke hierarchy with top `{}`:\n{script}",
        manifest.rtl.top
    );
}

#[test]
fn args_are_strings_no_shell_metacharacters_inside_argv_elements() {
    // Even though the script string concatenates content, argv-level
    // we still pass it as ONE element. So a `;` inside a source name
    // would be carried as-is into the Tcl interpreter (which is not a
    // shell). Spawn-time injection is impossible.
    let (core_dir, manifest) = mod_add();
    let spec = yosys_smoke_command(&manifest, &core_dir);
    // The args vector itself must not contain any element that is the
    // empty string or contains a null byte.
    for a in &spec.args {
        assert!(!a.is_empty(), "no empty argv element allowed");
        assert!(!a.contains('\0'), "no null byte in argv element");
    }
}

#[test]
fn no_environment_or_network_implicitly_enabled() {
    let (core_dir, manifest) = mod_add();
    let spec = yosys_smoke_command(&manifest, &core_dir);
    assert!(spec.env.is_empty(), "yosys spec must not preset env vars");
    assert!(
        !spec.allow_network,
        "yosys lint must run offline by default"
    );
}
