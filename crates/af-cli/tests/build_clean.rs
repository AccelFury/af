// SPDX-License-Identifier: Apache-2.0
//
// CLI integration coverage for `af build`. The `clean` subcommand is
// already covered in `build_root_layout.rs` (confirmation, removal,
// idempotency); this file focuses on the build orchestrator.
//
// Build paths to test:
//   * Default backend (`litex`) on the reference fixture ⇒ status passed
//     with FuseSoC/LiteX skeleton artifact in `<build-root>/litex/`.
//   * Unknown backend ⇒ AF_BACKEND_* envelope (exit 2 or 4) with no
//     panic.
//   * Missing core dir ⇒ AF_MANIFEST_* envelope (exit 2).
//   * Build report file lands in `<build-root>/reports/build-report.*`.

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

#[test]
fn build_with_default_backend_emits_artifacts_under_build_root() {
    let build_root = tempfile::TempDir::new().unwrap();
    let core = mod_add_dir();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "build",
            core.to_str().unwrap(),
            "--board",
            "digilent_arty_a7",
        ])
        .output()
        .expect("execute");

    assert!(
        out.status.success(),
        "default `litex` build must succeed on reference fixture; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert_eq!(value["status"].as_str(), Some("passed"));
    assert_eq!(value["backend"].as_str(), Some("litex"));
    assert_eq!(value["board"].as_str(), Some("digilent_arty_a7"));

    let artifacts = value["artifacts"].as_array().expect("artifacts array");
    assert!(!artifacts.is_empty(), "build must emit ≥1 artifact");

    // Every artifact path must live under `<build-root>/`.
    let br = build_root.path().to_string_lossy().to_string();
    for a in artifacts {
        let s = a.as_str().unwrap();
        assert!(
            s.contains(&br),
            "artifact `{s}` must be under build-root `{br}`"
        );
    }

    // build-report.{json,md} land in <build-root>/reports/
    let reports = &value["reports"];
    assert!(reports.is_object(), "reports must be object");
    let report_json = reports["json"].as_str().expect("reports.json path");
    let resolved = std::path::Path::new(report_json);
    let candidate = if resolved.is_absolute() {
        resolved.to_path_buf()
    } else {
        build_root.path().parent().unwrap().join(resolved)
    };
    assert!(
        std::path::Path::new(report_json).exists() || candidate.exists(),
        "report JSON file must exist at {report_json}"
    );
}

#[test]
fn build_with_unknown_backend_emits_envelope_without_panic() {
    let build_root = tempfile::TempDir::new().unwrap();
    let core = mod_add_dir();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "build",
            core.to_str().unwrap(),
            "--board",
            "digilent_arty_a7",
            "--backend",
            "totally-fake-backend",
        ])
        .output()
        .expect("execute");

    assert!(!out.status.success(), "unknown backend must fail");
    let exit = out.status.code().expect("exit code");
    // Documented per docs/cli-reference.md: 2 validation, 3 RTL/backend,
    // 4 backend unavailable, 9 build.
    assert!(
        matches!(exit, 2 | 3 | 4 | 9),
        "unknown backend exit code outside documented band: {exit}"
    );

    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON envelope");
    let code = value["code"].as_str().expect("code field");
    assert!(
        code.starts_with("AF_BACKEND_") || code.starts_with("AF_BUILD_"),
        "expected AF_BACKEND_* or AF_BUILD_* envelope, got {code}"
    );
    assert!(!value["message"].as_str().unwrap().is_empty());
    assert!(!value["hint"].as_str().unwrap().is_empty());
}

#[test]
fn build_with_missing_core_dir_emits_manifest_envelope() {
    let build_root = tempfile::TempDir::new().unwrap();
    let bogus = build_root.path().join("not-a-core-dir");
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "build",
            bogus.to_str().unwrap(),
            "--board",
            "digilent_arty_a7",
        ])
        .output()
        .expect("execute");

    assert!(!out.status.success(), "missing core dir must fail");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    let code = value["code"].as_str().expect("code");
    assert!(
        code.starts_with("AF_MANIFEST_") || code.starts_with("AF_CORE_"),
        "missing core dir expected AF_MANIFEST_* or AF_CORE_*, got {code}"
    );
    assert_eq!(value["exit_code"].as_i64(), Some(2));
}

#[test]
fn build_writes_report_under_build_root_reports_subdir() {
    let build_root = tempfile::TempDir::new().unwrap();
    let core = mod_add_dir();
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "build",
            core.to_str().unwrap(),
            "--board",
            "digilent_arty_a7",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success(), "build must succeed");

    let reports_dir = build_root.path().join("reports");
    assert!(
        reports_dir.is_dir(),
        "build must create <build-root>/reports/"
    );
    let json = reports_dir.join("build-report.json");
    let md = reports_dir.join("build-report.md");
    assert!(json.is_file(), "build-report.json must exist");
    assert!(md.is_file(), "build-report.md must exist");

    let report: Value =
        serde_json::from_slice(&std::fs::read(&json).unwrap()).expect("report JSON parses");
    assert_eq!(report["schema_version"].as_str(), Some("0.1"));
    assert_eq!(report["kind"].as_str(), Some("accelfury.report"));
}

#[test]
fn build_is_deterministic_for_same_inputs() {
    let core = mod_add_dir();
    let run = |br: &std::path::Path| -> Value {
        let out = af()
            .args([
                "--json",
                "--build-root",
                br.to_str().unwrap(),
                "build",
                core.to_str().unwrap(),
                "--board",
                "digilent_arty_a7",
            ])
            .output()
            .expect("execute");
        assert!(out.status.success());
        serde_json::from_slice(&out.stdout).unwrap()
    };

    let a = tempfile::TempDir::new().unwrap();
    let b = tempfile::TempDir::new().unwrap();
    let r_a = run(a.path());
    let r_b = run(b.path());

    // After replacing build-root paths with a marker, payloads must
    // match.
    fn redact(v: &Value, marker: &str) -> Value {
        match v {
            Value::String(s) if s.contains(marker) => Value::String("<build-root>".into()),
            Value::String(s) => Value::String(s.clone()),
            Value::Array(arr) => Value::Array(arr.iter().map(|x| redact(x, marker)).collect()),
            Value::Object(map) => {
                let mut out = serde_json::Map::new();
                for (k, val) in map {
                    out.insert(k.clone(), redact(val, marker));
                }
                Value::Object(out)
            }
            other => other.clone(),
        }
    }

    let s_a = redact(&r_a, a.path().to_str().unwrap());
    let s_b = redact(&r_b, b.path().to_str().unwrap());
    assert_eq!(
        s_a, s_b,
        "build JSON output (excluding build-root path) must be deterministic"
    );
}
