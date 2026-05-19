// SPDX-License-Identifier: AGPL-3.0-or-later
//
// `generate_mod_add_vectors` must be byte-for-byte deterministic given
// the same `seed` and `count`: re-running it produces the same JSON and
// the same SystemVerilog header. The XorShift64 + Goldilocks add_mod
// pair is the reference path used by hardware testbenches; any drift
// would silently invalidate every committed signoff trace that relies
// on the metadata_digest.

use af_vectors::{generate_mod_add_vectors, GenerateConfig};
use std::fs;
use tempfile::TempDir;

fn config_in(dir: &std::path::Path, seed: &str, count: usize) -> GenerateConfig {
    GenerateConfig {
        basic_out: dir.join("basic.json"),
        random_out: dir.join("random.json"),
        svh_out: dir.join("random.svh"),
        count,
        seed: seed.to_string(),
    }
}

#[test]
fn same_seed_produces_byte_identical_outputs() {
    let a = TempDir::new().unwrap();
    let b = TempDir::new().unwrap();
    let cfg_a = config_in(a.path(), "0x1234567890ABCDEF", 32);
    let cfg_b = config_in(b.path(), "0x1234567890ABCDEF", 32);

    let _ = generate_mod_add_vectors(&cfg_a).expect("first run");
    let _ = generate_mod_add_vectors(&cfg_b).expect("second run");

    for name in ["basic.json", "random.json", "random.svh"] {
        let lhs = fs::read(a.path().join(name)).unwrap();
        let rhs = fs::read(b.path().join(name)).unwrap();
        assert_eq!(
            lhs, rhs,
            "vectors must be byte-identical for the same seed: {name} diverged"
        );
    }
}

#[test]
fn different_seed_diverges() {
    let a = TempDir::new().unwrap();
    let b = TempDir::new().unwrap();
    let cfg_a = config_in(a.path(), "0x1234567890ABCDEF", 8);
    let cfg_b = config_in(b.path(), "0xCAFEBABEDEADBEEF", 8);

    let _ = generate_mod_add_vectors(&cfg_a).expect("seed A");
    let _ = generate_mod_add_vectors(&cfg_b).expect("seed B");

    let r_a = fs::read(a.path().join("random.json")).unwrap();
    let r_b = fs::read(b.path().join("random.json")).unwrap();
    assert_ne!(
        r_a, r_b,
        "different seeds must produce different random vectors"
    );

    // The `basic.json` *vectors[]* array is seed-independent (the
    // metadata.seed field tracks the seed and so will legitimately
    // diverge). Compare only the vector entries.
    let json_a: serde_json::Value =
        serde_json::from_slice(&fs::read(a.path().join("basic.json")).unwrap()).unwrap();
    let json_b: serde_json::Value =
        serde_json::from_slice(&fs::read(b.path().join("basic.json")).unwrap()).unwrap();
    assert_eq!(
        json_a["vectors"], json_b["vectors"],
        "basic vector entries are seed-independent and must match across seeds"
    );
}

#[test]
fn count_is_respected_in_random_set() {
    let dir = TempDir::new().unwrap();
    let cfg = config_in(dir.path(), "0x1234567890ABCDEF", 17);
    let report = generate_mod_add_vectors(&cfg).expect("generate");
    assert_eq!(report.random_count, 17);
    assert_eq!(report.basic_count, 4, "basic suite always has 4 entries");
}

#[test]
fn output_files_are_created_with_parent_dirs() {
    let dir = TempDir::new().unwrap();
    let nested = dir.path().join("a/b/c");
    let cfg = GenerateConfig {
        basic_out: nested.join("basic.json"),
        random_out: nested.join("random.json"),
        svh_out: nested.join("random.svh"),
        count: 4,
        seed: "0x1234567890ABCDEF".to_string(),
    };
    generate_mod_add_vectors(&cfg).expect("generate with nested dirs");
    for name in ["basic.json", "random.json", "random.svh"] {
        assert!(
            nested.join(name).is_file(),
            "{name} must exist under nested out dir"
        );
    }
}

#[test]
fn invalid_seed_string_returns_error() {
    let dir = TempDir::new().unwrap();
    let cfg = config_in(dir.path(), "not-a-hex-string", 4);
    let res = generate_mod_add_vectors(&cfg);
    assert!(res.is_err(), "non-hex seed must fail");
}

#[test]
fn metadata_hash_is_stable_across_runs() {
    let a = TempDir::new().unwrap();
    let b = TempDir::new().unwrap();
    let cfg_a = config_in(a.path(), "0x1234567890ABCDEF", 16);
    let cfg_b = config_in(b.path(), "0x1234567890ABCDEF", 16);
    generate_mod_add_vectors(&cfg_a).unwrap();
    generate_mod_add_vectors(&cfg_b).unwrap();
    let json_a: serde_json::Value =
        serde_json::from_slice(&fs::read(a.path().join("random.json")).unwrap()).unwrap();
    let json_b: serde_json::Value =
        serde_json::from_slice(&fs::read(b.path().join("random.json")).unwrap()).unwrap();
    let hash_a = json_a["metadata"]["metadata_hash"].as_str().unwrap();
    let hash_b = json_b["metadata"]["metadata_hash"].as_str().unwrap();
    assert_eq!(hash_a, hash_b, "metadata_hash must be deterministic");
    assert!(hash_a.starts_with("0x"), "metadata_hash is hex-prefixed");
}
