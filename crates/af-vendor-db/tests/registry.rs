// SPDX-License-Identifier: Apache-2.0
//
// Locks the vendor-resolution and capability matrix that downstream
// crates (af-resource-model, af-compatibility) build on top of. A change
// to `board_alias` or `capability()` is a public-contract change.

use af_vendor_db::{capability, resolve_target};

#[test]
fn known_board_resolves_to_documented_vendor_family() {
    let cases = &[
        ("sipeed_tang_nano_20k", "gowin", "gw2a"),
        ("sipeed_tang_nano_9k", "gowin", "gw1n"),
        ("digilent_arty_a7", "xilinx", "artix-7"),
        ("digilent_basys3_artix7", "xilinx", "artix-7"),
        ("alveo-u55c", "xilinx", "ultrascale-plus"),
        ("orangecrab_ecp5", "lattice", "ecp5"),
    ];
    for (board, vendor, family) in cases {
        let target = resolve_target(None, None, Some(board));
        assert_eq!(target.vendor, *vendor, "vendor for {board}");
        assert_eq!(
            target.family.as_deref(),
            Some(*family),
            "family for {board}"
        );
        assert_eq!(target.board.as_deref(), Some(*board), "board echoed");
    }
}

#[test]
fn unknown_board_falls_back_to_explicit_vendor() {
    let target = resolve_target(Some("xilinx"), Some("kintex-7"), Some("unknown-board"));
    assert_eq!(target.vendor, "xilinx");
    assert_eq!(target.family.as_deref(), Some("kintex-7"));
    assert_eq!(target.board.as_deref(), Some("unknown-board"));
}

#[test]
fn no_input_defaults_to_generic() {
    let target = resolve_target(None, None, None);
    assert_eq!(target.vendor, "generic");
    assert!(target.family.is_none());
    assert!(target.board.is_none());
}

#[test]
fn known_vendors_advertise_bram_dsp_pll() {
    for vendor in ["xilinx", "intel", "gowin"] {
        let target = resolve_target(Some(vendor), None, None);
        let cap = capability(&target);
        assert_eq!(cap.vendor, vendor);
        assert!(cap.bram, "vendor {vendor} must advertise BRAM");
        assert!(cap.dsp, "vendor {vendor} must advertise DSP");
        assert!(cap.pll, "vendor {vendor} must advertise PLL");
    }
}

#[test]
fn lattice_ecp5_has_dsp_but_ice40_does_not() {
    let ecp5 = capability(&resolve_target(Some("lattice"), Some("ecp5"), None));
    assert!(ecp5.dsp, "lattice/ecp5 has DSP blocks");

    let ice40 = capability(&resolve_target(Some("lattice"), Some("ice40-lp8k"), None));
    assert!(!ice40.dsp, "lattice/ice40 has no DSP blocks");
    assert!(ice40.bram, "lattice/ice40 still has BRAM");
}

#[test]
fn unknown_vendor_returns_safe_generic_capability() {
    let cap = capability(&resolve_target(
        Some("definitely-not-a-real-vendor"),
        None,
        None,
    ));
    assert!(!cap.bram, "unknown vendor must not falsely advertise BRAM");
    assert!(!cap.dsp);
    assert!(!cap.pll);
    assert!(cap.hard_ip.is_empty());
    assert!(
        cap.notes.iter().any(|n| n.contains("Unknown target")),
        "unknown vendor must surface a note"
    );
}

#[test]
fn xilinx_capability_lists_hard_ip_kinds() {
    let cap = capability(&resolve_target(
        Some("xilinx"),
        Some("ultrascale-plus"),
        None,
    ));
    assert!(cap.hard_ip.iter().any(|s| s.contains("pcie")));
    assert!(cap.hard_ip.iter().any(|s| s.contains("ddr")));
}

#[test]
fn capability_struct_round_trips_through_json() {
    let cap = capability(&resolve_target(Some("xilinx"), Some("artix-7"), None));
    let s = serde_json::to_string(&cap).unwrap();
    let back: af_vendor_db::VendorCapability = serde_json::from_str(&s).unwrap();
    assert_eq!(cap, back);
}
