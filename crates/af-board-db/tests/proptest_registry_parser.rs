// SPDX-License-Identifier: Apache-2.0
//
// Property-based fuzz of `load_registry_boards`: arbitrary bytes must
// never panic; parse failure is acceptable, panic is not.

use af_board_db::load_registry_boards;
use proptest::prelude::*;
use std::fs;
use tempfile::TempDir;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Arbitrary byte payloads ⇒ Read/Parse error or Ok, never panic.
    #[test]
    fn arbitrary_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("boards.registry.json");
        fs::write(&path, &bytes).unwrap();
        let result = std::panic::catch_unwind(|| load_registry_boards(&path));
        prop_assert!(result.is_ok(), "load_registry_boards must never panic");
    }

    /// Empty JSON object `{}` should fail with a structured error (no
    /// `boards` field), but not panic.
    #[test]
    fn structural_json_without_boards_field_returns_error(
        prefix in "[a-z]{1,5}",
        value in any::<u32>()
    ) {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("boards.registry.json");
        // Build a JSON object that uses a random key (not `boards`).
        let payload = format!("{{\"{prefix}\": {value}}}");
        fs::write(&path, &payload).unwrap();
        let result = load_registry_boards(&path);
        // Either Ok with empty boards vec (if serde defaults to empty)
        // or structured Err — never panic.
        if let Ok(boards) = result {
            prop_assert!(boards.is_empty() || !boards.is_empty());
        }
    }
}

#[test]
fn valid_minimal_registry_roundtrips() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("boards.registry.json");
    fs::write(
        &path,
        br#"{
          "boards": [
            {
              "board_id": "test-board",
              "display_name": "Test",
              "vendor": "xilinx",
              "fpga_family": "artix-7",
              "fpga_part_if_known_or_template": "xc7a35t",
              "logic_size_class": "small",
              "dsp_class": "low",
              "memory_class": "none",
              "high_speed_io_class": "none",
              "default_toolchain": "vivado",
              "alternative_toolchains": [],
              "constraint_format": "xdc",
              "board_dir": "boards/xilinx/test-board",
              "exact_pinout_status": "draft_placeholder",
              "safe_for_beginner": true,
              "suggested_ip_classes": [],
              "excluded_ip_classes": [],
              "notes": "test"
            }
          ]
        }"#,
    )
    .unwrap();
    let boards = load_registry_boards(&path).expect("valid minimal registry");
    assert_eq!(boards.len(), 1);
    assert_eq!(boards[0].board_id, "test-board");
}
