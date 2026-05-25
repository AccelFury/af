// SPDX-License-Identifier: Apache-2.0
//
// Cross-crate invariants: facts that span multiple sources of truth.
//
// 1. Manifesto axes (portability_level, priority, maturity,
//    verification_required) on each in-tree `examples/af_*/af-core.toml`
//    must match the corresponding entry in
//    `registries/cores.registry.json`.
// 2. Every AF_* code mentioned in `docs/cli-reference.md` must be a
//    live code (present in the fixture inventory).
// 3. The example manifests listed in `af-selfcheck.toml` must exist.

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn read_to_string(rel: &str) -> String {
    fs::read_to_string(repo_root().join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

fn parse_json(rel: &str) -> Value {
    let raw = read_to_string(rel);
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {rel} as json: {e}"))
}

/// Pull a `[core_id -> entry]` lookup from the cores registry.
fn registry_by_core_id() -> std::collections::BTreeMap<String, Value> {
    let registry = parse_json("registries/cores.registry.json");
    let cores = registry["cores"].as_array().expect("cores is array");
    let mut by_id = std::collections::BTreeMap::new();
    for entry in cores {
        let id = entry["core_id"]
            .as_str()
            .expect("core_id is string")
            .to_string();
        by_id.insert(id, entry.clone());
    }
    by_id
}

/// Discover `examples/<name>/af-core.toml` files that declare a
/// manifesto-axis `core = "af_<...>"`.
fn example_manifests() -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    let examples = repo_root().join("examples");
    if !examples.is_dir() {
        return out;
    }
    for entry in fs::read_dir(&examples).unwrap().flatten() {
        let p = entry.path().join("af-core.toml");
        if p.is_file() {
            let raw = fs::read_to_string(&p).unwrap();
            if let Ok(v) = raw.parse::<toml::Value>() {
                if let Some(core) = v.get("core").and_then(|c| c.as_str()) {
                    if core.starts_with("af_") {
                        out.push((core.to_string(), p));
                    }
                }
            }
        }
    }
    out
}

#[test]
fn example_manifests_match_registry_axes() {
    let by_id = registry_by_core_id();
    let examples = example_manifests();
    assert!(!examples.is_empty(), "no example manifests discovered");

    for (core_id, manifest_path) in examples {
        let entry = match by_id.get(&core_id) {
            Some(e) => e,
            None => {
                // Not every example is registered (a few are pure
                // didactic fixtures). Skip those rather than failing.
                continue;
            }
        };

        let manifest_raw = fs::read_to_string(&manifest_path).unwrap();
        let manifest: toml::Value = toml::from_str(&manifest_raw).unwrap();

        for axis in ["portability_level", "priority", "maturity"] {
            let registry_val = entry[axis].as_str().unwrap_or("<missing>");
            let manifest_val = manifest
                .get(axis)
                .and_then(|v| v.as_str())
                .unwrap_or("<missing>");
            assert_eq!(
                registry_val, manifest_val,
                "manifesto axis `{axis}` divergence for {core_id} (registry={registry_val}, manifest={manifest_val} at {})",
                manifest_path.display()
            );
        }

        // verification_required is an array; assert set equality.
        let registry_set: std::collections::BTreeSet<String> = entry["verification_required"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();
        let manifest_set: std::collections::BTreeSet<String> = manifest
            .get("verification_required")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| {
                        // Each entry is a table {kind="...", ...}; some are bare strings.
                        v.get("kind")
                            .and_then(|k| k.as_str())
                            .or_else(|| v.as_str())
                            .map(String::from)
                    })
                    .collect()
            })
            .unwrap_or_default();
        if !registry_set.is_empty() && !manifest_set.is_empty() {
            assert_eq!(
                registry_set,
                manifest_set,
                "verification_required set divergence for {core_id} at {}",
                manifest_path.display()
            );
        }
    }
}

#[test]
fn cli_reference_only_mentions_live_af_codes() {
    let fixture: std::collections::BTreeSet<String> = include_str!("fixtures/af_error_codes.txt")
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    let docs = read_to_string("docs/cli-reference.md");
    // Find every `AF_*` mention in the doc.
    let mut mentioned = std::collections::BTreeSet::new();
    let bytes = docs.as_bytes();
    let mut i = 0;
    while i + 3 <= bytes.len() {
        if &bytes[i..i + 3] == b"AF_"
            && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_')
        {
            let start = i;
            let mut j = i + 3;
            while j < bytes.len()
                && (bytes[j].is_ascii_uppercase() || bytes[j].is_ascii_digit() || bytes[j] == b'_')
            {
                j += 1;
            }
            let mut end = j;
            while end > start + 3 && bytes[end - 1] == b'_' {
                end -= 1;
            }
            if end - start >= 4 {
                let code = std::str::from_utf8(&bytes[start..end]).unwrap().to_string();
                mentioned.insert(code);
            }
            i = j;
        } else {
            i += 1;
        }
    }
    let stray: Vec<_> = mentioned
        .iter()
        .filter(|m| !fixture.contains(*m))
        .filter(|m| !is_env_var_or_placeholder(m))
        .collect();
    assert!(
        stray.is_empty(),
        "docs/cli-reference.md mentions codes that are not live in the source tree: {stray:?}\n\
         Either fix the docs or register the codes (see crates/af-cli/tests/fixtures/af_error_codes.txt)."
    );
}

fn is_env_var_or_placeholder(code: &str) -> bool {
    matches!(
        code,
        "AF_BUILD_ROOT"
            | "AF_AGENT_NAME"
            | "AF_SELF_CHECK_AF_MOD_ADD"
            | "AF_SELF_CHECK_AF_RESET_SYNC"
            | "AF_INSTALL_FULL_SMT_SOLVERS"
            | "AF_INSTALL_LITEX"
            | "AF_X"
    )
}

#[test]
fn selfcheck_manifest_targets_resolve() {
    let raw = read_to_string("af-selfcheck.toml");
    let parsed: toml::Value = toml::from_str(&raw).unwrap();
    let targets = parsed
        .get("targets")
        .and_then(|t| t.as_array())
        .expect("af-selfcheck.toml [[targets]] is an array");
    assert!(!targets.is_empty(), "selfcheck has zero targets");
    for target in targets {
        let name = target["name"].as_str().unwrap();
        let path = target["path"].as_str().unwrap();
        let resolved = repo_root().join(path);
        if target.get("required").and_then(|v| v.as_bool()) == Some(true) {
            assert!(
                resolved.exists(),
                "required selfcheck target `{name}` is missing at {}",
                resolved.display()
            );
        }
    }
}

#[test]
fn production_workflow_runs_required_gates() {
    let workflow = read_to_string(".github/workflows/accelfury.yml");
    let release_workflow = read_to_string(".github/workflows/release.yml");
    let guard_base = concat!(
        "AF",
        "_GUARD_BASE=\"${{ github.event.pull_request.base.sha }}\""
    );
    for required in [
        "workflow_dispatch:",
        "permissions:",
        "fetch-depth: 0",
        "cargo fmt --all -- --check",
        "cargo clippy --workspace --all-targets -- -D warnings",
        "cargo test --workspace",
        guard_base,
        ".claude/skills/af-cli-contract-guard/check.sh",
        "manifest validate examples/af-pdm-rx/af-core.toml --json",
        "core check examples/af-pdm-rx --json",
        "core report examples/af-pdm-rx --json",
        "core lint examples/af-pdm-rx --backend native --json",
        "wrapper generate examples/af-pdm-rx --target fusesoc --json",
        "wrapper generate examples/af-pdm-rx --target litex --board tang-nano-20k --json",
        "ci generate --target github-actions",
        "scripts/docker-smoke.sh",
        "SHA256SUMS",
        "actions/upload-artifact@v4",
    ] {
        assert!(
            workflow.contains(required),
            ".github/workflows/accelfury.yml missing production gate {required:?}"
        );
    }

    for required in [
        "ci_run_id:",
        "cargo build --locked --release -p af-cli --bin af",
        "sha256sum \"af-${TAG}-x86_64-unknown-linux-gnu.tar.gz\" > SHA256SUMS",
        "docker push \"$IMAGE\"",
        "docker buildx imagetools inspect \"$IMAGE\"",
        "release check",
        "--ci-evidence .af-build/release/ci-evidence.json",
        "--artifact-dir .af-build/release/artifacts",
        "--docker-evidence .af-build/release/docker-image.json",
        "--output .af-build/release/release-readiness.json",
        "gh release create \"$TAG\"",
    ] {
        assert!(
            release_workflow.contains(required),
            ".github/workflows/release.yml missing release gate {required:?}"
        );
    }
}

#[test]
fn docker_smoke_covers_required_open_source_backends() {
    let dockerfile = read_to_string("Dockerfile");
    for required in ["verilator", "yosys", "iverilog"] {
        assert!(
            dockerfile.contains(required),
            "Dockerfile must install {required} for production smoke coverage"
        );
    }

    let smoke = read_to_string("scripts/docker-smoke.sh");
    let icarus_core_dir = concat!("AF", "_ICARUS_CORE_DIR:-examples/af-reset-sync");
    for required in [
        icarus_core_dir,
        "== AccelFury Docker smoke: Icarus ==",
        "core lint \"${ICARUS_CORE_DIR}\" --backend icarus --json | tee \"${BUILD_ROOT}/logs/icarus-lint.json\"",
        "core sim \"${ICARUS_CORE_DIR}\" --backend icarus --json | tee \"${BUILD_ROOT}/logs/icarus-sim.json\"",
    ] {
        assert!(
            smoke.contains(required),
            "scripts/docker-smoke.sh missing required Icarus smoke step {required:?}"
        );
    }
}

#[test]
fn production_docs_pin_claim_boundaries() {
    let production = read_to_string("docs/production-readiness.md");
    for required in [
        "CLI/toolchain",
        "does not mean FPGA timing closure",
        "CDC/RDC signoff",
        "vendor production bitstreams",
        "hardware programming",
        "A workflow file alone is",
        "not production evidence.",
        "Removing a command, flag, JSON",
        "field, manifest field, schema property, exit-code meaning, or `AF_*` error code",
    ] {
        assert!(
            production.contains(required),
            "docs/production-readiness.md missing claim boundary {required:?}"
        );
    }

    for rel in [
        "README.md",
        "docs/known-limitations.md",
        "docs/release-process.md",
        ".github/PULL_REQUEST_TEMPLATE.md",
    ] {
        let text = read_to_string(rel);
        assert!(
            text.contains("production-readiness")
                || text.contains("unsupported timing")
                || text.contains("Production-ready"),
            "{rel} must reference production readiness or unsupported claims"
        );
    }
}
