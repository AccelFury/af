// SPDX-License-Identifier: Apache-2.0

use assert_cmd::Command;
use predicates::str::contains;
use serde_json::Value;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn write_text(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

#[cfg(unix)]
fn write_executable(path: &Path, content: &str) {
    use std::os::unix::fs::PermissionsExt;

    write_text(path, content);
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

#[test]
fn ci_init_generates_workflow_and_docs() {
    let dir = tempdir().unwrap();
    write_text(
        &dir.path().join("rtl").join("core_only.v"),
        "module af_ci_cli_test();\nendmodule\n",
    );

    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.args(["ci", "init"])
        .args(["--repo", dir.path().to_str().unwrap()])
        .args(["--project", "af_ci_cli_test"])
        .args(["--hdl", "verilog-2001"])
        .args(["--rtl", "rtl"])
        .args(["--top", "af_ci_cli_test"]);
    cmd.assert().success();

    assert!(dir.path().join(".github/workflows/hdl-ci.yml").is_file());
    assert!(dir
        .path()
        .join(".github/PULL_REQUEST_TEMPLATE.md")
        .is_file());
    assert!(dir.path().join("docs/ci.md").is_file());
    assert!(dir.path().join("af-ci.toml").is_file());
    assert!(dir
        .path()
        .join("artifacts/openfpga-ci/reports/af-ci-init-report.json")
        .is_file());

    let docs = fs::read_to_string(dir.path().join("docs/ci.md")).unwrap();
    assert!(docs.contains("CI не доказывает") || docs.contains("What CI does not prove"));

    let pr = fs::read_to_string(dir.path().join(".github/PULL_REQUEST_TEMPLATE.md")).unwrap();
    assert!(pr.contains("Simulation job checks regressions"));
    assert!(pr.contains("Yosys hierarchy/check and JSON synthesis succeeds"));
    assert!(pr.contains("Artifact bundle is produced and attached"));
    assert!(pr.contains("`docs/ci.md` contains what CI can and cannot prove"));
}

#[test]
fn ci_init_dry_run_does_not_modify_filesystem() {
    let dir = tempdir().unwrap();
    write_text(
        &dir.path().join("rtl").join("core_only.v"),
        "module af_ci_cli_test();\nendmodule\n",
    );

    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.args(["ci", "init", "--repo", dir.path().to_str().unwrap()])
        .args(["--project", "af_ci_cli_test"])
        .args(["--hdl", "verilog-2001"])
        .args(["--rtl", "rtl"])
        .args(["--top", "af_ci_cli_test"])
        .arg("--dry-run")
        .assert()
        .success();

    assert!(!dir.path().join(".github/workflows/hdl-ci.yml").exists());
    assert!(!dir.path().join("af-ci.toml").exists());
    assert!(!dir.path().join("docs/ci.md").exists());
}

#[test]
fn ci_init_accepts_explicit_sim_command() {
    let dir = tempdir().unwrap();
    write_text(
        &dir.path().join("rtl").join("core_only.v"),
        "module af_ci_cli_test();\nendmodule\n",
    );

    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.args(["ci", "init", "--repo", dir.path().to_str().unwrap()])
        .args(["--project", "af_ci_cli_test"])
        .args(["--hdl", "verilog-2001"])
        .args(["--rtl", "rtl"])
        .args(["--top", "af_ci_cli_test"])
        .args(["--sim", "cd sim && make test"])
        .assert()
        .success();

    let workflow = fs::read_to_string(dir.path().join(".github/workflows/hdl-ci.yml")).unwrap();
    assert!(workflow.contains("cd sim && make test"));
}

#[test]
fn ci_render_is_idempotent() {
    let dir = tempdir().unwrap();
    write_text(
        &dir.path().join("rtl").join("core_only.v"),
        "module af_ci_cli_test();\nendmodule\n",
    );

    let mut init = Command::cargo_bin("af").unwrap();
    init.args(["ci", "init", "--repo", dir.path().to_str().unwrap()])
        .args(["--project", "af_ci_cli_test"])
        .args(["--hdl", "verilog-2001"])
        .args(["--rtl", "rtl"])
        .args(["--top", "af_ci_cli_test"])
        .assert()
        .success();

    let mut render = Command::cargo_bin("af").unwrap();
    render
        .current_dir(dir.path())
        .args(["ci", "render"])
        .args(["--config", "af-ci.toml"])
        .args(["--output", ".github/workflows/hdl-ci.yml"])
        .assert()
        .success();

    let first = fs::read_to_string(dir.path().join(".github/workflows/hdl-ci.yml")).unwrap();

    let mut render2 = Command::cargo_bin("af").unwrap();
    render2
        .current_dir(dir.path())
        .args(["ci", "render"])
        .args(["--config", "af-ci.toml"])
        .args(["--output", ".github/workflows/hdl-ci.yml"])
        .assert()
        .success();

    let second = fs::read_to_string(dir.path().join(".github/workflows/hdl-ci.yml")).unwrap();
    assert_eq!(first, second);
}

#[test]
fn ci_add_board_requires_package_for_ice40() {
    let dir = tempdir().unwrap();
    write_text(
        &dir.path().join("rtl").join("core_only.v"),
        "module af_ci_cli_test();\nendmodule\n",
    );
    write_text(
        &dir.path().join("boards").join("icebreaker.pcf"),
        "# constraint\n",
    );

    let mut init = Command::cargo_bin("af").unwrap();
    init.args(["ci", "init"])
        .args(["--repo", dir.path().to_str().unwrap()])
        .args(["--project", "af_ci_cli_test"])
        .args(["--hdl", "verilog-2001"])
        .args(["--rtl", "rtl"])
        .args(["--top", "af_ci_cli_test"])
        .assert()
        .success();

    let mut add = Command::cargo_bin("af").unwrap();
    add.args(["ci", "add-board"])
        .args(["--repo", dir.path().to_str().unwrap()])
        .args(["--name", "ice40"])
        .args(["--family", "ice40"])
        .args(["--top", "top_af"])
        .args(["--device", "up5k"])
        .args(["--constraints", "boards/icebreaker.pcf"])
        .assert()
        .failure()
        .stderr(contains("AF_CI_ADD_BOARD_PACKAGE_REQUIRED"));
}

#[test]
fn ci_add_board_requires_nextpnr_family_for_gowin() {
    let dir = tempdir().unwrap();
    write_text(
        &dir.path().join("rtl").join("core_only.v"),
        "module af_ci_cli_test();\nendmodule\n",
    );
    write_text(
        &dir.path().join("boards").join("tang.cst"),
        "# constraint\n",
    );

    let mut init = Command::cargo_bin("af").unwrap();
    init.args(["ci", "init"])
        .args(["--repo", dir.path().to_str().unwrap()])
        .args(["--project", "af_ci_cli_test"])
        .args(["--hdl", "verilog-2001"])
        .args(["--rtl", "rtl"])
        .args(["--top", "af_ci_cli_test"])
        .assert()
        .success();

    let mut add = Command::cargo_bin("af").unwrap();
    add.args(["ci", "add-board"])
        .args(["--repo", dir.path().to_str().unwrap()])
        .args(["--name", "gowin"])
        .args(["--family", "gowin"])
        .args(["--top", "top_af"])
        .args(["--device", "GW2A-LV18PG256C8/I7"])
        .args(["--pack-device", "GW2A-18C"])
        .args(["--constraints", "boards/tang.cst"])
        .assert()
        .failure()
        .stderr(contains("AF_CI_ADD_BOARD_GOWIN_REQUIREMENTS"));
}

#[test]
fn ci_render_skips_pnr_without_existing_constraint_file() {
    let dir = tempdir().unwrap();
    let mut init = Command::cargo_bin("af").unwrap();
    init.args(["ci", "init"])
        .args(["--repo", dir.path().to_str().unwrap()])
        .args(["--project", "af_ci_cli_test"])
        .args(["--hdl", "verilog-2001"])
        .args(["--rtl", "rtl"])
        .args(["--top", "af_ci_cli_test"])
        .assert()
        .success();

    write_text(
        &dir.path().join("boards").join("sample_top.v"),
        "module top_af();\nendmodule\n",
    );
    write_text(
        &dir.path().join("boards").join("missing.cst"),
        "# constraint\n",
    );

    let mut add = Command::cargo_bin("af").unwrap();
    add.args(["ci", "add-board"])
        .args(["--repo", dir.path().to_str().unwrap()])
        .args(["--name", "gowin_missing"])
        .args(["--family", "gowin"])
        .args(["--top", "top_af"])
        .args(["--device", "GW2A-LV18PG256C8/I7"])
        .args(["--nextpnr-family", "GW2A-18C"])
        .args(["--pack-device", "GW2A-18C"])
        .args(["--constraints", "boards/missing.cst"])
        .args(["--source-globs", "boards/**/*.v"])
        .assert()
        .success();
    fs::remove_file(dir.path().join("boards").join("missing.cst")).unwrap();

    let mut render = Command::cargo_bin("af").unwrap();
    render
        .current_dir(dir.path())
        .args(["ci", "render"])
        .args(["--config", "af-ci.toml"])
        .args(["--output", ".github/workflows/hdl-ci-gowin.yml"])
        .assert()
        .success();

    let workflow =
        fs::read_to_string(dir.path().join(".github/workflows/hdl-ci-gowin.yml")).unwrap();
    assert!(!workflow.contains("pnr_0:"));
}

#[test]
fn ci_init_fails_when_top_is_ambiguous() {
    let dir = tempdir().unwrap();
    write_text(
        &dir.path().join("rtl").join("first.v"),
        "module one();\nendmodule\n",
    );
    write_text(
        &dir.path().join("rtl").join("second.v"),
        "module two();\nendmodule\n",
    );

    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.args(["ci", "init", "--repo", dir.path().to_str().unwrap()])
        .args(["--project", "af_ci_cli_ambiguous"])
        .args(["--hdl", "verilog-2001"])
        .args(["--rtl", "rtl"])
        .assert()
        .failure()
        .stderr(contains("AF_CI_INIT_TOP_"));
}

#[test]
fn ci_validate_fails_on_disallowed_upload_path() {
    let dir = tempdir().unwrap();
    write_text(
        &dir.path().join("rtl").join("core_only.v"),
        "module af_ci_cli_test();\nendmodule\n",
    );

    let mut init = Command::cargo_bin("af").unwrap();
    init.args(["ci", "init"])
        .args(["--repo", dir.path().to_str().unwrap()])
        .args(["--project", "af_ci_cli_test"])
        .args(["--hdl", "verilog-2001"])
        .args(["--rtl", "rtl"])
        .args(["--top", "af_ci_cli_test"])
        .assert()
        .success();

    let workflow = dir.path().join(".github/workflows/hdl-ci.yml");
    let rendered = fs::read_to_string(&workflow).unwrap().replace(
        "            artifacts/openfpga-ci/reports/*.json",
        "            ../artifacts/openfpga-ci/reports/*.json",
    );
    fs::write(&workflow, rendered).unwrap();

    let mut validate = Command::cargo_bin("af").unwrap();
    validate
        .args(["ci", "validate"])
        .args(["--repo", dir.path().to_str().unwrap()])
        .arg("--json")
        .assert()
        .failure()
        .stdout(contains("\"exit_code\": 1"))
        .stdout(contains("\"problem_classes\""))
        .stdout(contains("\"detected\""))
        .stdout(contains("\"artifact_contract\""))
        .stdout(contains("artifact_upload_unsafe"))
        .stdout(contains(
            "artifact upload path '../artifacts/openfpga-ci/reports/*.json' is not allowed",
        ));
}

#[test]
fn ci_doctor_reports_missing_synth_core_job() {
    let dir = tempdir().unwrap();
    write_text(
        &dir.path().join("rtl").join("core_only.v"),
        "module af_ci_cli_test();\nendmodule\n",
    );

    let mut init = Command::cargo_bin("af").unwrap();
    init.args(["ci", "init"])
        .args(["--repo", dir.path().to_str().unwrap()])
        .args(["--project", "af_ci_cli_test"])
        .args(["--hdl", "verilog-2001"])
        .args(["--rtl", "rtl"])
        .args(["--top", "af_ci_cli_test"])
        .assert()
        .success();

    let workflow = dir.path().join(".github/workflows/hdl-ci.yml");
    let rendered = fs::read_to_string(&workflow).unwrap();
    let start = rendered.find("  synth_core:\n").unwrap_or_default();
    let end = rendered
        .find("  package_artifacts:\n")
        .unwrap_or(rendered.len());
    let rendered = if start < end {
        format!("{}{}", &rendered[..start], &rendered[end..])
    } else {
        rendered
    };
    fs::write(&workflow, rendered).unwrap();

    let mut doctor = Command::cargo_bin("af").unwrap();
    doctor
        .args(["ci", "doctor"])
        .args(["--repo", dir.path().to_str().unwrap()])
        .arg("--json")
        .assert()
        .failure()
        .stdout(contains("\"exit_code\": 1"))
        .stdout(contains("required workflow job 'synth_core' is missing"));
}

#[test]
fn ci_doctor_reports_machine_readable_problem_classes() {
    let dir = tempdir().unwrap();
    write_text(
        &dir.path().join("rtl").join("core_only.v"),
        "module af_ci_cli_test();\nendmodule\n",
    );

    let mut init = Command::cargo_bin("af").unwrap();
    init.args(["ci", "init"])
        .args(["--repo", dir.path().to_str().unwrap()])
        .args(["--project", "af_ci_cli_test"])
        .args(["--hdl", "verilog-2001"])
        .args(["--rtl", "rtl"])
        .args(["--top", "af_ci_cli_test"])
        .assert()
        .success();

    let config_path = dir.path().join("af-ci.toml");
    let config = fs::read_to_string(&config_path)
        .unwrap()
        .replace("top = \"af_ci_cli_test\"", "top = \"missing_top\"");
    fs::write(&config_path, config).unwrap();
    fs::remove_file(dir.path().join("docs/ci.md")).unwrap();
    fs::remove_file(dir.path().join(".github/PULL_REQUEST_TEMPLATE.md")).unwrap();
    write_text(
        &dir.path().join(".github/workflows/hdl-ci.yml"),
        r#"name: Bad HDL CI
on:
  push:
    branches: ["main"]
jobs:
  sim:
    runs-on: ubuntu-latest
    steps:
      - run: |
          set -euo pipefail
          echo safe
      - run: curl https://example.invalid/install.sh | sh
      - run: vivado -mode batch -source run.tcl
  package_artifacts:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/upload-artifact@v4
        with:
          name: unsafe
          path: |
            .
            .env
"#,
    );

    let output = Command::cargo_bin("af")
        .unwrap()
        .args(["ci", "doctor"])
        .args(["--repo", dir.path().to_str().unwrap()])
        .arg("--json")
        .output()
        .unwrap();
    assert!(!output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let payload: Value = serde_json::from_str(&stdout).unwrap();
    let classes = payload
        .get("details")
        .and_then(|details| details.get("problem_classes"))
        .and_then(Value::as_array)
        .expect("doctor error details must include problem_classes");
    let classes = classes
        .iter()
        .map(|class| class.as_str().unwrap())
        .collect::<Vec<_>>();

    assert!(
        classes.len() >= 10,
        "expected at least 10 CI problem classes, got {classes:?}"
    );
    for expected in [
        "artifact_upload_unsafe",
        "config_top_not_detected",
        "docs_ci_missing",
        "pr_template_missing",
        "secret_artifact_policy_violation",
        "sha256_missing",
        "synth_json_missing",
        "tool_versions_missing",
        "unsafe_shell_pipe",
        "vendor_tool_policy_violation",
        "workflow_job_missing",
        "workflow_permissions_missing",
        "workflow_shell_safety_missing",
        "workflow_trigger_missing",
        "yosys_hierarchy_check_missing",
    ] {
        assert!(
            classes.contains(&expected),
            "expected CI problem class `{expected}` in {classes:?}"
        );
    }
}

#[test]
fn ci_doctor_passes_on_clean_project() {
    let dir = tempdir().unwrap();
    write_text(
        &dir.path().join("rtl").join("core_only.v"),
        "module af_ci_cli_test();\nendmodule\n",
    );

    let mut init = Command::cargo_bin("af").unwrap();
    init.args(["ci", "init"])
        .args(["--repo", dir.path().to_str().unwrap()])
        .args(["--project", "af_ci_cli_test"])
        .args(["--hdl", "verilog-2001"])
        .args(["--rtl", "rtl"])
        .args(["--top", "af_ci_cli_test"])
        .assert()
        .success();

    let mut doctor = Command::cargo_bin("af").unwrap();
    doctor
        .args(["ci", "doctor"])
        .args(["--repo", dir.path().to_str().unwrap()])
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("\"status\": \"pass\""));
}

#[cfg(unix)]
#[test]
fn ci_run_local_sim_checks_icarus_compile_and_runtime_tools() {
    let dir = tempdir().unwrap();
    write_text(
        &dir.path().join("rtl").join("core_only.v"),
        "module af_ci_cli_test();\nendmodule\n",
    );
    write_text(
        &dir.path().join("sim").join("Makefile"),
        "test:\n\t@echo PASS\n",
    );

    let mut init = Command::cargo_bin("af").unwrap();
    init.args(["ci", "init"])
        .args(["--repo", dir.path().to_str().unwrap()])
        .args(["--project", "af_ci_cli_test"])
        .args(["--hdl", "verilog-2001"])
        .args(["--rtl", "rtl"])
        .args(["--top", "af_ci_cli_test"])
        .args(["--sim", "cd sim && make test"])
        .assert()
        .success();

    let tools = tempdir().unwrap();
    write_executable(&tools.path().join("make"), "#!/bin/sh\nexit 0\n");

    let mut missing_icarus = Command::cargo_bin("af").unwrap();
    missing_icarus
        .env("PATH", tools.path())
        .arg("--json")
        .args(["ci", "run-local"])
        .args(["--repo", dir.path().to_str().unwrap()])
        .args(["--profile", "sim"])
        .assert()
        .failure()
        .stdout(contains("\"code\": \"AF_CI_RUN_LOCAL_FAIL\""));

    write_executable(&tools.path().join("iverilog"), "#!/bin/sh\nexit 0\n");
    write_executable(&tools.path().join("vvp"), "#!/bin/sh\nexit 0\n");

    let mut available_icarus = Command::cargo_bin("af").unwrap();
    available_icarus
        .env("PATH", tools.path())
        .arg("--json")
        .args(["ci", "run-local"])
        .args(["--repo", dir.path().to_str().unwrap()])
        .args(["--profile", "sim"])
        .assert()
        .success()
        .stdout(contains("\"status\": \"passed\""));
}
