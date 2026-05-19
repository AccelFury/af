// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Property-based fuzz of `generate_mod_add_vectors`:
//   - any valid hex seed (u64 range) must not panic
//   - same seed + same count ⇒ byte-identical bytes
//   - different counts ⇒ different random_count in report

use af_vectors::{generate_mod_add_vectors, GenerateConfig};
use proptest::prelude::*;
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

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// Arbitrary u64 seed ⇒ generator runs to completion or returns a
    /// structured error. No panic.
    #[test]
    fn any_seed_never_panics(seed_value: u64, count in 0usize..32) {
        let tmp = TempDir::new().unwrap();
        let cfg = config_in(tmp.path(), &format!("0x{seed_value:016X}"), count);
        let result = std::panic::catch_unwind(|| generate_mod_add_vectors(&cfg));
        prop_assert!(result.is_ok(), "generator must never panic");
    }

    /// Same seed + same count ⇒ byte-identical files across runs.
    #[test]
    fn same_seed_byte_identical(seed_value: u64, count in 1usize..16) {
        let a = TempDir::new().unwrap();
        let b = TempDir::new().unwrap();
        let seed = format!("0x{seed_value:016X}");
        let cfg_a = config_in(a.path(), &seed, count);
        let cfg_b = config_in(b.path(), &seed, count);
        let _ = generate_mod_add_vectors(&cfg_a).expect("run A");
        let _ = generate_mod_add_vectors(&cfg_b).expect("run B");
        for name in ["basic.json", "random.json", "random.svh"] {
            let ya = fs::read(a.path().join(name)).unwrap();
            let yb = fs::read(b.path().join(name)).unwrap();
            prop_assert_eq!(ya, yb, "file {} drifted for seed {}", name, seed);
        }
    }

    /// Report's random_count exactly equals the requested count.
    #[test]
    fn random_count_matches_request(count in 0usize..32) {
        let tmp = TempDir::new().unwrap();
        let cfg = config_in(tmp.path(), "0x1234567890ABCDEF", count);
        let report = generate_mod_add_vectors(&cfg).expect("run");
        prop_assert_eq!(report.random_count, count);
    }
}
