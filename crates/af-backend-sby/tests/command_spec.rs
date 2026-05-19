// SPDX-License-Identifier: Apache-2.0
//
// `sby_command(file, core_dir)` argv composition. The implementation
// already has an inline test for the basic shape; this file pins the
// invariant from the integration side and adds edge cases.

use af_backend_sby::sby_command;
use std::path::Path;

#[test]
fn program_is_sby_and_args_pass_file_via_minus_f() {
    let spec = sby_command("formal/demo.sby", Path::new("core"));
    assert_eq!(spec.program, "sby");
    assert_eq!(spec.args, vec!["-f", "formal/demo.sby"]);
    assert_eq!(spec.cwd, Some("core".into()));
    assert!(spec.env.is_empty());
    assert!(!spec.allow_network);
}

#[test]
fn file_argument_is_carried_verbatim_through_argv() {
    // Even with characters that would be interpreted by a shell, sby
    // receives the path as a single literal argv element.
    let spec = sby_command("path with spaces and $shell.sby", Path::new("core"));
    assert_eq!(spec.args, vec!["-f", "path with spaces and $shell.sby"]);
}

#[test]
fn cwd_can_be_absolute_path() {
    let spec = sby_command("a.sby", Path::new("/tmp/core"));
    assert_eq!(spec.cwd, Some(Path::new("/tmp/core").to_path_buf()));
}
