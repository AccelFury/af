// SPDX-License-Identifier: Apache-2.0
//
// `af core verify --tier <community|verified-package|enterprise>` walks
// the maturity rows produced by `core report` and asserts every required
// row is `supported`. The tier → required-rows table is pinned in
// `crates/af-cli/src/main.rs::tier_required_rows`:
//
//   community         → manifest_contract, source_portability
//   verified-package  → community ∪ {open_source_tool_evidence,
//                                    wrapper_package_compatibility,
//                                    docker_ci_cd_evidence}
//   enterprise        → verified-package ∪ {vendor_tool_evidence,
//                                            board_hardware_evidence,
//                                            release_support_legal_evidence}
//
// On any unmet row the command exits with code 2 and the
// `AF_TIER_REQUIREMENTS_UNMET` envelope (details: payload with
// `required_rows`, `missing[]`, `core`, `maturity_verdict`). Unknown
// tier name → `AF_TIER_UNKNOWN`.

use assert_cmd::Command;
use serde_json::Value;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn af() -> Command {
    Command::cargo_bin("af").expect("cargo bin `af`")
}

fn mod_add_dir() -> PathBuf {
    repo_root().join("examples").join("af-mod-add")
}

fn run_verify(tier: &str) -> (i32, Value) {
    let build_root = tempfile::TempDir::new().unwrap();
    let core = mod_add_dir();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "core",
            "verify",
            core.to_str().unwrap(),
            "--tier",
            tier,
        ])
        .output()
        .expect("execute");
    let exit = out.status.code().expect("exit code");
    let value: Value =
        serde_json::from_slice(&out.stdout).expect("verify must produce JSON on stdout");
    (exit, value)
}

#[test]
fn community_tier_runs_against_reference_fixture() {
    let (exit, value) = run_verify("community");
    // Exit may be 0 (passed) or 2 (unmet) depending on maturity state of
    // the reference fixture. We only assert the contract is consistent.
    if exit == 0 {
        assert_eq!(value["status"].as_str(), Some("passed"));
        let payload = &value["tier_verification"];
        assert_eq!(payload["tier"].as_str(), Some("community"));
        let required = payload["required_rows"]
            .as_array()
            .expect("required_rows array");
        assert!(
            !required.is_empty(),
            "community tier must list ≥1 required row"
        );
    } else {
        assert_eq!(exit, 2, "unmet tier must use exit code 2");
        assert_eq!(value["code"].as_str(), Some("AF_TIER_REQUIREMENTS_UNMET"));
        let details = &value["details"];
        let required = details["required_rows"]
            .as_array()
            .expect("required_rows in error details");
        assert!(!required.is_empty());
    }
}

#[test]
fn verified_package_tier_requires_more_rows_than_community() {
    let (_, community) = run_verify("community");
    let (_, verified) = run_verify("verified-package");

    let n = |v: &Value| -> usize {
        v.get("tier_verification")
            .or_else(|| v.get("details"))
            .and_then(|p| p["required_rows"].as_array())
            .map(|a| a.len())
            .unwrap_or(0)
    };
    assert!(
        n(&verified) > n(&community),
        "verified-package must have strictly more required rows than community"
    );
}

#[test]
fn enterprise_tier_requires_most_rows() {
    let (_, verified) = run_verify("verified-package");
    let (_, enterprise) = run_verify("enterprise");
    let n = |v: &Value| -> usize {
        v.get("tier_verification")
            .or_else(|| v.get("details"))
            .and_then(|p| p["required_rows"].as_array())
            .map(|a| a.len())
            .unwrap_or(0)
    };
    assert!(
        n(&enterprise) > n(&verified),
        "enterprise must require strictly more rows than verified-package"
    );
}

#[test]
fn unknown_tier_returns_af_tier_unknown_envelope() {
    let build_root = tempfile::TempDir::new().unwrap();
    let core = mod_add_dir();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "core",
            "verify",
            core.to_str().unwrap(),
            "--tier",
            "platinum-elite",
        ])
        .output()
        .expect("execute");
    assert!(!out.status.success());
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert_eq!(value["code"].as_str(), Some("AF_TIER_UNKNOWN"));
    assert_eq!(value["exit_code"].as_i64(), Some(2));
    let hint = value["hint"].as_str().expect("hint");
    assert!(hint.contains("community"));
    assert!(hint.contains("verified-package"));
    assert!(hint.contains("enterprise"));
}

#[test]
fn community_required_rows_subset_of_verified_subset_of_enterprise() {
    let collect = |tier: &str| -> std::collections::BTreeSet<String> {
        let (_, v) = run_verify(tier);
        v.get("tier_verification")
            .or_else(|| v.get("details"))
            .and_then(|p| p["required_rows"].as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    let community = collect("community");
    let verified = collect("verified-package");
    let enterprise = collect("enterprise");
    assert!(
        community.is_subset(&verified),
        "community rows must be ⊆ verified-package rows"
    );
    assert!(
        verified.is_subset(&enterprise),
        "verified-package rows must be ⊆ enterprise rows"
    );
}

#[test]
fn unmet_tier_envelope_lists_missing_rows() {
    let (exit, value) = run_verify("enterprise");
    if exit == 0 {
        // Reference fixture already qualifies — skip.
        return;
    }
    assert_eq!(exit, 2);
    assert_eq!(value["code"].as_str(), Some("AF_TIER_REQUIREMENTS_UNMET"));
    let missing = value["details"]["missing"]
        .as_array()
        .expect("missing array");
    assert!(
        !missing.is_empty(),
        "AF_TIER_REQUIREMENTS_UNMET must carry a non-empty missing[]"
    );
    for row in missing {
        assert!(
            row["area"].is_string(),
            "each missing row must name an area"
        );
    }
}
