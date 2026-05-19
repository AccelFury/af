// SPDX-License-Identifier: Apache-2.0
//
// `plan_nextpnr(board, build_root)` returns a `NextpnrPlan` with:
//   * `family` resolved from the board's device family
//   * `commands` containing exactly one CommandSpec for the resolved
//     tool (nextpnr-ice40 / -ecp5 / -gowin), or empty for unsupported
//     families
//   * `expected_artifacts` listing the routed JSON, log, and timing
//     report paths under `build_root/pnr/`
//   * `limitations[]` always populated with the offline disclaimer

use af_backend_nextpnr::plan_nextpnr;
use std::path::Path;

#[test]
fn ice40_board_routes_to_nextpnr_ice40_tool() {
    let plan = plan_nextpnr("ice40_demo", Path::new("/tmp/br"));
    assert_eq!(plan.family, "ice40");
    assert_eq!(plan.commands.len(), 1);
    assert_eq!(plan.commands[0].program, "nextpnr-ice40");
}

#[test]
fn ecp5_board_routes_to_nextpnr_ecp5_tool() {
    let plan = plan_nextpnr("orangecrab_ecp5", Path::new("/tmp/br"));
    assert_eq!(plan.family, "ecp5");
    assert_eq!(plan.commands.len(), 1);
    assert_eq!(plan.commands[0].program, "nextpnr-ecp5");
}

#[test]
fn unsupported_family_yields_empty_commands_and_limitation() {
    let plan = plan_nextpnr("some_unsupported_board", Path::new("/tmp/br"));
    assert!(plan.commands.is_empty());
    assert!(
        plan.limitations
            .iter()
            .any(|l| l.contains("does not map to a supported nextpnr family")),
        "must surface unsupported-family limitation: {:?}",
        plan.limitations
    );
}

#[test]
fn expected_artifacts_live_under_build_root_pnr_dir() {
    let plan = plan_nextpnr("ice40_demo", Path::new("/tmp/br"));
    for artifact in &plan.expected_artifacts {
        let s = artifact.display().to_string();
        assert!(
            s.contains("/tmp/br") || artifact.starts_with("/tmp/br"),
            "artifact `{s}` must live under build_root"
        );
        assert!(s.contains("pnr"), "artifact must live in pnr/ subdir: {s}");
    }
}

#[test]
fn nextpnr_command_passes_json_netlist_and_write_routed() {
    let plan = plan_nextpnr("ice40_demo", Path::new("/tmp/br"));
    let cmd = &plan.commands[0];
    assert!(
        cmd.args.iter().any(|a| a == "--json"),
        "nextpnr argv missing --json: {:?}",
        cmd.args
    );
    assert!(
        cmd.args.iter().any(|a| a == "--write"),
        "nextpnr argv missing --write: {:?}",
        cmd.args
    );
}
