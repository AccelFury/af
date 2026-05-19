// SPDX-License-Identifier: Apache-2.0
//
// Pins `schemas/af-report.schema.json` to the in-process schemars
// reflection of `AfReport`. Any change to `AfReport`, `CommandPayload`,
// `Reproducibility`, `ReusableCoreMaturity`, or transitively the
// `af-backend` / `af-manifest` / `af-security` public types must be
// reflected in the committed schema file (regenerate via
// `cargo run --quiet --example dump_schema -p af-report`).
//
// Without this gate, breaking schema drift can be merged without
// downstream consumers noticing. With it, the test fails noisily and
// the author is forced to either:
//   * regenerate the file (additive schema change), or
//   * bump `schema_version` + add a CHANGELOG entry (breaking).

use af_report::AfReport;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

#[test]
fn af_report_schema_file_matches_schemars_reflection() {
    let committed = std::fs::read_to_string(repo_root().join("schemas/af-report.schema.json"))
        .expect("schemas/af-report.schema.json exists");
    let live_schema = schemars::schema_for!(AfReport);
    let live =
        serde_json::to_string_pretty(&live_schema).expect("schema serialises deterministically");

    // Normalise trailing whitespace on both sides — the example writer
    // uses println! which appends `\n`, but a hand-edited file might
    // disagree on EOF newlines.
    let committed_norm = committed.trim_end();
    let live_norm = live.trim_end();

    if committed_norm != live_norm {
        // Help the contributor: print the unified-style diff hint.
        eprintln!(
            "\n`schemas/af-report.schema.json` is out of date.\n\nRegenerate:\n  cargo run --quiet --example dump_schema -p af-report > schemas/af-report.schema.json\n\nIf the change is intentional and breaks downstream consumers, also bump `schema_version` in AfReport and add a CHANGELOG.md entry under [Unreleased]."
        );
    }
    assert_eq!(
        committed_norm, live_norm,
        "schemas/af-report.schema.json must match schemars::schema_for!(AfReport)"
    );
}

#[test]
fn committed_schema_advertises_documented_root_keys() {
    let committed = std::fs::read_to_string(repo_root().join("schemas/af-report.schema.json"))
        .expect("schema exists");
    let value: serde_json::Value = serde_json::from_str(&committed).expect("schema parses as JSON");

    // schema_version 0.1 is the public contract. Bumping requires a
    // CHANGELOG entry per repo hard-rule #6.
    let required = value["required"].as_array().expect("required array");
    let required_strs: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
    for key in [
        "generated_by",
        "kind",
        "report_version",
        "schema_version",
        "status",
    ] {
        assert!(
            required_strs.contains(&key),
            "schema must mark {key} as required: {required_strs:?}"
        );
    }
}
