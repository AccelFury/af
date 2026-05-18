// SPDX-License-Identifier: Apache-2.0
//
// Emit JSON schema for `AfReport` to stdout. Used to regenerate
// `schemas/af-report.schema.json`. Run via:
//
//     cargo run --quiet --example dump_schema -p af-report \
//         > schemas/af-report.schema.json
//
// Regenerate after any change to public types under `crates/af-report::`
// or to public types they reference (`af-backend`, `af-manifest`,
// `af-security`). The `.claude/skills/af-cli-contract-guard/check.sh`
// hook flags missing regenerations.

use af_report::AfReport;

fn main() {
    let schema = schemars::schema_for!(AfReport);
    let pretty = serde_json::to_string_pretty(&schema)
        .expect("AfReport schema serialises to JSON deterministically");
    println!("{pretty}");
}
