// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Golden-file regression for `generate_mod_add_vectors`.
//
// `tests/data/mod_add.golden.json` is the committed bytes of
// `random.json` produced for the canonical seed=0x1234567890ABCDEF,
// count=16 configuration. Any drift in:
//
//   * XorShift64 state evolution,
//   * af_field_ref::goldilocks::add_mod,
//   * VectorEntry / VectorMetadata JSON ordering or formatting,
//   * fnv1a64 hash inputs,
//
// will mismatch the bytes and fail the test loudly. Regenerate the
// golden on intentional drift:
//
//   AF_VECTORS_GOLDEN_REGENERATE=1 cargo test -p af-vectors --test golden
//
// then commit the new `mod_add.golden.json`.

use af_vectors::{generate_mod_add_vectors, GenerateConfig};
use std::fs;
use std::path::{Path, PathBuf};

const CANONICAL_SEED: &str = "0x1234567890ABCDEF";
const CANONICAL_COUNT: usize = 16;

fn golden_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("mod_add.golden.json")
}

fn generate_canonical_to(dir: &Path) -> String {
    let cfg = GenerateConfig {
        basic_out: dir.join("basic.json"),
        random_out: dir.join("random.json"),
        svh_out: dir.join("random.svh"),
        count: CANONICAL_COUNT,
        seed: CANONICAL_SEED.to_string(),
    };
    generate_mod_add_vectors(&cfg).expect("generate canonical vectors");
    fs::read_to_string(&cfg.random_out).expect("read random.json")
}

#[test]
fn canonical_output_matches_committed_golden() {
    let tmp = tempfile::TempDir::new().unwrap();
    let generated = generate_canonical_to(tmp.path());

    if std::env::var("AF_VECTORS_GOLDEN_REGENERATE").is_ok() {
        let path = golden_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, &generated).unwrap();
        eprintln!("wrote new golden to {}", path.display());
        return;
    }

    let golden = match fs::read_to_string(golden_path()) {
        Ok(text) => text,
        Err(err) => panic!(
            "missing golden file at {}: {err}\nrun with AF_VECTORS_GOLDEN_REGENERATE=1 to create it",
            golden_path().display()
        ),
    };

    assert_eq!(
        golden, generated,
        "generated random.json drifted from committed golden;\n\
         if the drift is intentional, regenerate with:\n\
         AF_VECTORS_GOLDEN_REGENERATE=1 cargo test -p af-vectors --test golden"
    );
}

#[test]
fn metadata_hash_in_golden_matches_payload() {
    let golden = fs::read_to_string(golden_path()).expect("golden present");
    let v: serde_json::Value = serde_json::from_str(&golden).expect("golden is valid JSON");
    let hash = v["metadata"]["metadata_hash"]
        .as_str()
        .expect("metadata_hash present");
    assert!(
        hash.starts_with("0x"),
        "metadata_hash is hex-prefixed: {hash}"
    );
    assert_eq!(
        hash.len(),
        18,
        "metadata_hash is 0x + 16 hex digits (u64): {hash}"
    );
    assert_eq!(
        v["metadata"]["count"].as_u64(),
        Some(CANONICAL_COUNT as u64)
    );
    assert_eq!(
        v["vectors"].as_array().unwrap().len(),
        CANONICAL_COUNT,
        "vectors[] length matches metadata.count"
    );
}
