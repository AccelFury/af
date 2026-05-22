// SPDX-License-Identifier: Apache-2.0
use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str::contains;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

#[test]
fn doctor_json_works_without_optional_tools() {
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.args(["doctor", "--json"])
        .assert()
        .success()
        .stdout(contains("\"generated_by\""))
        .stdout(contains("\"tool_versions\""))
        .stdout(contains("\"tool\": \"iverilog\""))
        .stdout(contains("\"tool\": \"vvp\""))
        .stdout(contains("\"tool\": \"verilator\""))
        .stdout(contains("\"tool\": \"yosys\""))
        .stdout(contains("\"tool\": \"sby\""))
        .stdout(contains("\"tool\": \"fusesoc\""))
        .stdout(contains("\"tool\": \"xmllint\""))
        .stdout(contains("\"tool\": \"boolector\""))
        .stdout(contains("\"tool\": \"z3\""))
        .stdout(contains("\"tool\": \"yices-smt2\""))
        .stdout(contains("\"tool\": \"bitwuzla\""))
        .stdout(contains("\"tool\": \"cvc5\""))
        .stdout(contains("\"tool\": \"deno\""))
        .stdout(contains("\"tool\": \"deno-audit-repo\""))
        .stdout(contains("audit:repo"));
}

#[test]
fn command_lifecycle_logging_is_colored_and_enabled_by_default() {
    let mut colored = Command::cargo_bin("af").unwrap();
    colored
        .args(["doctor"])
        .assert()
        .success()
        .stderr(contains("\u{1b}["))
        .stderr(contains("af command started"))
        .stderr(contains("af command completed"))
        .stderr(contains("command").and(contains("doctor")));

    let mut plain = Command::cargo_bin("af").unwrap();
    plain
        .args(["--color", "never", "doctor"])
        .assert()
        .success()
        .stderr(contains("af command started"))
        .stderr(contains("\u{1b}[").not());
}

#[test]
fn human_errors_are_colored_by_default_and_json_stays_plain() {
    let mut colored = Command::cargo_bin("af").unwrap();
    colored
        .args([
            "manifest",
            "migrate",
            "missing.toml",
            "--from",
            "bad",
            "--to",
            "0.2",
        ])
        .assert()
        .failure()
        .stderr(contains("\u{1b}["))
        .stderr(contains("AF_MANIFEST_MIGRATION_UNSUPPORTED"));

    let mut json = Command::cargo_bin("af").unwrap();
    json.args([
        "--json",
        "manifest",
        "migrate",
        "missing.toml",
        "--from",
        "bad",
        "--to",
        "0.2",
    ])
    .assert()
    .failure()
    .stdout(contains("\u{1b}[").not())
    .stdout(contains("\"code\": \"AF_MANIFEST_MIGRATION_UNSUPPORTED\""));
}

#[test]
fn json_stdout_stays_plain_when_default_logging_is_enabled() {
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.args(["--json", "--color", "always", "doctor"])
        .assert()
        .success()
        .stdout(contains("\"generated_by\""))
        .stdout(contains("af command started").not())
        .stdout(contains("\u{1b}[").not())
        .stderr(contains("af command started"))
        .stderr(contains("af command completed"))
        .stderr(contains("\u{1b}["));
}

#[test]
fn quiet_suppresses_default_lifecycle_logging() {
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.args(["--quiet", "doctor"])
        .assert()
        .success()
        .stdout(contains("doctor").not())
        .stderr(contains("af command started").not());
}

#[test]
fn tooling_check_reports_missing_tools_without_host_path() {
    let root = repo_root();
    let empty_path = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(root)
        .env("PATH", empty_path.path())
        .args(["tooling", "check", "--json"])
        .assert()
        .success()
        .stdout(contains("\"kind\": \"accelfury.tooling_report\""))
        .stdout(contains("\"tool\": \"iverilog\""))
        .stdout(contains("\"tool\": \"vvp\""))
        .stdout(contains("\"tool\": \"verilator\""))
        .stdout(contains("\"tool\": \"fusesoc\""))
        .stdout(contains("\"tool\": \"edalize\""))
        .stdout(contains("\"tool\": \"xmllint\""))
        .stdout(contains("\"status\": \"warning\""))
        .stdout(contains("docker-runtime"));
}

#[test]
fn tooling_plan_groups_icarus_compile_and_runtime_system_package() {
    let root = repo_root();
    let empty_path = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(root)
        .env("PATH", empty_path.path())
        .args([
            "tooling",
            "plan",
            "--install-mode",
            "system",
            "--tools",
            "iverilog,vvp",
            "--allow-network",
            "--allow-system",
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"provider\": \"apt\""))
        .stdout(contains("\"tool\": \"iverilog\""))
        .stdout(contains("\"tool\": \"vvp\""))
        .stdout(contains("\"requires_sudo\": true"))
        .stdout(contains("\"apt-get\""))
        .stdout(contains("\"iverilog\""));
}

#[test]
fn tooling_plan_does_not_promise_portable_system_sby_package() {
    let root = repo_root();
    let empty_path = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(root)
        .env("PATH", empty_path.path())
        .args([
            "tooling",
            "plan",
            "--install-mode",
            "system",
            "--tools",
            "sby",
            "--allow-network",
            "--allow-system",
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains(
            "\"code\": \"AF_TOOLING_SYSTEM_INSTALL_UNSUPPORTED\"",
        ))
        .stdout(contains("has no portable system package action"));
}

#[test]
fn tooling_plan_defaults_to_docker_runtime_actions() {
    let root = repo_root();
    let empty_path = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(root)
        .env("PATH", empty_path.path())
        .args(["tooling", "plan", "--tools", "verilator,yosys", "--json"])
        .assert()
        .success()
        .stdout(contains("\"id\": \"docker-runtime\""))
        .stdout(contains("\"requires_network\": true"))
        .stdout(contains("af-toolchain policy is offline/no-network"));
}

#[test]
fn tooling_plan_includes_smt_solvers_in_docker_runtime() {
    let root = repo_root();
    let empty_path = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(root)
        .env("PATH", empty_path.path())
        .args([
            "tooling",
            "plan",
            "--tools",
            "boolector,z3,yices,bitwuzla,cvc5",
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"id\": \"docker-runtime\""))
        .stdout(contains("\"tool\": \"boolector\""))
        .stdout(contains("\"tool\": \"z3\""))
        .stdout(contains("\"tool\": \"yices-smt2\""))
        .stdout(contains("\"tool\": \"bitwuzla\""))
        .stdout(contains("\"tool\": \"cvc5\""));
}

#[test]
fn tooling_plan_includes_core_integration_tools_in_docker_runtime() {
    let root = repo_root();
    let empty_path = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(root)
        .env("PATH", empty_path.path())
        .args([
            "tooling",
            "plan",
            "--tools",
            "xmllint,fusesoc,edalize",
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"id\": \"docker-runtime\""))
        .stdout(contains("\"tool\": \"xmllint\""))
        .stdout(contains("\"tool\": \"fusesoc\""))
        .stdout(contains("\"tool\": \"edalize\""));
}

#[test]
fn tooling_ensure_requires_confirmation_before_install() {
    let root = repo_root();
    let empty_path = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(root)
        .env("PATH", empty_path.path())
        .args([
            "tooling",
            "ensure",
            "--tools",
            "verilator",
            "--allow-network",
            "--json",
        ])
        .assert()
        .failure()
        .stdout(contains("\"code\": \"AF_TOOLING_CONFIRMATION_REQUIRED\""));
}

#[test]
fn core_check_af_pdm_rx_passes() {
    let root = repo_root();
    let build = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.arg("--build-root")
        .arg(build.path())
        .args(["core", "check"])
        .arg(root.join("examples/af-pdm-rx"))
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("\"status\": \"passed\""));

    assert!(build.path().join("reports/core-check.json").is_file());
    assert!(build.path().join("reports/core-check.md").is_file());
    assert!(build
        .path()
        .join("reports/core_check_report.json")
        .is_file());
    assert!(build.path().join("reports/core_check_report.md").is_file());
}

#[test]
fn self_check_runs_in_tree_examples_and_skips_missing_optional_targets() {
    let root = repo_root();
    let build = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(&root)
        .env_remove("AF_SELF_CHECK_AF_MOD_ADD")
        .env_remove("AF_SELF_CHECK_AF_RESET_SYNC")
        .arg("--build-root")
        .arg(build.path())
        .args(["self", "check", "--json"])
        .assert()
        .success()
        .stdout(contains("\"kind\": \"accelfury.self_check\""))
        .stdout(contains("\"name\": \"example-af-pdm-rx\""))
        .stdout(contains(
            "\"source\": \"https://github.com/AccelFury/af-pdm-rx\"",
        ))
        .stdout(contains("\"name\": \"example-af-mod-add\""))
        .stdout(contains("\"status\": \"passed\""));

    let report = build.path().join("reports/self-check.json");
    assert!(report.is_file());
    let text = std::fs::read_to_string(report).unwrap();
    assert!(text.contains("\"path_env\": \"AF_SELF_CHECK_AF_MOD_ADD\""));
    assert!(text.contains("\"path_env\": \"AF_SELF_CHECK_AF_RESET_SYNC\""));
}

#[test]
fn core_report_includes_reusable_maturity_verdict() {
    let root = repo_root();
    let build = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.arg("--build-root")
        .arg(build.path())
        .args(["core", "report"])
        .arg(root.join("examples/af-mod-add"))
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("\"maturity\""))
        .stdout(contains("\"verdict\": \"blocked\""))
        .stdout(contains("\"manifest_contract\""))
        .stdout(contains("\"wrapper_package_compatibility\""))
        .stdout(contains("\"buyer_grade_readiness\""))
        .stdout(contains("\"enterprise_grade_readiness\""));

    let report = std::fs::read_to_string(build.path().join("reports/core-report.json")).unwrap();
    assert!(report.contains("\"maturity\""));
    assert!(report.contains("\"open_source_tool_evidence\""));
    assert!(report.contains("\"vendor_tool_evidence\""));
}

#[test]
fn evidence_ingest_normalizes_report_and_release_gate() {
    let build = tempdir().unwrap();
    let input_dir = tempdir().unwrap();
    let log = input_dir.path().join("sim.log");
    std::fs::write(&log, "PASS simulation completed\n").unwrap();

    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.arg("--build-root")
        .arg(build.path())
        .args(["evidence", "ingest"])
        .args(["--kind", "simulation-log"])
        .arg("--input")
        .arg(&log)
        .args(["--tool", "iverilog"])
        .args(["--core", "af_demo"])
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("\"kind\": \"accelfury.evidence_ingest\""))
        .stdout(contains("\"evidence_status\": \"passed\""))
        .stdout(contains("\"status\": \"satisfied\""))
        .stdout(contains("\"fingerprint_fnv1a64\""));

    let report = build
        .path()
        .join("reports/evidence/simulation_report-sim_log.json");
    let copied = build.path().join("evidence/simulation-log/sim_log");
    assert!(report.is_file());
    assert!(copied.is_file());

    let report_text = std::fs::read_to_string(report).unwrap();
    assert!(report_text.contains("\"tool\": \"iverilog\""));
    assert!(report_text.contains("\"release_gate\""));
}

#[test]
fn wrapper_generate_fusesoc_writes_core() {
    let root = repo_root();
    let build = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.arg("--build-root")
        .arg(build.path())
        .args(["wrapper", "generate"])
        .arg(root.join("examples/af-pdm-rx"))
        .args(["--target", "fusesoc", "--json"])
        .assert()
        .success()
        .stdout(contains("accelfury_audio_af_pdm_rx.core"));

    let core_file = build.path().join("fusesoc/accelfury_audio_af_pdm_rx.core");
    assert!(core_file.is_file());
    let content = std::fs::read_to_string(core_file).unwrap();
    assert!(content.starts_with("# Generated by AccelFury IP Toolchain"));
}

#[test]
fn wrapper_generate_litex_writes_skeleton() {
    let root = repo_root();
    let build = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.arg("--build-root")
        .arg(build.path())
        .args(["wrapper", "generate"])
        .arg(root.join("examples/af-pdm-rx"))
        .args(["--target", "litex", "--board", "tang-nano-20k", "--json"])
        .assert()
        .success()
        .stdout(contains("af_pdm_rx_litex.py"));

    let wrapper = build.path().join("litex/af_pdm_rx_litex.py");
    assert!(wrapper.is_file());
    let content = std::fs::read_to_string(wrapper).unwrap();
    assert!(content.starts_with("# Generated by AccelFury IP Toolchain"));
    assert!(content.contains("must not generate CDC"));
}

#[test]
fn wrapper_generate_ipxact_writes_skeleton() {
    let root = repo_root();
    let build = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.arg("--build-root")
        .arg(build.path())
        .args(["wrapper", "generate"])
        .arg(root.join("examples/af-pdm-rx"))
        .args(["--target", "ipxact", "--json"])
        .assert()
        .success()
        .stdout(contains("af-pdm-rx"));

    let artifacts: Vec<_> = std::fs::read_dir(build.path().join("ipxact"))
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .collect();
    assert_eq!(artifacts.len(), 1);
    let wrapper = &artifacts[0];
    assert!(wrapper.is_file());
    let content = std::fs::read_to_string(wrapper).unwrap();
    assert!(content.starts_with("<?xml version=\"1.0\""));
    assert!(content.contains("<spirit:component"));
    assert!(content.contains("<spirit:name>af-pdm-rx</spirit:name>"));
}

#[test]
fn wrapper_generate_stream_fifo_writes_ready_valid_adapter() {
    let fixture = tempdir().unwrap();
    let core = fixture.path().join("af-sync-fifo");
    write_fifo_core_fixture(&core);
    let build = tempdir().unwrap();

    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.arg("--build-root")
        .arg(build.path())
        .args(["wrapper", "generate"])
        .arg(&core)
        .args(["--target", "stream-fifo", "--json"])
        .assert()
        .success()
        .stdout(contains("af_sync_fifo_stream_fifo.v"));

    let wrapper = build.path().join("stream-fifo/af_sync_fifo_stream_fifo.v");
    assert!(wrapper.is_file());
    let text = std::fs::read_to_string(wrapper).unwrap();
    assert!(text.contains("assign s_ready      = !fifo_full_w || fifo_rd_en_w;"));
}

#[test]
fn manifest_validate_and_core_report_resolve_workspace_dependencies() {
    let fixture = tempdir().unwrap();
    std::fs::write(fixture.path().join("Cargo.toml"), "[workspace]\n").unwrap();
    let projects = fixture.path().join("projects");
    let producer = projects.join("producer");
    let consumer = projects.join("consumer");
    write_simple_core_fixture(&producer, "producer", None);
    write_simple_core_fixture(&consumer, "consumer", Some("../producer"));
    let build = tempdir().unwrap();

    let mut validate = Command::cargo_bin("af").unwrap();
    validate
        .arg("--build-root")
        .arg(build.path())
        .args(["manifest", "validate"])
        .arg(consumer.join("af-core.toml"))
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("\"dependency_resolutions\""))
        .stdout(contains("accelfury:ip:producer:0.1.0"));

    let mut report = Command::cargo_bin("af").unwrap();
    report
        .arg("--build-root")
        .arg(build.path())
        .args(["core", "report"])
        .arg(&consumer)
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("dependency:accelfury:ip:producer:0.1.0"));
}

#[test]
fn manifest_validate_resolves_workspace_dependencies_from_core_cwd() {
    let fixture = tempdir().unwrap();
    std::fs::write(fixture.path().join("Cargo.toml"), "[workspace]\n").unwrap();
    let projects = fixture.path().join("projects");
    let producer = projects.join("producer");
    let consumer = projects.join("consumer");
    write_simple_core_fixture(&producer, "producer", None);
    write_simple_core_fixture(&consumer, "consumer", Some("../producer"));
    let build = tempdir().unwrap();

    let run_manifest_validate = |cwd: &Path, manifest_path: &str| -> serde_json::Value {
        let output = Command::cargo_bin("af")
            .unwrap()
            .current_dir(cwd)
            .arg("--build-root")
            .arg(build.path())
            .args(["manifest", "validate", manifest_path, "--json"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "manifest validate {manifest_path} from {} failed\nstdout:\n{}\nstderr:\n{}",
            cwd.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    };

    let from_workspace = run_manifest_validate(fixture.path(), "projects/consumer/af-core.toml");
    let from_core = run_manifest_validate(&consumer, "af-core.toml");

    assert_eq!(from_workspace, from_core);
}

#[test]
fn board_check_and_backend_list_work() {
    let root = repo_root();
    let mut board = Command::cargo_bin("af").unwrap();
    board
        .args(["board", "check"])
        .arg(root.join("boards/tang-nano-20k/af-board.toml"))
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("\"id\": \"tang-nano-20k\""));

    let mut backend = Command::cargo_bin("af").unwrap();
    backend
        .args(["backend", "list", "--json"])
        .assert()
        .success()
        .stdout(contains("native-portable-core-check"))
        .stdout(contains("iverilog-elaboration"))
        .stdout(contains("litex-wrapper-skeleton"))
        .stdout(contains("ipxact-wrapper-skeleton"))
        .stdout(contains("yosys-syntax-smoke"))
        .stdout(contains("sby-formal"))
        .stdout(contains("nextpnr-report-capture"));

    let mut nextpnr = Command::cargo_bin("af").unwrap();
    nextpnr
        .args(["backend", "run", "nextpnr", "--target", "doctor", "--json"])
        .assert()
        .success()
        .stdout(contains("\"backend\": \"nextpnr\""));
}

#[test]
fn registry_vectors_and_board_scaffold_workflows_are_covered() {
    let root = repo_root();

    let mut registry = Command::cargo_bin("af").unwrap();
    registry
        .current_dir(&root)
        .args(["registry", "check", "--json"])
        .assert()
        .success()
        .stdout(contains("\"status\": \"passed\""))
        .stdout(contains("\"board_count\""));

    let dir = tempdir().unwrap();
    let matrix_out = dir.path().join("board_matrix.md");
    let mut matrix = Command::cargo_bin("af").unwrap();
    matrix
        .current_dir(&root)
        .args(["board", "matrix"])
        .args(["--output"])
        .arg(&matrix_out)
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("\"status\": \"passed\""))
        .stdout(contains("board_matrix.md"));
    let matrix_text = std::fs::read_to_string(&matrix_out).unwrap();
    assert!(matrix_text.starts_with("# AccelFury Board Matrix"));
    assert!(matrix_text.contains("Generated from `registries/boards.registry.json`"));
    assert!(matrix_text.contains("sipeed_tang_nano_20k"));

    let basic_out = dir.path().join("vectors/basic.json");
    let random_out = dir.path().join("vectors/random.json");
    let svh_out = dir.path().join("vectors/random.svh");
    let mut vectors = Command::cargo_bin("af").unwrap();
    vectors
        .args(["vectors", "generate"])
        .args(["--basic-out"])
        .arg(&basic_out)
        .args(["--random-out"])
        .arg(&random_out)
        .args(["--svh-out"])
        .arg(&svh_out)
        .args(["--count", "3", "--seed", "0x2A", "--json"])
        .assert()
        .success()
        .stdout(contains("\"status\": \"passed\""))
        .stdout(contains("\"random_count\": 3"));
    assert!(basic_out.is_file());
    assert!(random_out.is_file());
    assert!(svh_out.is_file());

    let board_root = dir.path().join("board-root");
    std::fs::create_dir_all(board_root.join("registries")).unwrap();
    std::fs::write(
        board_root.join("registries/boards.registry.json"),
        "{\"boards\":[]}\n",
    )
    .unwrap();
    let mut board_new = Command::cargo_bin("af").unwrap();
    board_new
        .args(["board", "new"])
        .args(["--root"])
        .arg(&board_root)
        .args([
            "--board-id",
            "demo_board",
            "--vendor",
            "demo_vendor",
            "--family",
            "demo_family",
            "--constraint-format",
            "pcf",
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"board_id\": \"demo_board\""));
    assert!(board_root
        .join("boards/demo_vendor/demo_board/constraints/pins.pcf")
        .is_file());

    let mut generated_registry = Command::cargo_bin("af").unwrap();
    generated_registry
        .args(["registry", "check"])
        .args(["--root"])
        .arg(&board_root)
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("\"board_count\": 1"));
}

#[test]
fn board_new_reports_structured_unsupported_constraint_format() {
    let dir = tempdir().unwrap();
    let mut board_new = Command::cargo_bin("af").unwrap();
    board_new
        .args(["board", "new"])
        .args(["--root"])
        .arg(dir.path())
        .args([
            "--board-id",
            "bad_board",
            "--vendor",
            "demo",
            "--family",
            "demo",
            "--constraint-format",
            "badfmt",
            "--json",
        ])
        .assert()
        .failure()
        .stdout(contains(
            "\"code\": \"AF_BOARD_CONSTRAINT_FORMAT_UNSUPPORTED\"",
        ));
}

#[test]
fn ci_generate_filters_backends_and_includes_matrix() {
    let workdir = tempdir().unwrap();
    let output = workdir.path().join("accelfury.yml");
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.args(["ci", "generate"])
        .args(["--target", "github-actions"])
        .args(["--backends", "native,verilator"])
        .args(["--output", output.to_str().unwrap(), "--json"])
        .assert()
        .success()
        .stdout(contains("\"status\": \"passed\""));

    let workflow = std::fs::read_to_string(&output).unwrap();
    assert!(workflow.contains("- backend: native"));
    assert!(workflow.contains("- backend: verilator"));
    assert!(!workflow.contains("- backend: yosys"));
    assert!(workflow.contains("continue-on-error: ${{ matrix.optional }}"));

    let fail_closed = workdir.path().join("accelfury_fail_closed.yml");
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.args(["ci", "generate"])
        .args(["--target", "github-actions"])
        .args(["--backends", "native,verilator"])
        .args(["--optional-fail-closed"])
        .args(["--output", fail_closed.to_str().unwrap(), "--json"])
        .assert()
        .success();
    let workflow = std::fs::read_to_string(&fail_closed).unwrap();
    assert!(!workflow.contains("continue-on-error: ${{ matrix.optional }}"));
}

#[test]
fn core_new_supports_verilog_2001_scaffold() {
    let dir = tempdir().unwrap();
    let core_dir = dir.path().join("verilog-demo");
    let build = tempdir().unwrap();

    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.arg("--build-root")
        .arg(build.path())
        .args(["core", "new"])
        .arg(&core_dir)
        .args(["--name", "verilog-demo", "--language", "verilog", "--json"])
        .assert()
        .success()
        .stdout(contains("\"language\": \"verilog-2001\""))
        .stdout(contains("\"project_class\": \"simple-portable\""))
        .stdout(contains("AF_COMPLEXITY_CLASS_INFERRED"));

    assert!(core_dir.join("af-core.toml").is_file());
    assert!(core_dir.join("rtl/verilog_demo.v").is_file());
    assert!(core_dir.join("artifacts/openfpga-ci/README.md").is_file());

    let mut check = Command::cargo_bin("af").unwrap();
    check
        .arg("--build-root")
        .arg(build.path())
        .args(["core", "check"])
        .arg(&core_dir)
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("\"status\": \"passed\""));
}

#[test]
fn core_tooling_writes_smt_solver_artifacts_into_project() {
    let dir = tempdir().unwrap();
    let core_dir = dir.path().join("solver-demo");
    let build = tempdir().unwrap();
    let empty_path = tempdir().unwrap();

    let mut new_core = Command::cargo_bin("af").unwrap();
    new_core
        .arg("--build-root")
        .arg(build.path())
        .args(["core", "new"])
        .arg(&core_dir)
        .args(["--name", "solver-demo", "--json"])
        .assert()
        .success();

    let mut tooling = Command::cargo_bin("af").unwrap();
    tooling
        .arg("--build-root")
        .arg(build.path())
        .env("PATH", empty_path.path())
        .args(["core", "tooling"])
        .arg(&core_dir)
        .arg("--json")
        .assert()
        .success()
        .stdout(contains(
            "\"kind\": \"accelfury.core_development_tooling_check\"",
        ))
        .stdout(contains("\"status\": \"warning\""))
        .stdout(contains("\"id\": \"smt_solvers\""))
        .stdout(contains("\"id\": \"package_integration\""))
        .stdout(contains("\"tool\": \"boolector\""))
        .stdout(contains("\"tool\": \"z3\""))
        .stdout(contains("\"tool\": \"yices-smt2\""))
        .stdout(contains("\"tool\": \"bitwuzla\""))
        .stdout(contains("\"tool\": \"cvc5\""))
        .stdout(contains("\"tool\": \"xmllint\""))
        .stdout(contains("\"tool\": \"fusesoc\""))
        .stdout(contains("\"tool\": \"edalize\""))
        .stdout(contains("core-tooling.json"))
        .stdout(contains("core-smt-solvers.json"))
        .stdout(contains("core-integration-tools.json"))
        .stdout(contains("smt-solvers-tool-versions.txt"))
        .stdout(contains("integration-tool-versions.txt"));

    assert!(core_dir
        .join("artifacts/openfpga-ci/reports/core-tooling.json")
        .is_file());
    assert!(core_dir
        .join("artifacts/openfpga-ci/reports/core-smt-solvers.json")
        .is_file());
    assert!(core_dir
        .join("artifacts/openfpga-ci/reports/core-integration-tools.json")
        .is_file());
    assert!(core_dir
        .join("artifacts/openfpga-ci/logs/smt-solvers-tool-versions.txt")
        .is_file());
    assert!(core_dir
        .join("artifacts/openfpga-ci/logs/integration-tool-versions.txt")
        .is_file());
    assert!(build.path().join("reports/core-tooling.json").is_file());
    assert!(build.path().join("reports/core-smt-solvers.json").is_file());
    assert!(build
        .path()
        .join("reports/core-integration-tools.json")
        .is_file());

    let text = std::fs::read_to_string(
        core_dir.join("artifacts/openfpga-ci/logs/smt-solvers-tool-versions.txt"),
    )
    .unwrap();
    assert!(text.contains("boolector: missing"));
    assert!(text.contains("cvc5: missing"));
    let integration_text = std::fs::read_to_string(
        core_dir.join("artifacts/openfpga-ci/logs/integration-tool-versions.txt"),
    )
    .unwrap();
    assert!(integration_text.contains("xmllint: missing"));
    assert!(integration_text.contains("edalize: missing"));
}

#[test]
fn core_sim_af_mod_add_reports_unavailable_without_verilator() {
    let root = repo_root();
    let build = tempdir().unwrap();
    let empty_path = tempdir().unwrap();

    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(root)
        .env("PATH", empty_path.path())
        .arg("--build-root")
        .arg(build.path())
        .args([
            "core",
            "sim",
            "examples/af-mod-add",
            "--backend",
            "verilator",
        ])
        .arg("--json")
        .assert()
        .failure()
        .stdout(contains("\"code\": \"AF_BACKEND_UNAVAILABLE\""))
        .stdout(contains("\"backend\": \"verilator\""))
        .stdout(contains("\"status\": \"Unavailable\""));
}

#[test]
fn complexity_workflow_generates_complex_template_and_reports() {
    let dir = tempdir().unwrap();
    let core_dir = dir.path().join("af-ntt");
    let constructor_dir = dir.path().join("constructor-export");
    let build = tempdir().unwrap();

    let mut create = Command::cargo_bin("af").unwrap();
    create
        .arg("--build-root")
        .arg(build.path())
        .args(["core", "new"])
        .arg(&core_dir)
        .args([
            "--name",
            "af-ntt",
            "--class",
            "complex-vendor-aware",
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"project_class\": \"complex-vendor-aware\""))
        .stdout(contains("vendor_dsp_backend_required"));

    assert!(core_dir.join("af-arch.toml").is_file());
    assert!(core_dir.join("rtl/common/af_ntt_core.v").is_file());
    assert!(core_dir.join("rtl/generic/af_ntt_work_ram.v").is_file());
    assert!(core_dir.join("vendor/README.md").is_file());

    let mut check = Command::cargo_bin("af").unwrap();
    check
        .arg("--build-root")
        .arg(build.path())
        .args(["core", "check"])
        .arg(&core_dir)
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("\"portable_verilog_policy\": \"pass\""));

    let mut classify = Command::cargo_bin("af").unwrap();
    classify
        .args(["project", "classify"])
        .arg(&core_dir)
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("\"project_class\": \"complex-vendor-aware\""))
        .stdout(contains("AF_COMPLEXITY_UNDERMODELED").not());

    let mut arch = Command::cargo_bin("af").unwrap();
    arch.args(["architecture", "check"])
        .arg(&core_dir)
        .arg("--json")
        .assert()
        .success()
        // Complex-vendor-aware scaffold ships with declared verification gates
        // and no evidence yet, so architecture check returns "warning" status
        // (AF_VERIFICATION_EVIDENCE_PLANNED) without raising issues.
        .stdout(contains("\"status\": \"warning\""))
        .stdout(contains("AF_VERIFICATION_EVIDENCE_PLANNED"))
        .stdout(contains("\"issues\": []"));

    let mut resource = Command::cargo_bin("af").unwrap();
    resource
        .args(["resource", "plan"])
        .arg(&core_dir)
        .args([
            "--vendor",
            "xilinx",
            "--family",
            "ultrascale-plus",
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"bram\""))
        .stdout(contains("\"dsp\""))
        .stdout(contains("\"policy\": \"require_vendor\""));

    let mut constructor = Command::cargo_bin("af").unwrap();
    constructor
        .args(["constructor", "export"])
        .arg(&core_dir)
        .args(["--output"])
        .arg(&constructor_dir)
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("limitations.json"));
    assert!(constructor_dir.join("core.json").is_file());
    assert!(constructor_dir.join("resources.json").is_file());

    let mut signoff = Command::cargo_bin("af").unwrap();
    signoff
        .args(["signoff", "plan"])
        .arg(&core_dir)
        .args(["--class", "complex-vendor-aware", "--json"])
        .assert()
        .success()
        .stdout(contains("backend-equivalence"))
        .stdout(contains("\"status\": \"planned\""));

    let mut backend = Command::cargo_bin("af").unwrap();
    backend
        .args(["backend", "scaffold"])
        .arg(&core_dir)
        .args([
            "--vendor",
            "xilinx",
            "--family",
            "ultrascale-plus",
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("no fake working RTL"));
    assert!(core_dir.join("vendor/xilinx/backend.toml").is_file());

    let mut deps = Command::cargo_bin("af").unwrap();
    deps.args(["dependency", "graph"])
        .arg(&core_dir)
        .args(["--format", "dot", "--json"])
        .assert()
        .success()
        .stdout(contains("af-stream-skid-buffer"));
}

#[test]
fn architecture_check_rejects_vendor_leakage_in_common_layer() {
    let dir = tempdir().unwrap();
    let core_dir = dir.path().join("bad-layer");

    let mut create = Command::cargo_bin("af").unwrap();
    create
        .args(["core", "new"])
        .arg(&core_dir)
        .args([
            "--name",
            "bad-layer",
            "--class",
            "complex-vendor-aware",
            "--json",
        ])
        .assert()
        .success();

    let common = core_dir.join("rtl/common/bad_layer_core.v");
    let mut text = std::fs::read_to_string(&common).unwrap();
    text.push_str("\n// RAMB36E2 must stay out of common\n");
    std::fs::write(&common, text).unwrap();

    let mut arch = Command::cargo_bin("af").unwrap();
    arch.args(["architecture", "check"])
        .arg(&core_dir)
        .arg("--json")
        .assert()
        .failure()
        .stdout(contains("\"code\": \"AF_ARCH_LAYER_VIOLATION\""))
        .stdout(contains("AF_ARCH_LAYER_VIOLATION"));
}

#[test]
fn project_new_generates_system_platform_skeleton() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path().join("pcie-ntt");

    let mut create = Command::cargo_bin("af").unwrap();
    create
        .args(["project", "new"])
        .arg(&project_dir)
        .args(["--class", "system-platform", "--name", "pcie-ntt", "--json"])
        .assert()
        .success()
        .stdout(contains("\"project_class\": \"system-platform\""));

    assert!(project_dir.join("af-project.toml").is_file());
    assert!(project_dir.join("platforms/README.md").is_file());

    let mut classify = Command::cargo_bin("af").unwrap();
    classify
        .args(["project", "classify"])
        .arg(&project_dir)
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("\"project_class\": \"system-platform\""));
}

#[test]
fn project_new_generates_product_stack_skeleton() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path().join("zk-stack");

    let mut create = Command::cargo_bin("af").unwrap();
    create
        .args(["project", "new"])
        .arg(&project_dir)
        .args(["--class", "product-stack", "--name", "zk-stack", "--json"])
        .assert()
        .success()
        .stdout(contains("\"project_class\": \"product-stack\""));

    assert!(project_dir.join("af-product.toml").is_file());
    assert!(project_dir.join("constructor_catalog/README.md").is_file());

    let mut classify = Command::cargo_bin("af").unwrap();
    classify
        .args(["project", "classify"])
        .arg(&project_dir)
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("\"project_class\": \"product-stack\""));
}

#[test]
fn compatibility_check_reports_conflicts_and_adapters() {
    let dir = tempdir().unwrap();
    let left = dir.path().join("left-core");
    let right = dir.path().join("right-core");
    write_compat_core(&left, "left_core", "ready_valid", "32", "clk_a", "low");
    write_compat_core(&right, "right_core", "valid_only", "64", "clk_b", "high");

    let mut check = Command::cargo_bin("af").unwrap();
    check
        .args(["compatibility", "check"])
        .arg(&left)
        .arg(&right)
        .arg("--json")
        .assert()
        .failure()
        .stdout(contains("\"code\": \"AF_COMPAT_PROTOCOL_MISMATCH\""))
        .stdout(contains("AF_COMPAT_CLOCK_MISMATCH"))
        .stdout(contains("stream_width_adapter"))
        .stdout(contains("async_fifo_cdc"))
        .stdout(contains("reset_polarity_adapter"));
}

#[test]
fn compatibility_check_warns_on_unqualified_drop_in_replacement() {
    let dir = tempdir().unwrap();
    let left = dir.path().join("naive-core");
    let right = dir.path().join("good-core");
    write_compat_core_with_description(
        &left,
        "naive_core",
        "ready_valid",
        "32",
        "clk_a",
        "low",
        "drop-in replacement for Xilinx FIFO Generator",
    );
    write_compat_core(&right, "good_core", "ready_valid", "32", "clk_a", "low");

    let mut check = Command::cargo_bin("af").unwrap();
    check
        .args(["compatibility", "check"])
        .arg(&left)
        .arg(&right)
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("AF_COMPATIBILITY_OVERPROMISING_CLAIM"));
}

#[test]
fn compatibility_check_accepts_qualified_replacement_language() {
    let dir = tempdir().unwrap();
    let left = dir.path().join("quoted-core");
    let right = dir.path().join("good-core");
    write_compat_core_with_description(
        &left,
        "quoted_core",
        "ready_valid",
        "32",
        "clk_a",
        "low",
        "behavioral equivalent of FIFO Generator after verification (compatibility wrapper)",
    );
    write_compat_core(&right, "good_core", "ready_valid", "32", "clk_a", "low");

    let mut check = Command::cargo_bin("af").unwrap();
    check
        .args(["compatibility", "check"])
        .arg(&left)
        .arg(&right)
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("AF_COMPATIBILITY_OVERPROMISING_CLAIM").not());
}

fn write_compat_core_with_description(
    dir: &Path,
    core: &str,
    protocol: &str,
    width: &str,
    clock: &str,
    reset_active: &str,
    description: &str,
) {
    write_compat_core(dir, core, protocol, width, clock, reset_active);
    // Inject [metadata].description after the fact. The fixture writer above
    // does not declare metadata, so we append a fresh table to the manifest.
    let manifest_path = dir.join("af-core.toml");
    let mut content = std::fs::read_to_string(&manifest_path).unwrap();
    content.push_str(&format!("\n[metadata]\ndescription = \"{description}\"\n"));
    std::fs::write(&manifest_path, content).unwrap();
}

fn write_compat_core(
    dir: &Path,
    core: &str,
    protocol: &str,
    width: &str,
    clock: &str,
    reset_active: &str,
) {
    std::fs::create_dir_all(dir.join("rtl")).unwrap();
    std::fs::write(
        dir.join("rtl").join(format!("{core}.v")),
        format!(
            "`default_nettype none\nmodule {core} #(\n  parameter DATA_WIDTH = {width}\n) (\n  input  wire clk,\n  input  wire rst_n,\n  input  wire [DATA_WIDTH-1:0] data,\n  input  wire valid,\n  output wire ready\n);\n  assign ready = 1'b1;\nendmodule\n`default_nettype wire\n",
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("af-core.toml"),
        format!(
            r#"
af_version = "0.2"
name = "{core}"
vendor = "accelfury"
library = "ip"
core = "{core}"
version = "0.1.0"

[rtl]
top = "{core}"
language = "verilog-2001"

[sources]
files = ["rtl/{core}.v"]

[[parameters]]
name = "DATA_WIDTH"
value = "{width}"

[[clocks]]
name = "{clock}"
port = "clk"

[[resets]]
name = "rst_n"
port = "rst_n"
active = "{reset_active}"
clock_domain = "{clock}"

[[ports]]
name = "clk"
direction = "input"
width = 1

[[ports]]
name = "rst_n"
direction = "input"
width = 1

[[ports]]
name = "data"
direction = "input"
width = "DATA_WIDTH"
clock = "{clock}"
reset = "rst_n"

[[ports]]
name = "valid"
direction = "input"
width = 1
clock = "{clock}"
reset = "rst_n"

[[ports]]
name = "ready"
direction = "output"
width = 1
clock = "{clock}"
reset = "rst_n"

[[stream_interfaces]]
name = "stream"
kind = "{protocol}"
clock_domain = "{clock}"
data = "data"
valid = "valid"
ready = "ready"
data_width = "{width}"
"#,
        ),
    )
    .unwrap();
}

fn write_simple_core_fixture(dir: &Path, core: &str, dependency_path: Option<&str>) {
    std::fs::create_dir_all(dir.join("rtl")).unwrap();
    write_legal_files(dir);
    std::fs::write(
        dir.join("rtl").join(format!("{core}.v")),
        format!(
            "`default_nettype none\nmodule {core}(input wire clk, input wire rst); endmodule\n`default_nettype wire\n"
        ),
    )
    .unwrap();
    let dependency = dependency_path
        .map(|path| {
            format!(
                r#"
[[dependencies.cores]]
name = "producer"
version = ">=0.1.0"
role = "test_dependency"
path = "{path}"
"#
            )
        })
        .unwrap_or_default();
    std::fs::write(
        dir.join("af-core.toml"),
        format!(
            r#"
af_version = "0.3"
name = "{core}"
vendor = "accelfury"
library = "ip"
core = "{core}"
version = "0.1.0"
known_limitations = ["test limitation"]

[metadata]
license = "AccelFury Source Available License v1.0"

[rtl]
top = "{core}"
language = "verilog-2001"

[sources]
files = ["rtl/{core}.v"]

{dependency}
[[clocks]]
name = "clk"
port = "clk"

[[resets]]
name = "rst"
port = "rst"
active = "high"

[[ports]]
name = "clk"
direction = "input"
width = 1

[[ports]]
name = "rst"
direction = "input"
width = 1
"#
        ),
    )
    .unwrap();
}

fn write_fifo_core_fixture(dir: &Path) {
    std::fs::create_dir_all(dir.join("rtl")).unwrap();
    write_legal_files(dir);
    std::fs::write(
        dir.join("af-core.toml"),
        r#"
af_version = "0.3"
name = "af-sync-fifo"
vendor = "accelfury"
library = "ip"
core = "af_sync_fifo"
version = "0.1.0"

[metadata]
license = "AccelFury Source Available License v1.0"

[rtl]
top = "af_sync_fifo"
language = "verilog-2001"

[sources]
files = ["rtl/af_sync_fifo.v"]

[[parameters]]
name = "DATA_BITS"
value = "32"

[[parameters]]
name = "FIFO_ADDR_BITS"
value = "4"

[[parameters]]
name = "ALMOST_FULL_TH"
value = "(1 << FIFO_ADDR_BITS) - 2"

[[clocks]]
name = "clk"
port = "clk"

[[resets]]
name = "rst"
port = "rst"
active = "high"

[[ports]]
name = "clk"
direction = "input"
width = 1

[[ports]]
name = "rst"
direction = "input"
width = 1

[[ports]]
name = "clear"
direction = "input"
width = 1

[[ports]]
name = "wr_en"
direction = "input"
width = 1

[[ports]]
name = "wr_data"
direction = "input"
width = "DATA_BITS"

[[ports]]
name = "full"
direction = "output"
width = 1

[[ports]]
name = "almost_full"
direction = "output"
width = 1

[[ports]]
name = "rd_en"
direction = "input"
width = 1

[[ports]]
name = "rd_data"
direction = "output"
width = "DATA_BITS"

[[ports]]
name = "empty"
direction = "output"
width = 1

[[ports]]
name = "level"
direction = "output"
width = "FIFO_ADDR_BITS + 1"

[contracts.fifo]
kind = "single_clock"
interface = "wr_rd_control"
read_mode = "first_word_fall_through"
full_write_policy = "accept_when_full_with_read"
clear_behavior = "sync_flush"
overflow_policy = "backpressure_no_drop"
"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("rtl/af_sync_fifo.v"),
        r#"`default_nettype none
module af_sync_fifo #(
    parameter DATA_BITS = 32,
    parameter FIFO_ADDR_BITS = 4,
    parameter ALMOST_FULL_TH = (1 << FIFO_ADDR_BITS) - 2
) (
    input wire clk,
    input wire rst,
    input wire clear,
    input wire wr_en,
    input wire [DATA_BITS-1:0] wr_data,
    output wire full,
    output wire almost_full,
    input wire rd_en,
    output wire [DATA_BITS-1:0] rd_data,
    output wire empty,
    output wire [FIFO_ADDR_BITS:0] level
);
endmodule
`default_nettype wire
"#,
    )
    .unwrap();
}

fn write_legal_files(dir: &Path) {
    std::fs::write(
        dir.join("LICENSE"),
        "AccelFury Source Available License v1.0\n\nCopyright (c) 2026 AccelFury.\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("COMMERCIAL-LICENSE.md"),
        "# Commercial Licensing\n\nClosed-source and commercial use requires a separate paid commercial license from AccelFury.\nCommercial triggers include closed-source FPGA products and proprietary repositories.\nContact AccelFury for commercial terms, support, warranty options, and custom integration work.\n",
    )
    .unwrap();
}

#[test]
fn core_new_rejects_systemverilog_base_scaffold() {
    let dir = tempdir().unwrap();
    let core_dir = dir.path().join("sv-demo");

    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.args(["core", "new"])
        .arg(&core_dir)
        .args(["--name", "sv-demo", "--language", "systemverilog", "--json"])
        .assert()
        .failure()
        .stdout(contains("\"code\": \"AF_CORE_NEW_LANGUAGE_UNSUPPORTED\""))
        .stdout(contains("portable Verilog-2001"));
}

#[test]
fn native_backend_lints_generated_portable_core_without_external_tools() {
    let dir = tempdir().unwrap();
    let core_dir = dir.path().join("native-demo");
    let build = tempdir().unwrap();

    let mut create = Command::cargo_bin("af").unwrap();
    create
        .arg("--build-root")
        .arg(build.path())
        .args(["core", "new"])
        .arg(&core_dir)
        .args(["--name", "native-demo", "--json"])
        .assert()
        .success();

    let mut lint = Command::cargo_bin("af").unwrap();
    lint.arg("--build-root")
        .arg(build.path())
        .args(["core", "lint"])
        .arg(&core_dir)
        .args(["--backend", "native", "--json"])
        .assert()
        .success()
        .stdout(contains("\"backend\": \"native\""))
        .stdout(contains("\"portable_verilog_policy\": \"pass\""));

    let mut run = Command::cargo_bin("af").unwrap();
    run.arg("--build-root")
        .arg(build.path())
        .args(["backend", "run", "native", "--target", "portable-check"])
        .arg("--core-dir")
        .arg(&core_dir)
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("\"backend\": \"native\""));
}

#[test]
fn native_backend_reports_portable_core_diagnostics() {
    let dir = tempdir().unwrap();
    let core_dir = dir.path().join("bad-native-demo");
    let rtl_dir = core_dir.join("rtl");
    let build = tempdir().unwrap();
    std::fs::create_dir_all(&rtl_dir).unwrap();
    std::fs::write(
        core_dir.join("af-core.toml"),
        r#"
af_version = "0.2"
name = "bad-native-demo"
vendor = "accelfury"
library = "ip"
core = "bad_native_demo"
version = "0.1.0"

[rtl]
top = "bad_native_demo"
language = "verilog-2001"
default_clock = "clk"
default_reset = "rst"

[sources]
files = ["rtl/bad_native_demo.v"]

[[clocks]]
name = "sys"
port = "clk"

[[resets]]
name = "sys_rst"
port = "rst"
active = "high"
clock_domain = "sys"
style = "sync"

[[ports]]
name = "clk"
direction = "input"
width = 1

[[ports]]
name = "rst"
direction = "input"
width = 1
"#,
    )
    .unwrap();
    std::fs::write(
        rtl_dir.join("bad_native_demo.v"),
        r#"`default_nettype none
module bad_native_demo (
  input logic clk,
  input wire rst
);
endmodule
`default_nettype wire
"#,
    )
    .unwrap();

    let mut lint = Command::cargo_bin("af").unwrap();
    lint.arg("--build-root")
        .arg(build.path())
        .args(["core", "lint"])
        .arg(&core_dir)
        .args(["--backend", "native", "--json"])
        .assert()
        .failure()
        .stdout(contains("\"code\": \"AF_LINT_FAILED\""))
        .stdout(contains("AF_PORTABLE_SYSTEMVERILOG_CONSTRUCT"));
}

#[test]
fn manifest_migrate_rejects_invalid_v02_result() {
    let dir = tempdir().unwrap();
    let manifest = dir.path().join("af-core.toml");
    std::fs::write(
        &manifest,
        r#"
af_version = "0.1"
name = "bad-migrate-demo"
vendor = "accelfury"
library = "ip"
core = "bad_migrate_demo"
version = "0.1.0"

[rtl]
top = "bad_migrate_demo"

[sources]
files = ["rtl/bad_migrate_demo.v"]
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.args(["manifest", "migrate"])
        .arg(&manifest)
        .args(["--from", "0.1", "--to", "0.2", "--json"])
        .assert()
        .failure()
        .stdout(contains("\"code\": \"AF_MANIFEST_INVALID\""))
        .stdout(contains("\"message\": \"manifest validation failed\""));
}

#[test]
fn core_new_reset_sync_profile_generates_canonical_verilog_core() {
    let dir = tempdir().unwrap();
    let core_dir = dir.path().join("af-reset-sync");
    let build = tempdir().unwrap();

    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.arg("--build-root")
        .arg(build.path())
        .args(["core", "new"])
        .arg(&core_dir)
        .args([
            "--name",
            "af-reset-sync",
            "--language",
            "verilog",
            "--profile",
            "reset-sync",
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"profile\": \"reset-sync\""))
        .stdout(contains("\"portability_level\": \"U0\""));

    let rtl = core_dir.join("rtl/af_reset_sync.v");
    assert!(core_dir.join("af-core.toml").is_file());
    assert!(core_dir.join("LICENSE").is_file());
    assert!(core_dir.join("COMMERCIAL-LICENSE.md").is_file());
    assert!(core_dir.join("NOTICE").is_file());
    assert!(rtl.is_file());
    // Canonical form keeps polarity inside the single module via RESET_POLARITY,
    // so the historical `<name>_n.v` wrapper should NOT exist anymore.
    assert!(!core_dir.join("rtl/af_reset_sync_n.v").exists());

    let manifest = std::fs::read_to_string(core_dir.join("af-core.toml")).unwrap();
    assert!(manifest.contains("AccelFury Source Available License v1.0"));
    assert!(manifest.contains("portability_level = \"U0\""));
    assert!(manifest.contains("priority = \"P0\""));
    assert!(manifest.contains("RESET_POLARITY"));
    assert!(manifest.contains("[[verification_required]]"));

    let content = std::fs::read_to_string(rtl).unwrap();
    assert!(content.contains("`default_nettype none"));
    // Both polarity arms must be present in the generate block.
    assert!(content.contains("always @(posedge clk or negedge src_rst)"));
    assert!(content.contains("always @(posedge clk or posedge src_rst)"));
    assert!(!content.contains("always_ff"));
    assert!(!content.contains("logic"));

    let mut check = Command::cargo_bin("af").unwrap();
    check
        .arg("--build-root")
        .arg(build.path())
        .args(["core", "check"])
        .arg(&core_dir)
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("\"portable_verilog_policy\": \"pass\""));
}

#[test]
fn core_registry_list_filters_by_priority() {
    let root = repo_root();

    let mut all = Command::cargo_bin("af").unwrap();
    all.current_dir(&root)
        .args(["core", "registry", "list", "--json"])
        .assert()
        .success()
        .stdout(contains("\"status\": \"passed\""))
        .stdout(contains("\"core_id\": \"af_reset_sync\""));

    let mut p0 = Command::cargo_bin("af").unwrap();
    p0.current_dir(&root)
        .args(["core", "registry", "list", "--priority", "P0", "--json"])
        .assert()
        .success()
        // The shipped registry has multiple P0 cores; this one must appear.
        .stdout(contains("\"priority\": \"P0\""))
        .stdout(contains("\"core_id\": \"af_reset_sync\""))
        // af_pdm_rx is P2 and must be filtered out by --priority P0.
        .stdout(contains("\"af_pdm_rx\"").not());

    let mut u1 = Command::cargo_bin("af").unwrap();
    u1.current_dir(&root)
        .args(["core", "registry", "list", "--portability", "U1", "--json"])
        .assert()
        .success()
        .stdout(contains("\"portability_level\": \"U1\""))
        .stdout(contains("\"af_reset_sync\"").not());
}

#[test]
fn registry_check_includes_cores_registry() {
    let root = repo_root();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(&root)
        .args(["registry", "check", "--json"])
        .assert()
        .success()
        .stdout(contains("\"cores_registry\""))
        .stdout(contains("\"status\": \"passed\""))
        // 15 cores after Iteration 1+2; +6 field-arithmetic lineage in Iter 3.
        .stdout(contains("\"core_count\": 21"));
}

#[test]
fn core_check_accepts_canonical_reset_sync_example() {
    let root = repo_root();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(&root)
        .args(["core", "check", "examples/af-reset-sync", "--json"])
        .assert()
        .success()
        .stdout(contains("\"status\": \"passed\""))
        .stdout(contains("\"portable_verilog_policy\": \"pass\""));
}

#[test]
fn architecture_check_emits_planned_warning_for_verification_gates_without_evidence() {
    let root = repo_root();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(&root)
        .args(["architecture", "check", "examples/af-reset-sync", "--json"])
        .assert()
        // The reset-sync example declares two verification_required gates with
        // no evidence path; both must surface as planned-warnings.
        .stdout(contains("AF_VERIFICATION_EVIDENCE_PLANNED"));
}

#[test]
fn core_check_detects_implicit_reset_in_native_backend() {
    let dir = tempdir().unwrap();
    let core_dir = dir.path().join("implicit-reset-core");
    std::fs::create_dir_all(core_dir.join("rtl")).unwrap();
    std::fs::write(
        core_dir.join("af-core.toml"),
        r#"af_version = "0.3"
name = "implicit-reset"
vendor = "accelfury"
library = "tests"
core = "implicit_reset"
version = "0.1.0"

[rtl]
top = "implicit_reset"
language = "verilog-2001"

[sources]
files = ["rtl/implicit_reset.v"]

[[clocks]]
name = "clk"
port = "clk"

[[resets]]
name = "rst_n"
port = "rst_n"
active = "low"

[[ports]]
name = "clk"
direction = "input"
width = 1

[[ports]]
name = "rst_n"
direction = "input"
width = 1

[[ports]]
name = "done"
direction = "output"
width = 1
"#,
    )
    .unwrap();
    std::fs::write(
        core_dir.join("rtl/implicit_reset.v"),
        r#"`default_nettype none
module implicit_reset (
  input wire clk,
  input wire rst_n,
  output reg done
);
  initial begin
    done = 1'b0;
  end
  always @(posedge clk or negedge rst_n) begin
    if (!rst_n) done <= 1'b0;
    else        done <= 1'b1;
  end
endmodule
`default_nettype wire
"#,
    )
    .unwrap();

    let build = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.arg("--build-root")
        .arg(build.path())
        .args(["core", "check"])
        .arg(&core_dir)
        .arg("--json")
        .assert()
        .failure()
        .stdout(contains("AF_PORTABLE_IMPLICIT_RESET"));
}

#[test]
fn core_check_detects_encrypted_netlist_extension() {
    let dir = tempdir().unwrap();
    let core_dir = dir.path().join("encrypted-core");
    std::fs::create_dir_all(core_dir.join("rtl")).unwrap();
    std::fs::write(
        core_dir.join("af-core.toml"),
        r#"af_version = "0.3"
name = "encrypted"
vendor = "accelfury"
library = "tests"
core = "encrypted"
version = "0.1.0"

[rtl]
top = "encrypted"
language = "verilog-2001"

[sources]
files = ["rtl/encrypted.v", "rtl/blackbox.dcp"]

[[clocks]]
name = "clk"
port = "clk"

[[resets]]
name = "rst_n"
port = "rst_n"
active = "low"

[[ports]]
name = "clk"
direction = "input"
width = 1

[[ports]]
name = "rst_n"
direction = "input"
width = 1

[[ports]]
name = "done"
direction = "output"
width = 1
"#,
    )
    .unwrap();
    std::fs::write(
        core_dir.join("rtl/encrypted.v"),
        "`default_nettype none\nmodule encrypted(input wire clk, input wire rst_n, output wire done); assign done = rst_n; endmodule\n`default_nettype wire\n",
    )
    .unwrap();
    std::fs::write(core_dir.join("rtl/blackbox.dcp"), "<binary>\n").unwrap();

    let build = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.arg("--build-root")
        .arg(build.path())
        .args(["core", "check"])
        .arg(&core_dir)
        .arg("--json")
        .assert()
        .failure()
        .stdout(contains("AF_PORTABLE_ENCRYPTED_NETLIST"));
}

#[test]
fn core_verify_community_tier_passes_on_reset_sync_example() {
    let root = repo_root();
    let build = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(&root)
        .arg("--build-root")
        .arg(build.path())
        .args([
            "core",
            "verify",
            "examples/af-reset-sync",
            "--tier",
            "community",
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"status\": \"passed\""))
        .stdout(contains("\"tier\": \"community\""))
        .stdout(contains("\"missing\": []"));
}

#[test]
fn core_verify_verified_package_tier_fails_with_missing_rows() {
    let root = repo_root();
    let build = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(&root)
        .arg("--build-root")
        .arg(build.path())
        .args([
            "core",
            "verify",
            "examples/af-reset-sync",
            "--tier",
            "verified-package",
            "--json",
        ])
        .assert()
        .failure()
        .stdout(contains("AF_TIER_REQUIREMENTS_UNMET"))
        .stdout(contains("\"tier\": \"verified-package\""))
        .stdout(contains("\"area\": \"docker_ci_cd_evidence\""));
}

#[test]
fn constructor_assemble_writes_fusesoc_core_for_two_cores() {
    let root = repo_root();
    let dir = tempdir().unwrap();
    let out = dir.path().join("iter5-demo");
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(&root)
        .args(["constructor", "assemble"])
        .arg("examples/af-reset-sync")
        .arg("examples/af-pdm-rx")
        .args(["--board", "tang-nano-20k"])
        .args(["--name", "iter5-demo"])
        .arg("--output")
        .arg(&out)
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("\"status\": \"passed\""))
        .stdout(contains("\"board\": \"tang-nano-20k\""))
        .stdout(contains("\"toolchain\": \"gowin_eda\""));

    assert!(out.join("iter5-demo.core").is_file());
    assert!(out.join("Makefile").is_file());
    assert!(out.join("README.md").is_file());
    let core_text = std::fs::read_to_string(out.join("iter5-demo.core")).unwrap();
    assert!(core_text.contains("CAPI=2:"));
    assert!(core_text.contains("af_reset_sync"));
    assert!(core_text.contains("af-pdm-rx") || core_text.contains("af_pdm_rx"));
}

#[test]
fn constructor_assemble_rejects_unknown_board() {
    let root = repo_root();
    let dir = tempdir().unwrap();
    let out = dir.path().join("bad");
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(&root)
        .args(["constructor", "assemble"])
        .arg("examples/af-reset-sync")
        .args(["--board", "no_such_board"])
        .args(["--name", "bad"])
        .arg("--output")
        .arg(&out)
        .arg("--json")
        .assert()
        .failure()
        .stdout(contains("AF_CONSTRUCTOR_ASSEMBLE_BOARD_UNKNOWN"));
}

#[test]
fn agent_kinds_lists_all_seven_templates() {
    let root = repo_root();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(&root)
        .args(["agent", "kinds", "--json"])
        .assert()
        .success()
        .stdout(contains("\"status\": \"passed\""))
        .stdout(contains("\"kind\": \"bug\""))
        .stdout(contains("\"kind\": \"feature\""))
        .stdout(contains("\"kind\": \"question\""))
        .stdout(contains("\"kind\": \"board-bringup\""))
        .stdout(contains("\"kind\": \"board-request\""))
        .stdout(contains("\"kind\": \"ip-request\""))
        .stdout(contains("\"kind\": \"agent-report\""))
        .stdout(contains("\"template_file\": \"bug_report.md\""))
        .stdout(contains("\"template_file\": \"agent_report.md\""));
}

#[test]
fn agent_context_includes_af_version_and_repro() {
    let root = repo_root();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(&root)
        .args(["agent", "context", "--json"])
        .assert()
        .success()
        .stdout(contains("\"af_version\""))
        .stdout(contains("\"reproducibility\""))
        .stdout(contains("\"environment_hash\""))
        .stdout(contains("\"repo_owner\""))
        .stdout(contains("\"repo_name\""));
}

#[test]
fn agent_issue_renders_markdown_with_required_sections() {
    let root = repo_root();
    let dir = tempdir().unwrap();
    let out = dir.path().join("body.md");
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(&root)
        .args(["agent", "issue"])
        .args(["--kind", "agent-report"])
        .args(["--title", "smoke title"])
        .args(["--summary", "one line summary"])
        .arg("--output")
        .arg(&out)
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("\"status\": \"passed\""))
        .stdout(contains("\"kind\": \"agent-report\""))
        .stdout(contains("\"gh_url\""))
        .stdout(contains("\"gh_cli\""));
    let body = std::fs::read_to_string(&out).unwrap();
    assert!(body.contains("## Summary"), "missing Summary heading");
    assert!(body.contains("one line summary"), "summary not rendered");
    assert!(
        body.contains("## Agent context"),
        "missing Agent context heading"
    );
    assert!(
        body.contains("`automated_submission`: `true`"),
        "missing automated_submission marker"
    );
    assert!(
        !body.contains("## Structured failure"),
        "Structured failure must be absent without --from-error"
    );
}

#[test]
fn agent_issue_emits_structured_failure_block_with_from_error() {
    let root = repo_root();
    let dir = tempdir().unwrap();
    let err_file = dir.path().join("err.json");
    std::fs::write(
        &err_file,
        r#"{"code":"AF_TEST_SMOKE","message":"x","hint":"y","exit_code":2}"#,
    )
    .unwrap();
    let out = dir.path().join("body.md");
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(&root)
        .args(["agent", "issue"])
        .args(["--kind", "bug"])
        .args(["--title", "with error"])
        .arg("--from-error")
        .arg(&err_file)
        .arg("--output")
        .arg(&out)
        .arg("--json")
        .assert()
        .success();
    let body = std::fs::read_to_string(&out).unwrap();
    assert!(body.contains("## Structured failure"));
    assert!(body.contains("AF_TEST_SMOKE"));
}

#[test]
fn agent_gh_url_percent_encodes_title_and_includes_labels() {
    let root = repo_root();
    let dir = tempdir().unwrap();
    let body = dir.path().join("body.md");
    std::fs::write(&body, "Hello world\n").unwrap();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(&root)
        .args(["agent", "gh-url"])
        .args(["--kind", "bug"])
        .args(["--title", "Hello, world"])
        .arg("--body-file")
        .arg(&body)
        .arg("--json")
        .assert()
        .success()
        // "Hello, world" → percent-encoded comma + space
        .stdout(contains("\"gh_url\""))
        .stdout(contains("title=Hello%2C%20world"))
        .stdout(contains("template=bug_report.md"))
        .stdout(contains("labels=bug%2Cagent-generated"));
}

#[test]
fn agent_gh_cli_emits_label_flags_from_kind_defaults() {
    let root = repo_root();
    let dir = tempdir().unwrap();
    let body = dir.path().join("body.md");
    std::fs::write(&body, "test\n").unwrap();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(&root)
        .args(["agent", "gh-cli"])
        .args(["--kind", "feature"])
        .args(["--title", "feat: smoke"])
        .arg("--body-file")
        .arg(&body)
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("gh issue create"))
        .stdout(contains("--label 'enhancement'"))
        .stdout(contains("--label 'agent-generated'"))
        .stdout(contains("'feat: smoke'"));
}

#[test]
fn agent_issue_rejects_unknown_kind() {
    let root = repo_root();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(&root)
        .args(["agent", "issue"])
        .args(["--kind", "nonsense"])
        .args(["--title", "t"])
        .arg("--json")
        .assert()
        .failure()
        .stdout(contains("AF_AGENT_KIND_UNSUPPORTED"));
}

#[test]
fn core_verify_rejects_unknown_tier() {
    let root = repo_root();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(&root)
        .args([
            "core",
            "verify",
            "examples/af-reset-sync",
            "--tier",
            "platinum",
            "--json",
        ])
        .assert()
        .failure()
        .stdout(contains("AF_TIER_UNKNOWN"));
}

#[test]
fn project_classify_emits_candidate_portability_levels() {
    let root = repo_root();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(&root)
        .args([
            "project",
            "classify",
            "--from-spec",
            "docs/product-requirements.md",
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"candidate_portability_levels\""));
}

#[test]
fn broken_fixtures_fail_without_panics() {
    let root = repo_root();
    let fixtures = [
        "missing-source",
        "missing-top",
        "invalid-port-width",
        "unknown-clock-domain",
        "path-traversal",
    ];

    for fixture in fixtures {
        let mut cmd = Command::cargo_bin("af").unwrap();
        cmd.args(["core", "check"])
            .arg(root.join("tests/fixtures/broken").join(fixture))
            .arg("--json")
            .assert()
            .failure()
            .stdout(contains("\"code\""))
            .stdout(contains("\"exit_code\""));
    }
}

fn scaffold_portable_core(core_dir: &Path, build: &Path, name: &str) {
    Command::cargo_bin("af")
        .unwrap()
        .arg("--build-root")
        .arg(build)
        .args(["core", "new"])
        .arg(core_dir)
        .args(["--name", name, "--language", "verilog", "--json"])
        .assert()
        .success();
}

fn write_workflow_placeholder(core_dir: &Path) {
    let workflow = core_dir.join(".github/workflows/hdl-ci.yml");
    std::fs::create_dir_all(workflow.parent().unwrap()).unwrap();
    std::fs::write(
        &workflow,
        "# Generated for test purposes\nname: hdl-ci\non: [push]\njobs: {}\n",
    )
    .unwrap();
}

fn ingest_ci_run_json(
    build: &Path,
    core: &str,
    payload: &serde_json::Value,
) -> assert_cmd::Command {
    let input_dir = tempdir().unwrap();
    let input = input_dir.path().join("ci-run.json");
    std::fs::write(&input, serde_json::to_string_pretty(payload).unwrap()).unwrap();

    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.arg("--build-root")
        .arg(build)
        .args(["evidence", "ingest"])
        .args(["--kind", "ci-run"])
        .arg("--input")
        .arg(&input)
        .args(["--core", core])
        .arg("--json")
        .assert()
        .success();

    // keep tempdir alive by leaking the input file path; the directory is dropped after the
    // call to `assert().success()` returns, but the ingested copy now lives under build-root.
    let mut keep = Command::cargo_bin("af").unwrap();
    keep.arg("--build-root").arg(build);
    keep
}

fn ci_evidence_row_status(stdout: &str) -> &str {
    // Locate the first occurrence of "docker_ci_cd_evidence" and the status that follows it.
    let area_idx = stdout
        .find("\"area\": \"docker_ci_cd_evidence\"")
        .expect("docker_ci_cd_evidence row must be present");
    let tail = &stdout[area_idx..];
    let status_idx = tail
        .find("\"status\":")
        .expect("docker_ci_cd_evidence row must have a status field");
    let tail = &tail[status_idx + "\"status\":".len()..];
    let first_quote = tail.find('"').unwrap();
    let rest = &tail[first_quote + 1..];
    let end_quote = rest.find('"').unwrap();
    &rest[..end_quote]
}

fn git_init_and_commit_head(repo_dir: &Path) -> String {
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .current_dir(repo_dir)
            .env("GIT_AUTHOR_NAME", "AF CI Test")
            .env("GIT_AUTHOR_EMAIL", "ci@example.com")
            .env("GIT_COMMITTER_NAME", "AF CI Test")
            .env("GIT_COMMITTER_EMAIL", "ci@example.com")
            .output()
            .expect("git invocation failed")
    };
    let out = git(&["init", "-q", "-b", "main"]);
    assert!(out.status.success(), "git init failed: {:?}", out);
    let out = git(&["add", "-A"]);
    assert!(out.status.success(), "git add failed: {:?}", out);
    let out = git(&[
        "-c",
        "commit.gpgsign=false",
        "commit",
        "-q",
        "-m",
        "fixture",
    ]);
    assert!(out.status.success(), "git commit failed: {:?}", out);
    let head = git(&["rev-parse", "HEAD"]);
    assert!(head.status.success(), "git rev-parse failed: {:?}", head);
    String::from_utf8(head.stdout).unwrap().trim().to_string()
}

#[test]
fn core_report_ci_gate_blocks_when_workflow_lacks_attributable_evidence() {
    let dir = tempdir().unwrap();
    let core_dir = dir.path().join("ci-gate-no-evidence");
    let build = tempdir().unwrap();
    scaffold_portable_core(&core_dir, build.path(), "ci-gate-no-evidence");
    write_workflow_placeholder(&core_dir);

    let output = Command::cargo_bin("af")
        .unwrap()
        .arg("--build-root")
        .arg(build.path())
        .args(["core", "report"])
        .arg(&core_dir)
        .arg("--json")
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert_eq!(ci_evidence_row_status(&stdout), "blocked");
    assert!(stdout.contains("Workflow file present without an attributable"));
}

#[test]
fn core_report_ci_gate_blocks_stale_evidence() {
    let dir = tempdir().unwrap();
    let core_dir = dir.path().join("ci-gate-stale");
    let build = tempdir().unwrap();
    scaffold_portable_core(&core_dir, build.path(), "ci-gate-stale");
    write_workflow_placeholder(&core_dir);
    git_init_and_commit_head(&core_dir);

    let stale_payload = serde_json::json!({
        "workflow_run_url": "https://github.com/example/repo/actions/runs/42",
        "commit_sha": "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
        "conclusion": "success",
        "artifact_bundle": ".af-build/ci/bundle.tar.gz",
        "sha256sums": ".af-build/ci/SHA256SUMS"
    });
    let _ = ingest_ci_run_json(build.path(), "ci-gate-stale", &stale_payload);

    let output = Command::cargo_bin("af")
        .unwrap()
        .arg("--build-root")
        .arg(build.path())
        .args(["core", "report"])
        .arg(&core_dir)
        .arg("--json")
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert_eq!(ci_evidence_row_status(&stdout), "blocked");
    assert!(stdout.contains("CI evidence is stale"));
}

#[test]
fn core_report_ci_gate_marks_supported_with_matching_evidence() {
    let dir = tempdir().unwrap();
    let core_dir = dir.path().join("ci-gate-supported");
    let build = tempdir().unwrap();
    scaffold_portable_core(&core_dir, build.path(), "ci-gate-supported");
    write_workflow_placeholder(&core_dir);
    let head_sha = git_init_and_commit_head(&core_dir);

    let payload = serde_json::json!({
        "workflow_run_url": "https://github.com/example/repo/actions/runs/100",
        "commit_sha": head_sha,
        "conclusion": "success",
        "artifact_bundle": ".af-build/ci/bundle.tar.gz",
        "sha256sums": ".af-build/ci/SHA256SUMS"
    });
    let _ = ingest_ci_run_json(build.path(), "ci-gate-supported", &payload);

    let output = Command::cargo_bin("af")
        .unwrap()
        .arg("--build-root")
        .arg(build.path())
        .args(["core", "report"])
        .arg(&core_dir)
        .arg("--json")
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert_eq!(ci_evidence_row_status(&stdout), "supported");
    assert!(stdout.contains("workflow_run_url=https://github.com/example/repo/actions/runs/100"));
    assert!(stdout.contains(&format!("commit_sha={head_sha}")));
    assert!(stdout.contains("conclusion=success"));
}

#[test]
fn core_report_ci_gate_blocks_failed_conclusion() {
    let dir = tempdir().unwrap();
    let core_dir = dir.path().join("ci-gate-failed");
    let build = tempdir().unwrap();
    scaffold_portable_core(&core_dir, build.path(), "ci-gate-failed");
    write_workflow_placeholder(&core_dir);
    let head_sha = git_init_and_commit_head(&core_dir);

    let payload = serde_json::json!({
        "workflow_run_url": "https://github.com/example/repo/actions/runs/7",
        "commit_sha": head_sha,
        "conclusion": "failure"
    });
    let _ = ingest_ci_run_json(build.path(), "ci-gate-failed", &payload);

    let output = Command::cargo_bin("af")
        .unwrap()
        .arg("--build-root")
        .arg(build.path())
        .args(["core", "report"])
        .arg(&core_dir)
        .arg("--json")
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert_eq!(ci_evidence_row_status(&stdout), "blocked");
    assert!(stdout.contains("not `success`"));
}

#[test]
fn evidence_ingest_ci_run_rejects_invalid_payload() {
    let build = tempdir().unwrap();
    let input_dir = tempdir().unwrap();
    let input = input_dir.path().join("bad.json");
    std::fs::write(
        &input,
        serde_json::to_string(&serde_json::json!({
            "commit_sha": "not-hex",
            "conclusion": "success"
        }))
        .unwrap(),
    )
    .unwrap();

    Command::cargo_bin("af")
        .unwrap()
        .arg("--build-root")
        .arg(build.path())
        .args(["evidence", "ingest"])
        .args(["--kind", "ci-run"])
        .arg("--input")
        .arg(&input)
        .arg("--json")
        .assert()
        .failure()
        .stdout(contains("AF_EVIDENCE_CI_RUN_INVALID"))
        .stdout(contains("\"exit_code\": 2"));

    let input2 = input_dir.path().join("bad-conclusion.json");
    std::fs::write(
        &input2,
        serde_json::to_string(&serde_json::json!({
            "commit_sha": "abcdef1234567890",
            "conclusion": "weird-value"
        }))
        .unwrap(),
    )
    .unwrap();

    Command::cargo_bin("af")
        .unwrap()
        .arg("--build-root")
        .arg(build.path())
        .args(["evidence", "ingest"])
        .args(["--kind", "ci-run"])
        .arg("--input")
        .arg(&input2)
        .arg("--json")
        .assert()
        .failure()
        .stdout(contains("AF_EVIDENCE_CI_RUN_INVALID"))
        .stdout(contains("\"exit_code\": 2"));
}

#[test]
fn evidence_ingest_ci_run_writes_ci_run_block() {
    let build = tempdir().unwrap();
    let input_dir = tempdir().unwrap();
    let input = input_dir.path().join("ok.json");
    std::fs::write(
        &input,
        serde_json::to_string(&serde_json::json!({
            "workflow_run_url": "https://github.com/x/y/actions/runs/1",
            "commit_sha": "abcdef0123456789",
            "conclusion": "success",
            "artifact_bundle": ".af-build/ci/bundle.tar.gz",
            "sha256sums": ".af-build/ci/SHA256SUMS"
        }))
        .unwrap(),
    )
    .unwrap();

    Command::cargo_bin("af")
        .unwrap()
        .arg("--build-root")
        .arg(build.path())
        .args(["evidence", "ingest"])
        .args(["--kind", "ci-run"])
        .arg("--input")
        .arg(&input)
        .args(["--core", "ci-demo"])
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("\"evidence_kind\": \"ci-run\""))
        .stdout(contains("\"evidence_status\": \"passed\""))
        .stdout(contains("\"ci_run\""))
        .stdout(contains("\"conclusion\": \"success\""));

    let report_path = build
        .path()
        .join("reports/evidence/ci_run_report-ok_json.json");
    assert!(report_path.is_file(), "expected report at {report_path:?}");
    let text = std::fs::read_to_string(&report_path).unwrap();
    assert!(text.contains("\"workflow_run_url\""));
    assert!(text.contains("\"artifact_bundle\""));
}

// --- P2: broken-input panic-freedom and structured-error contract -------------
//
// NFR-007 ("CLI must not panic on broken fixtures") is enforced for the full
// surface of subcommands that read external files/dirs. Read-only commands that
// do not take broken input (doctor, tooling check/plan/ensure, registry check,
// board list, board matrix, backend list, vectors generate, self check, ci
// doctor, ci run-local) are exercised by their own tests above.

struct BrokenCase {
    label: &'static str,
    args: &'static [&'static str],
    fixture_args: &'static [&'static str],
    /// If Some, exit code must equal this value (FR-011 stable exit codes).
    /// If None, the test only asserts no-panic on stderr and well-formed JSON.
    expect_exit: Option<i32>,
}

const BROKEN_CASES: &[BrokenCase] = &[
    BrokenCase {
        label: "manifest validate (invalid toml)",
        args: &["manifest", "validate"],
        fixture_args: &["tests/fixtures/broken/manifest-invalid-toml/af-core.toml"],
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "manifest validate (unsupported version)",
        args: &["manifest", "validate"],
        fixture_args: &["tests/fixtures/broken/manifest-unsupported-version/af-core.toml"],
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "manifest migrate (invalid toml)",
        args: &["manifest", "migrate", "--from", "0.1", "--to", "0.3"],
        fixture_args: &["tests/fixtures/broken/manifest-invalid-toml/af-core.toml"],
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "architecture check (missing-source)",
        args: &["architecture", "check"],
        fixture_args: &["tests/fixtures/broken/missing-source"],
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "resource plan (missing-source)",
        args: &[
            "resource",
            "plan",
            "--vendor",
            "xilinx",
            "--family",
            "ultrascale-plus",
        ],
        fixture_args: &["tests/fixtures/broken/missing-source"],
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "compatibility check (two broken cores)",
        args: &["compatibility", "check"],
        fixture_args: &[
            "tests/fixtures/broken/missing-source",
            "tests/fixtures/broken/missing-top",
        ],
        // fail-closed: AF_CORE_CHECK_FAILED (exit 2) on the first invalid manifest.
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "signoff plan (missing-source)",
        args: &["signoff", "plan", "--class", "simple-portable"],
        fixture_args: &["tests/fixtures/broken/missing-source"],
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "dependency graph (missing-source)",
        args: &["dependency", "graph", "--format", "json"],
        fixture_args: &["tests/fixtures/broken/missing-source"],
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "wrapper generate fusesoc",
        args: &["wrapper", "generate", "--target", "fusesoc"],
        fixture_args: &["tests/fixtures/broken/missing-source"],
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "wrapper generate litex",
        args: &[
            "wrapper",
            "generate",
            "--target",
            "litex",
            "--board",
            "tang-nano-20k",
        ],
        fixture_args: &["tests/fixtures/broken/missing-source"],
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "wrapper generate ipxact",
        args: &["wrapper", "generate", "--target", "ipxact"],
        fixture_args: &["tests/fixtures/broken/missing-source"],
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "core lint native",
        args: &["core", "lint", "--backend", "native"],
        fixture_args: &["tests/fixtures/broken/missing-source"],
        // exit 7 (lint failed) per docs/cli-reference.md table.
        expect_exit: Some(7),
    },
    BrokenCase {
        label: "core lint verilator",
        args: &["core", "lint", "--backend", "verilator"],
        fixture_args: &["tests/fixtures/broken/missing-source"],
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "core lint yosys",
        args: &["core", "lint", "--backend", "yosys"],
        fixture_args: &["tests/fixtures/broken/missing-source"],
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "core lint icarus",
        args: &["core", "lint", "--backend", "icarus"],
        fixture_args: &["tests/fixtures/broken/missing-source"],
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "core sim verilator",
        args: &["core", "sim", "--backend", "verilator"],
        fixture_args: &["tests/fixtures/broken/missing-source"],
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "core sim icarus",
        args: &["core", "sim", "--backend", "icarus"],
        fixture_args: &["tests/fixtures/broken/missing-source"],
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "core formal sby",
        args: &["core", "formal", "--backend", "sby"],
        fixture_args: &["tests/fixtures/broken/missing-source"],
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "core tooling (missing-source)",
        args: &["core", "tooling"],
        fixture_args: &["tests/fixtures/broken/missing-source"],
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "core package (missing-source)",
        args: &["core", "package", "--format", "manifest"],
        fixture_args: &["tests/fixtures/broken/missing-source"],
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "core report (missing-source)",
        args: &["core", "report"],
        fixture_args: &["tests/fixtures/broken/missing-source"],
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "board check (broken)",
        args: &["board", "check"],
        fixture_args: &["tests/fixtures/broken/board-broken/af-board.toml"],
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "ci validate (invalid config)",
        args: &[
            "ci",
            "validate",
            "--repo",
            "tests/fixtures/broken/ci-config-invalid",
        ],
        fixture_args: &[],
        // ci validate exits 3 (inspection/orchestration) on a malformed config tree.
        expect_exit: Some(3),
    },
    BrokenCase {
        label: "evidence ingest (missing input)",
        args: &[
            "evidence",
            "ingest",
            "--kind",
            "simulation-log",
            "--tool",
            "fake",
            "--input",
            "tests/fixtures/broken/does-not-exist.log",
        ],
        fixture_args: &[],
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "evidence ingest ci-run (missing input)",
        args: &[
            "evidence",
            "ingest",
            "--kind",
            "ci-run",
            "--input",
            "tests/fixtures/broken/does-not-exist.json",
        ],
        fixture_args: &[],
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "project classify (empty spec)",
        args: &[
            "project",
            "classify",
            "--from-spec",
            "tests/fixtures/broken/spec-empty.md",
        ],
        fixture_args: &[],
        // fail-closed: empty spec must not produce a confident verdict.
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "constructor export (missing-source)",
        args: &["constructor", "export"],
        fixture_args: &["tests/fixtures/broken/missing-source"],
        // fail-closed: invalid manifest must not produce placeholder bundle.
        expect_exit: Some(2),
    },
    BrokenCase {
        label: "backend scaffold (missing-source)",
        args: &[
            "backend",
            "scaffold",
            "--vendor",
            "xilinx",
            "--family",
            "ultrascale-plus",
        ],
        fixture_args: &["tests/fixtures/broken/missing-source"],
        // fail-closed: refuse to write vendor scaffolding into a broken tree.
        expect_exit: Some(2),
    },
];

fn assert_no_panic_in_stderr(stderr: &str, label: &str) {
    for needle in &[
        "panicked at",
        "note: run with `RUST_BACKTRACE",
        "thread 'main' panicked",
        "internal error: entered unreachable code",
    ] {
        assert!(
            !stderr.contains(needle),
            "{label}: stderr contains panic marker {needle:?}; stderr was:\n{stderr}"
        );
    }
}

fn assert_structured_failure(stdout: &str, label: &str) {
    for needle in &["\"code\":", "\"message\":", "\"hint\":", "\"exit_code\":"] {
        assert!(
            stdout.contains(needle),
            "{label}: --json stdout missing required error field {needle:?}; stdout was:\n{stdout}"
        );
    }
}

#[test]
fn broken_inputs_never_panic_across_subcommands() {
    let root = repo_root();
    for case in BROKEN_CASES {
        let build = tempdir().unwrap();
        let mut cmd = Command::cargo_bin("af").unwrap();
        cmd.current_dir(&root)
            .arg("--build-root")
            .arg(build.path())
            .args(case.args)
            .args(case.fixture_args)
            .arg("--json");
        let assert = cmd.assert();
        let output = assert.get_output();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert_no_panic_in_stderr(&stderr, case.label);

        if let Some(expected_exit) = case.expect_exit {
            let actual = output
                .status
                .code()
                .unwrap_or_else(|| panic!("{}: process terminated by signal", case.label));
            assert_eq!(
                actual, expected_exit,
                "{}: expected exit={expected_exit}, got {actual}; stdout=\n{stdout}\nstderr=\n{stderr}",
                case.label
            );
            assert_structured_failure(&stdout, case.label);
        } else {
            // graceful path: the command must at least emit a JSON object on stdout
            // so an LLM operator gets a machine-readable response, not a silent failure.
            let trimmed = stdout.trim_start();
            assert!(
                trimmed.starts_with('{') || trimmed.starts_with('['),
                "{}: stdout is not JSON; stdout was:\n{}",
                case.label,
                stdout
            );
        }
    }
}

#[test]
fn broken_inputs_emit_stable_exit_codes() {
    let root = repo_root();
    // Subset that we want to lock in as part of the FR-011 stable exit-code contract.
    // The set intentionally spans different error classes: parse (2), inspection (3),
    // backend-unavailable / lint-failed (4/7).
    let pinned: &[(&str, &[&str], &[&str], i32)] = &[
        (
            "manifest_validate_parse_error",
            &["manifest", "validate"],
            &["tests/fixtures/broken/manifest-invalid-toml/af-core.toml"],
            2,
        ),
        (
            "manifest_validate_schema_error",
            &["manifest", "validate"],
            &["tests/fixtures/broken/manifest-unsupported-version/af-core.toml"],
            2,
        ),
        (
            "core_check_missing_source_inspection",
            &["core", "check"],
            &["tests/fixtures/broken/missing-source"],
            2,
        ),
        (
            "core_check_path_traversal_security",
            &["core", "check"],
            &["tests/fixtures/broken/path-traversal"],
            // path-traversal is rejected during manifest validation (exit 2),
            // not at backend execution time.
            2,
        ),
        (
            "core_lint_native_lint_failed",
            &["core", "lint", "--backend", "native"],
            &["tests/fixtures/broken/missing-source"],
            7,
        ),
        (
            "board_check_invalid_manifest",
            &["board", "check"],
            &["tests/fixtures/broken/board-broken/af-board.toml"],
            2,
        ),
        (
            "compatibility_check_rejects_broken_cores",
            &["compatibility", "check"],
            &[
                "tests/fixtures/broken/missing-source",
                "tests/fixtures/broken/missing-top",
            ],
            2,
        ),
        (
            "signoff_plan_rejects_broken_core",
            &["signoff", "plan", "--class", "simple-portable"],
            &["tests/fixtures/broken/missing-source"],
            2,
        ),
        (
            "dependency_graph_rejects_broken_core",
            &["dependency", "graph", "--format", "json"],
            &["tests/fixtures/broken/missing-source"],
            2,
        ),
        (
            "project_classify_rejects_empty_spec",
            &[
                "project",
                "classify",
                "--from-spec",
                "tests/fixtures/broken/spec-empty.md",
            ],
            &[],
            2,
        ),
        (
            "constructor_export_rejects_broken_core",
            &["constructor", "export"],
            &["tests/fixtures/broken/missing-source"],
            2,
        ),
        (
            "backend_scaffold_rejects_broken_core",
            &[
                "backend",
                "scaffold",
                "--vendor",
                "xilinx",
                "--family",
                "ultrascale-plus",
            ],
            &["tests/fixtures/broken/missing-source"],
            2,
        ),
    ];

    for (label, args, fixture_args, expected) in pinned {
        let build = tempdir().unwrap();
        let mut cmd = Command::cargo_bin("af").unwrap();
        cmd.current_dir(&root)
            .arg("--build-root")
            .arg(build.path())
            .args(*args)
            .args(*fixture_args)
            .arg("--json");
        let output = cmd.assert().get_output().clone();
        let actual = output
            .status
            .code()
            .unwrap_or_else(|| panic!("{label}: process terminated by signal"));
        assert_eq!(
            actual,
            *expected,
            "{label}: expected stable exit={expected}, got {actual}; stdout=\n{}\nstderr=\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
}

#[test]
fn project_new_refuses_to_overwrite_existing_manifest_dir() {
    // A directory that already contains an AccelFury manifest must not be
    // silently re-scaffolded; that is a typo-class footgun for LLM operators.
    let dir = tempdir().unwrap();
    let build = tempdir().unwrap();
    std::fs::write(dir.path().join("af-project.toml"), "# existing\n").unwrap();
    Command::cargo_bin("af")
        .unwrap()
        .arg("--build-root")
        .arg(build.path())
        .args(["project", "new"])
        .arg(dir.path())
        .args(["--class", "system-platform", "--name", "pn", "--json"])
        .assert()
        .failure()
        .stdout(contains("\"code\": \"AF_PROJECT_NEW_DIR_NOT_EMPTY\""))
        .stdout(contains("\"exit_code\": 2"));
}

#[test]
fn project_new_accepts_fresh_directory() {
    let dir = tempdir().unwrap();
    let build = tempdir().unwrap();
    let target = dir.path().join("brand-new-project");
    Command::cargo_bin("af")
        .unwrap()
        .arg("--build-root")
        .arg(build.path())
        .args(["project", "new"])
        .arg(&target)
        .args(["--class", "system-platform", "--name", "pn", "--json"])
        .assert()
        .success();
    assert!(target.join("af-project.toml").is_file());
}

// --- M0 closure: "Generated by AccelFury IP Toolchain" stamp coverage --------

fn assert_every_generated_file_has_stamp(dir: &Path, label: &str) {
    use std::ffi::OsStr;
    fn walk(p: &Path, acc: &mut Vec<PathBuf>) {
        if p.is_file() {
            acc.push(p.to_path_buf());
            return;
        }
        if !p.is_dir() {
            return;
        }
        let Ok(entries) = std::fs::read_dir(p) else {
            return;
        };
        for entry in entries.flatten() {
            walk(&entry.path(), acc);
        }
    }
    let mut files = Vec::new();
    walk(dir, &mut files);
    assert!(
        !files.is_empty(),
        "{label}: expected at least one generated file under {}",
        dir.display()
    );
    for file in files {
        // Skip directory placeholders and binary outputs (none are produced
        // by `core new`, but be defensive for future scaffolds).
        if file.extension() == Some(OsStr::new("bin"))
            || file.extension() == Some(OsStr::new("png"))
        {
            continue;
        }
        let text = std::fs::read_to_string(&file)
            .unwrap_or_else(|err| panic!("{label}: failed to read {}: {err}", file.display()));
        assert!(
            text.contains("Generated by AccelFury IP Toolchain"),
            "{label}: file {} is missing the AccelFury generated stamp",
            file.display()
        );
    }
}

#[test]
fn core_new_simple_portable_stamps_every_generated_file() {
    let dir = tempdir().unwrap();
    let core_dir = dir.path().join("aud_simple");
    let build = tempdir().unwrap();
    Command::cargo_bin("af")
        .unwrap()
        .arg("--build-root")
        .arg(build.path())
        .args(["core", "new"])
        .arg(&core_dir)
        .args(["--name", "aud_simple", "--language", "verilog", "--json"])
        .assert()
        .success();
    assert_every_generated_file_has_stamp(&core_dir, "simple-portable");
}

#[test]
fn core_new_reset_sync_profile_stamps_every_generated_file() {
    let dir = tempdir().unwrap();
    let core_dir = dir.path().join("aud_reset");
    let build = tempdir().unwrap();
    Command::cargo_bin("af")
        .unwrap()
        .arg("--build-root")
        .arg(build.path())
        .args(["core", "new"])
        .arg(&core_dir)
        .args([
            "--name",
            "aud_reset",
            "--class",
            "simple-portable",
            "--profile",
            "reset-sync",
            "--language",
            "verilog",
            "--json",
        ])
        .assert()
        .success();
    assert_every_generated_file_has_stamp(&core_dir, "reset-sync");
}

#[test]
fn core_new_composite_portable_stamps_every_generated_file() {
    let dir = tempdir().unwrap();
    let core_dir = dir.path().join("aud_composite");
    let build = tempdir().unwrap();
    Command::cargo_bin("af")
        .unwrap()
        .arg("--build-root")
        .arg(build.path())
        .args(["core", "new"])
        .arg(&core_dir)
        .args([
            "--name",
            "aud_composite",
            "--class",
            "composite-portable",
            "--language",
            "verilog",
            "--json",
        ])
        .assert()
        .success();
    assert_every_generated_file_has_stamp(&core_dir, "composite-portable");
}

#[test]
fn core_new_complex_vendor_aware_stamps_every_generated_file() {
    let dir = tempdir().unwrap();
    let core_dir = dir.path().join("aud_complex");
    let build = tempdir().unwrap();
    Command::cargo_bin("af")
        .unwrap()
        .arg("--build-root")
        .arg(build.path())
        .args(["core", "new"])
        .arg(&core_dir)
        .args([
            "--name",
            "aud_complex",
            "--class",
            "complex-vendor-aware",
            "--language",
            "verilog",
            "--json",
        ])
        .assert()
        .success();
    assert_every_generated_file_has_stamp(&core_dir, "complex-vendor-aware");
}

#[test]
fn project_new_system_platform_stamps_every_generated_file() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("sys-proj");
    let build = tempdir().unwrap();
    Command::cargo_bin("af")
        .unwrap()
        .arg("--build-root")
        .arg(build.path())
        .args(["project", "new"])
        .arg(&target)
        .args(["--class", "system-platform", "--name", "sys-proj", "--json"])
        .assert()
        .success();
    assert_every_generated_file_has_stamp(&target, "project new system-platform");
}

#[test]
fn project_new_product_stack_stamps_every_generated_file() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("prod-stack");
    let build = tempdir().unwrap();
    Command::cargo_bin("af")
        .unwrap()
        .arg("--build-root")
        .arg(build.path())
        .args(["project", "new"])
        .arg(&target)
        .args(["--class", "product-stack", "--name", "prod-stack", "--json"])
        .assert()
        .success();
    assert_every_generated_file_has_stamp(&target, "project new product-stack");
}

#[test]
fn wrapper_generate_ipxact_xml_carries_stamp() {
    let root = repo_root();
    let build = tempdir().unwrap();
    Command::cargo_bin("af")
        .unwrap()
        .current_dir(&root)
        .arg("--build-root")
        .arg(build.path())
        .args([
            "wrapper",
            "generate",
            "examples/af-pdm-rx",
            "--target",
            "ipxact",
            "--json",
        ])
        .assert()
        .success();
    assert_every_generated_file_has_stamp(&build.path().join("ipxact"), "wrapper ipxact");
}

#[test]
fn constructor_export_every_json_carries_stamp() {
    let root = repo_root();
    let build = tempdir().unwrap();
    let out = build.path().join("constructor");
    Command::cargo_bin("af")
        .unwrap()
        .current_dir(&root)
        .arg("--build-root")
        .arg(build.path())
        .args(["constructor", "export", "examples/af-pdm-rx", "--output"])
        .arg(&out)
        .arg("--json")
        .assert()
        .success();
    assert_every_generated_file_has_stamp(&out, "constructor export");
}

#[test]
fn backend_scaffold_every_vendor_file_carries_stamp() {
    let root = repo_root();
    let dir = tempdir().unwrap();
    let core_dir = dir.path().join("af-mod-add");
    // Copy a known-good example so manifest validation passes.
    copy_directory(&root.join("examples/af-mod-add"), &core_dir);
    let build = tempdir().unwrap();
    Command::cargo_bin("af")
        .unwrap()
        .current_dir(&root)
        .arg("--build-root")
        .arg(build.path())
        .args(["backend", "scaffold"])
        .arg(&core_dir)
        .args([
            "--vendor",
            "xilinx",
            "--family",
            "ultrascale-plus",
            "--json",
        ])
        .assert()
        .success();
    assert_every_generated_file_has_stamp(&core_dir.join("vendor/xilinx"), "backend scaffold");
}

fn copy_directory(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap().flatten() {
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_directory(&path, &target);
        } else {
            std::fs::copy(&path, &target).unwrap();
        }
    }
}

#[test]
fn ci_init_every_generated_file_carries_stamp() {
    let dir = tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("rtl")).unwrap();
    std::fs::write(
        dir.path().join("rtl").join("demo.v"),
        "module demo; endmodule\n",
    )
    .unwrap();
    let build = tempdir().unwrap();
    Command::cargo_bin("af")
        .unwrap()
        .arg("--build-root")
        .arg(build.path())
        .args(["ci", "init", "--repo"])
        .arg(dir.path())
        .args([
            "--project",
            "demo",
            "--hdl",
            "verilog-2001",
            "--rtl",
            "rtl",
            "--top",
            "demo",
            "--provider",
            "github",
            "--json",
        ])
        .assert()
        .success();
    // The user's rtl/demo.v is handwritten; only assert on AF-generated files.
    for rel in [
        ".github/workflows/hdl-ci.yml",
        ".github/PULL_REQUEST_TEMPLATE.md",
        "docs/ci.md",
        "af-ci.toml",
        "scripts/ci/prepare_paths.sh",
        "artifacts/openfpga-ci/reports/af-ci-init-report.json",
        "artifacts/openfpga-ci/reports/af-ci-init-report.txt",
    ] {
        let path = dir.path().join(rel);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("ci init: failed to read {}: {err}", path.display()));
        assert!(
            text.contains("Generated by AccelFury IP Toolchain"),
            "ci init: {rel} is missing the AccelFury generated stamp"
        );
    }
}

// --- M3 typed report contracts: reproducibility metadata --------------------

#[test]
fn core_report_includes_reproducibility_metadata() {
    let root = repo_root();
    let build = tempdir().unwrap();
    let output = Command::cargo_bin("af")
        .unwrap()
        .current_dir(&root)
        .arg("--build-root")
        .arg(build.path())
        .args(["core", "report", "examples/af-pdm-rx", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("\"reproducibility\""),
        "core report --json must include the reproducibility block; stdout:\n{stdout}"
    );
    assert!(stdout.contains("\"host_os\""));
    assert!(stdout.contains("\"host_arch\""));
    assert!(stdout.contains("\"environment_hash\""));
    assert!(stdout.contains("\"af_version\""));

    // The on-disk artefact must also carry the block (single source of truth).
    let on_disk = std::fs::read_to_string(build.path().join("reports/core-report.json")).unwrap();
    assert!(on_disk.contains("\"reproducibility\""));
}

fn extract_environment_hash_from(report_path: &Path) -> String {
    let text = std::fs::read_to_string(report_path).unwrap();
    let idx = text
        .find("\"environment_hash\"")
        .expect("environment_hash must be present");
    let tail = &text[idx + "\"environment_hash\"".len()..];
    let q1 = tail.find('"').unwrap();
    let rest = &tail[q1 + 1..];
    let q2 = rest.find('"').unwrap();
    rest[..q2].to_string()
}

#[test]
fn core_check_reproducibility_is_deterministic_between_runs() {
    let root = repo_root();
    let build_a = tempdir().unwrap();
    let build_b = tempdir().unwrap();
    for build in [&build_a, &build_b] {
        Command::cargo_bin("af")
            .unwrap()
            .current_dir(&root)
            .arg("--build-root")
            .arg(build.path())
            .args(["core", "check", "examples/af-pdm-rx", "--json"])
            .assert()
            .success();
    }
    assert_eq!(
        extract_environment_hash_from(&build_a.path().join("reports/core-check.json")),
        extract_environment_hash_from(&build_b.path().join("reports/core-check.json")),
        "environment_hash must be deterministic across runs with the same toolchain"
    );
}

#[test]
fn core_lint_native_carries_typed_lint_payload() {
    let root = repo_root();
    let build = tempdir().unwrap();
    let output = Command::cargo_bin("af")
        .unwrap()
        .current_dir(&root)
        .arg("--build-root")
        .arg(build.path())
        .args([
            "core",
            "lint",
            "examples/af-pdm-rx",
            "--backend",
            "native",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("\"command_payload\""),
        "core lint --json must carry the typed payload; stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("\"kind\": \"lint\""),
        "command_payload.kind must be `lint`; stdout:\n{stdout}"
    );
    assert!(stdout.contains("\"backend\": \"native\""));
}

#[test]
fn core_package_carries_typed_package_payload() {
    let root = repo_root();
    let build = tempdir().unwrap();
    let output = Command::cargo_bin("af")
        .unwrap()
        .current_dir(&root)
        .arg("--build-root")
        .arg(build.path())
        .args([
            "core",
            "package",
            "examples/af-pdm-rx",
            "--format",
            "manifest",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("\"kind\": \"package\""));
    assert!(stdout.contains("\"format\": \"manifest\""));
}

#[test]
fn core_check_carries_typed_check_payload() {
    let root = repo_root();
    let build = tempdir().unwrap();
    let output = Command::cargo_bin("af")
        .unwrap()
        .current_dir(&root)
        .arg("--build-root")
        .arg(build.path())
        .args(["core", "check", "examples/af-pdm-rx", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("\"kind\": \"check\""));
    assert!(stdout.contains("\"manifest_status\""));
    assert!(stdout.contains("\"inspection_issue_count\""));
}

#[test]
fn core_report_carries_typed_report_payload() {
    let root = repo_root();
    let build = tempdir().unwrap();
    let output = Command::cargo_bin("af")
        .unwrap()
        .current_dir(&root)
        .arg("--build-root")
        .arg(build.path())
        .args(["core", "report", "examples/af-pdm-rx", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("\"kind\": \"report\""));
    assert!(stdout.contains("\"input_kind\": \"core_with_manifest\""));
    assert!(stdout.contains("\"maturity_verdict\""));
}

#[test]
fn doctor_carries_typed_doctor_payload() {
    let build = tempdir().unwrap();
    let output = Command::cargo_bin("af")
        .unwrap()
        .arg("--build-root")
        .arg(build.path())
        .args(["doctor", "--json"])
        .assert();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("\"kind\": \"doctor\""));
    assert!(stdout.contains("\"total_tools\""));
    assert!(stdout.contains("\"missing_tools\""));
}

#[test]
fn core_tooling_carries_typed_tooling_payload() {
    let root = repo_root();
    let build = tempdir().unwrap();
    let output = Command::cargo_bin("af")
        .unwrap()
        .current_dir(&root)
        .arg("--build-root")
        .arg(build.path())
        .args(["core", "tooling", "examples/af-pdm-rx", "--json"])
        .assert();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("\"kind\": \"tooling\""));
    assert!(stdout.contains("\"total_tools\""));
    assert!(stdout.contains("\"available_tools\""));
}

// --- NFR-004 enforcement: byte-determinism for `af core new` -----------------

fn read_files_sha256(dir: &Path) -> std::collections::BTreeMap<String, String> {
    use std::collections::BTreeMap;
    fn walk(p: &Path, base: &Path, acc: &mut BTreeMap<String, String>) {
        if p.is_file() {
            let rel = p.strip_prefix(base).unwrap().display().to_string();
            let bytes = std::fs::read(p).unwrap();
            let mut h: u128 = 0xcbf29ce484222325 ^ ((bytes.len() as u128) << 64);
            for byte in &bytes {
                h ^= *byte as u128;
                h = h.wrapping_mul(0x100000001b3);
            }
            acc.insert(rel, format!("{h:032x}"));
            return;
        }
        if !p.is_dir() {
            return;
        }
        let Ok(entries) = std::fs::read_dir(p) else {
            return;
        };
        for entry in entries.flatten() {
            walk(&entry.path(), base, acc);
        }
    }
    let mut acc = BTreeMap::new();
    walk(dir, dir, &mut acc);
    acc
}

fn run_core_new(out_dir: &Path, name: &str, class: &str, profile: Option<&str>) {
    let build = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.arg("--build-root")
        .arg(build.path())
        .args(["core", "new"])
        .arg(out_dir)
        .args(["--name", name, "--class", class, "--language", "verilog"]);
    if let Some(profile) = profile {
        cmd.args(["--profile", profile]);
    }
    cmd.arg("--json").assert().success();
}

#[test]
fn core_new_output_is_byte_deterministic_simple_portable() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    run_core_new(&a, "aud_det", "simple-portable", None);
    run_core_new(&b, "aud_det", "simple-portable", None);
    assert_eq!(
        read_files_sha256(&a),
        read_files_sha256(&b),
        "core new simple-portable produced non-deterministic output between runs"
    );
}

#[test]
fn core_new_output_is_byte_deterministic_reset_sync() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    run_core_new(&a, "aud_det", "simple-portable", Some("reset-sync"));
    run_core_new(&b, "aud_det", "simple-portable", Some("reset-sync"));
    assert_eq!(
        read_files_sha256(&a),
        read_files_sha256(&b),
        "core new reset-sync produced non-deterministic output between runs"
    );
}

#[test]
fn core_new_output_is_byte_deterministic_composite_portable() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    run_core_new(&a, "aud_det", "composite-portable", None);
    run_core_new(&b, "aud_det", "composite-portable", None);
    assert_eq!(
        read_files_sha256(&a),
        read_files_sha256(&b),
        "core new composite-portable produced non-deterministic output between runs"
    );
}

#[test]
fn core_new_output_is_byte_deterministic_complex_vendor_aware() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    run_core_new(&a, "aud_det", "complex-vendor-aware", None);
    run_core_new(&b, "aud_det", "complex-vendor-aware", None);
    assert_eq!(
        read_files_sha256(&a),
        read_files_sha256(&b),
        "core new complex-vendor-aware produced non-deterministic output between runs"
    );
}

#[test]
fn wrapper_generate_is_byte_deterministic_fusesoc() {
    let root = repo_root();
    let build_a = tempdir().unwrap();
    let build_b = tempdir().unwrap();
    for build in [&build_a, &build_b] {
        Command::cargo_bin("af")
            .unwrap()
            .current_dir(&root)
            .arg("--build-root")
            .arg(build.path())
            .args([
                "wrapper",
                "generate",
                "examples/af-pdm-rx",
                "--target",
                "fusesoc",
                "--json",
            ])
            .assert()
            .success();
    }
    let a = build_a.path().join("fusesoc");
    let b = build_b.path().join("fusesoc");
    assert_eq!(
        read_files_sha256(&a),
        read_files_sha256(&b),
        "wrapper generate fusesoc produced non-deterministic output between runs"
    );
}

// --- P2: honest board metadata surface ---------------------------------------

#[test]
fn board_list_human_marks_draft_boards() {
    let root = repo_root();
    let output = Command::cargo_bin("af")
        .unwrap()
        .current_dir(&root)
        .args(["board", "list"])
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    // Every line must start with either [VERIFIED] or [DRAFT]; the registry
    // currently has only draft_placeholder entries, so we expect [DRAFT] present.
    for line in stdout.lines().filter(|l| !l.trim().is_empty()) {
        assert!(
            line.starts_with("[VERIFIED] ") || line.starts_with("[DRAFT] "),
            "board list line is missing status marker: {line}"
        );
    }
    assert!(
        stdout.contains("[DRAFT] sipeed_tang_nano_1k"),
        "expected at least one [DRAFT] entry for sipeed_tang_nano_1k; stdout:\n{stdout}"
    );
}

#[test]
fn wrapper_generate_warns_on_draft_board() {
    let root = repo_root();
    let build = tempdir().unwrap();
    let output = Command::cargo_bin("af")
        .unwrap()
        .current_dir(&root)
        .arg("--build-root")
        .arg(build.path())
        .args([
            "wrapper",
            "generate",
            "examples/af-pdm-rx",
            "--target",
            "litex",
            "--board",
            "sipeed_tang_nano_1k",
        ])
        .arg("--json")
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("sipeed_tang_nano_1k") && stdout.contains("placeholder"),
        "expected placeholder warning for draft board; stdout:\n{stdout}"
    );
}

#[test]
fn wrapper_generate_warns_on_unknown_board_in_registry() {
    let root = repo_root();
    let build = tempdir().unwrap();
    let output = Command::cargo_bin("af")
        .unwrap()
        .current_dir(&root)
        .arg("--build-root")
        .arg(build.path())
        .args([
            "wrapper",
            "generate",
            "examples/af-pdm-rx",
            "--target",
            "litex",
            "--board",
            "completely-fictional-board-xyz",
        ])
        .arg("--json")
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("completely-fictional-board-xyz")
            && stdout.contains("not found in registries"),
        "expected unknown-board warning; stdout:\n{stdout}"
    );
}

#[test]
fn core_report_lists_placeholder_boards_in_maturity_row() {
    let root = repo_root();
    let build = tempdir().unwrap();
    let output = Command::cargo_bin("af")
        .unwrap()
        .current_dir(&root)
        .arg("--build-root")
        .arg(build.path())
        .args(["core", "report"])
        .arg(root.join("examples/af-mod-add"))
        .arg("--json")
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    // examples/af-mod-add declares 30 boards in its manifest, all draft_placeholder.
    let area_idx = stdout
        .find("\"area\": \"board_hardware_evidence\"")
        .expect("board_hardware_evidence row must be present");
    let tail = &stdout[area_idx..];
    let row_end = tail.find("\n        }").unwrap_or(tail.len());
    let row = &tail[..row_end];
    assert!(
        row.contains("(draft)"),
        "expected at least one (draft) evidence entry in board_hardware_evidence row:\n{row}"
    );
    assert!(
        row.contains("draft_placeholder") || row.contains("non-verified"),
        "expected placeholder-status limitation in board_hardware_evidence row:\n{row}"
    );
    assert!(
        row.contains("sipeed_tang_nano_1k"),
        "expected at least one specific draft board id in the row:\n{row}"
    );
}
