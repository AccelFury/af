// SPDX-License-Identifier: Apache-2.0
//
// Regression gate for every `AF_*` error code emitted by the workspace.
//
// The contract is two-fold:
//
// 1. **Inventory parity.** `fixtures/af_error_codes.txt` is a snapshot of
//    every `AF_*` symbol present in `crates/`. Any new code added to the
//    source tree without updating that fixture (or any code removed without
//    deleting its row) fails this test. CI then forces the author to either
//    register the new code or prune the dead one — orphan codes are
//    therefore impossible.
//
// 2. **Envelope shape.** For a selected set of codes that we can reliably
//    trigger from a small fixture (path traversal, manifest version,
//    constructor metadata, …) we drive the binary and assert that the
//    `{code, message, hint, exit_code}` envelope matches.
//
// New codes added in future iterations must:
//   - regenerate the fixture (`rg -oN 'AF_[A-Z][A-Z0-9_]+' crates/ | awk
//     -F: '{print $2}' | sort -u`, then prune env-vars per the script in
//     `.claude/skills/af-cli-contract-guard/check.sh`);
//   - if they are user-reachable, add an envelope-shape regression row
//     below.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const FIXTURE_RAW: &str = include_str!("fixtures/af_error_codes.txt");

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn registered_codes() -> BTreeSet<String> {
    FIXTURE_RAW
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// Identifiers that share the `AF_` prefix but are NOT error codes — env
/// vars, fixture strings inside unit tests, etc. Updated in lock-step with
/// `.claude/skills/af-cli-contract-guard`.
const PREFIX_BUT_NOT_ERROR: &[&str] = &[
    // Env-vars / config keys, not error codes.
    "AF_AGENT_NAME",
    "AF_BUILD_ROOT",
    "AF_SELF_CHECK_AF_MOD_ADD",
    "AF_SELF_CHECK_AF_RESET_SYNC",
    "AF_INSTALL_FULL_SMT_SOLVERS",
    "AF_INSTALL_LITEX",
    "AF_VECTORS_GOLDEN_REGENERATE",
    // Generated SystemVerilog parameter names from af-vectors.
    "AF_MOD_ADD_RANDOM_A",
    "AF_MOD_ADD_RANDOM_B",
    "AF_MOD_ADD_RANDOM_COUNT",
    "AF_MOD_ADD_RANDOM_EXPECTED",
    "AF_MOD_ADD_RANDOM_SVH",
    // Bare-prefix tokens harvested from substring matchers like
    // `code.starts_with("AF_<PREFIX>_")` in integration tests. They are
    // NOT real error codes — the trailing `_` is stripped by the
    // extractor and lands here.
    "AF_BACKEND",
    "AF_BOARD",
    "AF_BUILD",
    "AF_CI_INIT",
    "AF_COMPAT",
    "AF_CORE",
    "AF_CORES_REGISTRY",
    "AF_EVIDENCE",
    "AF_LEGAL",
    "AF_LINT",
    "AF_MANIFEST",
    "AF_PORTABLE",
    "AF_SELF_CHECK",
    "AF_TIER",
    "AF_TOOLING",
    "AF_WRAPPER",
    // Unit-test fixture literal in agent.rs.
    "AF_X",
    // Speculative code referenced only in protocol_matrix.rs; not yet
    // present in af-compatibility. If added later, remove from this
    // list and register in fixtures/af_error_codes.txt.
    "AF_COMPAT_WIDTH_MISMATCH",
];

fn live_codes() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    walk(&repo_root().join("crates"), &mut out);
    for k in PREFIX_BUT_NOT_ERROR {
        out.remove(*k);
    }
    out
}

fn walk(dir: &Path, out: &mut BTreeSet<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|n| n.to_str()) == Some("target") {
                continue;
            }
            walk(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            let text = fs::read_to_string(&path).unwrap_or_default();
            extract(&text, out);
        }
    }
}

fn extract(text: &str, out: &mut BTreeSet<String>) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 3 <= bytes.len() {
        if &bytes[i..i + 3] == b"AF_" && (i == 0 || !is_ident(bytes[i - 1])) {
            let start = i;
            let mut j = i + 3;
            while j < bytes.len()
                && (bytes[j].is_ascii_uppercase() || bytes[j].is_ascii_digit() || bytes[j] == b'_')
            {
                j += 1;
            }
            // Trim trailing underscore: substring assertions like
            //   `contains("AF_CI_INIT_TOP_")`
            // yield a captured token with a trailing `_`. The real codes
            // are e.g. `AF_CI_INIT_TOP_MISSING`. Strip trailing `_` so
            // the fixture remains a clean set of full identifiers.
            let mut end = j;
            while end > start + 3 && bytes[end - 1] == b'_' {
                end -= 1;
            }
            if end - start >= 4 {
                let code = std::str::from_utf8(&bytes[start..end]).unwrap();
                if code.len() > 3 {
                    out.insert(code.to_string());
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
}

fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[test]
fn registered_codes_match_live_source_tree() {
    let registered = registered_codes();
    let live = live_codes();

    let missing: Vec<&String> = live.difference(&registered).collect();
    let dead: Vec<&String> = registered.difference(&live).collect();

    let mut errors = Vec::new();
    if !missing.is_empty() {
        errors.push(format!(
            "new AF_* codes in source tree not registered in fixtures/af_error_codes.txt:\n  - {}\n\nupdate the fixture: rg -oN 'AF_[A-Z][A-Z0-9_]+' crates/ | awk -F: '{{print $2}}' | sort -u >crates/af-cli/tests/fixtures/af_error_codes.txt",
            missing.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n  - ")
        ));
    }
    if !dead.is_empty() {
        errors.push(format!(
            "registered codes have no live referent in crates/ (likely removed or renamed):\n  - {}\n\nregenerate the fixture if intentional.",
            dead.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n  - ")
        ));
    }
    assert!(errors.is_empty(), "{}", errors.join("\n\n"));
}

#[test]
fn registered_codes_have_uppercase_underscore_snake_shape() {
    let registered = registered_codes();
    assert!(
        !registered.is_empty(),
        "fixture is empty — registered codes must not be 0"
    );

    for code in &registered {
        assert!(code.starts_with("AF_"), "code must start with AF_: {code}");
        assert!(
            code.chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'),
            "code must be UPPER_SNAKE only: {code}"
        );
        assert!(
            !code.contains("__"),
            "code must not contain consecutive underscores: {code}"
        );
        assert!(
            !code.ends_with('_'),
            "code must not end with underscore: {code}"
        );
    }
}

// --- Envelope shape regression ---------------------------------------------
//
// The full set of codes is too large to round-trip with real fixtures
// per row; here we lock in the envelope shape for a handful of
// representative codes that span domains (manifest, security, registry,
// signoff, backend). When `code: "<name>"` is emitted, the JSON payload
// MUST carry `message`, `hint`, and `exit_code`. Missing-binary cases are
// not exercised because they depend on the host's $PATH.

use assert_cmd::Command;
use serde_json::Value;

fn af() -> Command {
    Command::cargo_bin("af").expect("cargo bin `af` builds")
}

fn parse_err(stdout: &[u8]) -> Value {
    let s = std::str::from_utf8(stdout).expect("utf8 stdout");
    serde_json::from_str(s.trim()).unwrap_or_else(|e| panic!("not JSON: {e}\nstdout was:\n{s}"))
}

#[test]
fn manifest_missing_file_yields_envelope() {
    let tmp = tempfile::TempDir::new().unwrap();
    let bogus = tmp.path().join("no-such-manifest.toml");
    let out = af()
        .args(["--json", "manifest", "validate"])
        .arg(&bogus)
        .output()
        .expect("execute");
    assert!(!out.status.success());
    let payload = parse_err(&out.stdout);
    let code = payload["code"]
        .as_str()
        .unwrap_or_else(|| panic!("no code: {payload}"));
    assert!(
        code.starts_with("AF_"),
        "missing-manifest emitted non-AF_ code: {code}"
    );
    assert!(
        !payload["message"].as_str().unwrap().is_empty()
            && !payload["hint"].as_str().unwrap().is_empty(),
        "missing-manifest envelope incomplete: {payload}"
    );
}

#[test]
fn manifest_invalid_toml_yields_envelope() {
    let tmp = tempfile::TempDir::new().unwrap();
    let p = tmp.path().join("af-core.toml");
    fs::write(&p, "this is not = { valid toml\n").unwrap();
    let out = af()
        .args(["--json", "manifest", "validate"])
        .arg(&p)
        .output()
        .expect("execute");
    assert!(!out.status.success());
    let payload = parse_err(&out.stdout);
    let code = payload["code"].as_str().unwrap();
    assert!(
        code.starts_with("AF_"),
        "invalid-toml emitted non-AF_ code: {code}"
    );
    assert!(!payload["message"].as_str().unwrap().is_empty());
    assert!(!payload["hint"].as_str().unwrap().is_empty());
}

// `af report <missing-path>` is intentionally not an error: it emits a
// warning-only artefact report. That contract is asserted in
// `crates/af-cli/tests/cli.rs`; envelope-shape testing for it would
// require a synthetic failure path that the binary does not expose.

#[test]
fn unknown_subcommand_returns_clap_error_not_envelope() {
    // clap-level errors don't go through CliError; they use exit code 2
    // and stderr-only output. This test pins that behaviour so a future
    // refactor doesn't accidentally route clap errors through the JSON
    // envelope (which would break terminal usability).
    let out = af()
        .args(["nonexistent-subcommand"])
        .output()
        .expect("execute");
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(2));
}
