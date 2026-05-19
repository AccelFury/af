// SPDX-License-Identifier: AGPL-3.0-or-later
//
// `af-host` is the AGPL-isolated host-bringup shell. The crate ships a
// single subcommand (`ping`) that proves the binary builds, parses, and
// prints the expected lines. Functional host-link work lives outside
// this repo.

use assert_cmd::Command;
use predicates::str::contains;

fn af_host() -> Command {
    Command::cargo_bin("af-host").expect("cargo bin `af-host` builds")
}

#[test]
fn ping_prints_ready_lines() {
    af_host()
        .args(["ping"])
        .assert()
        .success()
        .stdout(contains("af-host ready"))
        .stdout(contains("serial layer ready"));
}

#[test]
fn no_subcommand_fails_with_clap_exit_code_2() {
    let out = af_host().output().expect("execute");
    assert_eq!(
        out.status.code(),
        Some(2),
        "missing subcommand must use clap exit code 2"
    );
}

#[test]
fn unknown_subcommand_fails_with_clap_exit_code_2() {
    let out = af_host().args(["nonexistent"]).output().expect("execute");
    assert_eq!(out.status.code(), Some(2));
}
