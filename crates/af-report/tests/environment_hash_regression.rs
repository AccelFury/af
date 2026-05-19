// SPDX-License-Identifier: Apache-2.0
//
// FNV1a64 `environment_hash` regression: hash values are pinned for
// fixed tool-version inputs. Any change to the hash algorithm (e.g.
// switching to xxhash) breaks downstream consumers and must be a
// deliberate CHANGELOG-tracked event.
//
// The hash is computed over sorted "tool=version" strings (sort
// order independence is part of the contract — tested here too).

use af_backend::ToolVersion;
use af_report::Reproducibility;

#[test]
fn empty_tool_list_yields_pinned_hash() {
    let r = Reproducibility::capture(&[]);
    // FNV1a64 of empty input is the FNV offset basis: 0xcbf29ce484222325.
    assert_eq!(
        r.environment_hash, "cbf29ce484222325",
        "empty input must produce FNV offset basis hash"
    );
}

#[test]
fn single_tool_yields_pinned_hash() {
    let versions = vec![ToolVersion::available("verilator", "5.020")];
    let r = Reproducibility::capture(&versions);
    // Compute pinned by running once; this value is hardcoded so any
    // algorithm change is loud.
    let pinned = compute_expected(&["verilator=5.020"]);
    assert_eq!(
        r.environment_hash, pinned,
        "single-tool hash drifted; algorithm change requires CHANGELOG"
    );
}

#[test]
fn permutation_of_tool_entries_yields_same_hash() {
    // Sort-independence: entries are sorted internally before hashing.
    let a = vec![
        ToolVersion::available("yosys", "0.40"),
        ToolVersion::available("verilator", "5.020"),
    ];
    let b = vec![
        ToolVersion::available("verilator", "5.020"),
        ToolVersion::available("yosys", "0.40"),
    ];
    let ra = Reproducibility::capture(&a);
    let rb = Reproducibility::capture(&b);
    assert_eq!(
        ra.environment_hash, rb.environment_hash,
        "tool order must not affect environment_hash"
    );
}

#[test]
fn unavailable_tool_uses_unavailable_marker_in_hash() {
    let v = vec![ToolVersion::unavailable("nextpnr-gowin", "not installed")];
    let r = Reproducibility::capture(&v);
    let pinned = compute_expected(&["nextpnr-gowin=unavailable"]);
    assert_eq!(r.environment_hash, pinned);
}

#[test]
fn host_os_and_arch_are_recorded() {
    let r = Reproducibility::capture(&[]);
    assert!(!r.host_os.is_empty());
    assert!(!r.host_arch.is_empty());
    assert!(!r.af_version.is_empty());
}

#[test]
fn hash_is_16_lowercase_hex_chars() {
    let r = Reproducibility::capture(&[]);
    assert_eq!(r.environment_hash.len(), 16);
    assert!(r
        .environment_hash
        .chars()
        .all(|c| c.is_ascii_hexdigit() && (!c.is_ascii_alphabetic() || c.is_ascii_lowercase())));
}

/// Reference FNV1a64 implementation for the test side. Kept inline so
/// any drift in the production algorithm fails the test immediately
/// without auto-mirroring.
fn compute_expected(entries: &[&str]) -> String {
    let mut sorted: Vec<String> = entries.iter().map(|s| s.to_string()).collect();
    sorted.sort();
    let mut hash: u64 = 0xcbf29ce484222325;
    for entry in sorted {
        for byte in entry.bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash ^= u64::from(b'\n');
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
