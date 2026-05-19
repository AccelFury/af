// SPDX-License-Identifier: Apache-2.0
//
// Integration tests for `af-board-db` public API:
//
//   - `list_boards(root)` ⇒ Vec<BoardEntry>
//   - `load_registry_boards(path)` ⇒ Vec<BoardEntry>
//   - `load_board_aliases(path)` ⇒ Vec<BoardAlias>
//   - `resolve_board_id(id, &aliases)` ⇒ canonical name
//   - `check_registry(root)` ⇒ RegistryCheckReport with structured issues
//
// We exercise:
//   * Real in-repo `registries/` data (happy path).
//   * Synthetic fixtures injecting duplicate IDs, alias collisions,
//     dangling alias targets, malformed JSON.
//   * `resolve_board_id` semantics (alias hit and miss).

use af_board_db::{
    check_registry, list_boards, load_board_aliases, load_registry_boards, resolve_board_id,
    BoardAlias, BoardDbError,
};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn copy_registries_into(dst: &Path) {
    let src = repo_root().join("registries");
    let dst_reg = dst.join("registries");
    fs::create_dir_all(&dst_reg).unwrap();
    for entry in fs::read_dir(&src).unwrap().flatten() {
        let p = entry.path();
        if p.is_file() {
            fs::copy(&p, dst_reg.join(p.file_name().unwrap())).unwrap();
        }
    }
}

#[test]
fn list_boards_against_in_repo_data() {
    let boards = list_boards(repo_root()).expect("list_boards must succeed");
    assert!(!boards.is_empty(), "in-repo registry must list ≥1 board");

    // Known boards from the registry (smoke check; if any of these is
    // intentionally removed update both this list and the registry).
    let ids: std::collections::BTreeSet<String> =
        boards.iter().map(|b| b.board_id.clone()).collect();
    for known in ["sipeed_tang_nano_1k", "digilent_arty_a7"] {
        assert!(
            ids.contains(known),
            "registry must contain known board `{known}`; got {ids:?}"
        );
    }
}

#[test]
fn check_registry_on_real_data_returns_valid_report() {
    let report = check_registry(repo_root()).expect("check_registry must succeed");
    assert!(report.board_count >= 1);
    // The in-repo data MAY have legitimate issues; we just gate against
    // hard-rule problems (duplicate IDs, alias collisions). If it does,
    // surface them clearly.
    let critical: Vec<&str> = report
        .issues
        .iter()
        .filter(|i| {
            matches!(
                i.code.as_str(),
                "AF_BOARD_DUPLICATE_ID" | "AF_BOARD_ALIAS_COLLIDES"
            )
        })
        .map(|i| i.code.as_str())
        .collect();
    assert!(
        critical.is_empty(),
        "in-repo registry must not have duplicate IDs or alias collisions: {critical:?}"
    );
}

#[test]
fn duplicate_board_id_is_flagged() {
    let tmp = TempDir::new().unwrap();
    copy_registries_into(tmp.path());
    // Inject duplicate id by appending a clone of the first entry.
    let reg = tmp.path().join("registries/boards.registry.json");
    let mut text = fs::read_to_string(&reg).unwrap();
    text = text.replace(
        "\"board_id\": \"sipeed_tang_nano_1k\"",
        "\"board_id\": \"sipeed_tang_nano_1k\", \"_duplicate\": 1",
    );
    // Add a second entry with the same id via raw JSON append:
    let dup_entry = r#",
    {
      "board_id": "sipeed_tang_nano_1k",
      "display_name": "Sipeed Tang Nano 1K (duplicate)",
      "vendor": "gowin",
      "fpga_family": "gowin_gw1n",
      "fpga_part_if_known_or_template": "GW1N-?",
      "logic_size_class": "small",
      "dsp_class": "low",
      "memory_class": "none",
      "high_speed_io_class": "none",
      "default_toolchain": "gowin_eda",
      "alternative_toolchains": ["gowin_eda"],
      "constraint_format": "cst",
      "board_dir": "boards/gowin/sipeed_tang_nano_1k",
      "exact_pinout_status": "draft_placeholder",
      "safe_for_beginner": false,
      "suggested_ip_classes": [],
      "excluded_ip_classes": [],
      "notes": "duplicate"
    }"#;
    text = text.replacen("\n  ]", &format!("{dup_entry}\n  ]"), 1);
    fs::write(&reg, &text).unwrap();

    let report = check_registry(tmp.path()).expect("check_registry must surface duplicates");
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.code == "AF_BOARD_DUPLICATE_ID"),
        "duplicate board id must raise AF_BOARD_DUPLICATE_ID: {:?}",
        report.issues
    );
}

#[test]
fn alias_targeting_unknown_board_is_flagged() {
    let tmp = TempDir::new().unwrap();
    copy_registries_into(tmp.path());
    let aliases_path = tmp.path().join("registries/board_aliases.json");
    let payload = r#"{
      "aliases": [
        { "alias": "totally-fake-alias", "canonical": "no-such-canonical-board" }
      ]
    }"#;
    fs::write(&aliases_path, payload).unwrap();

    let report = check_registry(tmp.path()).expect("check_registry");
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.code == "AF_BOARD_ALIAS_TARGET_UNKNOWN"),
        "alias to unknown canonical board must raise AF_BOARD_ALIAS_TARGET_UNKNOWN: {:?}",
        report.issues
    );
}

#[test]
fn alias_colliding_with_canonical_id_is_flagged() {
    let tmp = TempDir::new().unwrap();
    copy_registries_into(tmp.path());
    let aliases_path = tmp.path().join("registries/board_aliases.json");
    // An alias whose name matches an existing canonical board id.
    let payload = r#"{
      "aliases": [
        { "alias": "digilent_arty_a7", "canonical": "digilent_arty_a7" }
      ]
    }"#;
    fs::write(&aliases_path, payload).unwrap();

    let report = check_registry(tmp.path()).expect("check_registry");
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.code == "AF_BOARD_ALIAS_COLLIDES"),
        "alias colliding with canonical id must raise AF_BOARD_ALIAS_COLLIDES: {:?}",
        report.issues
    );
}

#[test]
fn malformed_registry_json_returns_parse_error() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("registries")).unwrap();
    fs::write(
        tmp.path().join("registries/boards.registry.json"),
        b"{ this is not valid json [[",
    )
    .unwrap();
    let res = load_registry_boards(tmp.path().join("registries/boards.registry.json"));
    match res {
        Err(BoardDbError::Parse { .. }) | Err(BoardDbError::Read { .. }) => {}
        other => panic!("malformed registry should fail with Parse/Read error, got {other:?}"),
    }
}

#[test]
fn missing_aliases_file_returns_empty_list_not_error() {
    let tmp = TempDir::new().unwrap();
    let no_such = tmp.path().join("registries/board_aliases.json");
    let aliases = load_board_aliases(&no_such).expect("absent file must not error");
    assert!(aliases.is_empty());
}

#[test]
fn resolve_board_id_returns_canonical_on_alias_hit() {
    let aliases = vec![
        BoardAlias {
            alias: "legacy-name".to_string(),
            canonical: "canonical-board".to_string(),
            reason: None,
        },
        BoardAlias {
            alias: "old-name".to_string(),
            canonical: "current-board".to_string(),
            reason: None,
        },
    ];
    assert_eq!(resolve_board_id("legacy-name", &aliases), "canonical-board");
    assert_eq!(resolve_board_id("old-name", &aliases), "current-board");
}

#[test]
fn resolve_board_id_returns_input_on_alias_miss() {
    let aliases = vec![BoardAlias {
        alias: "old".to_string(),
        canonical: "new".to_string(),
        reason: None,
    }];
    // Unknown id passes through unchanged.
    assert_eq!(resolve_board_id("unknown", &aliases), "unknown");
}
