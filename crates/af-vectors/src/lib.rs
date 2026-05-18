// SPDX-License-Identifier: AGPL-3.0-or-later
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

mod vector_format;

use af_field_ref::goldilocks::{add_mod, MODULUS as GOLDILOCKS};
use serde::{Deserialize, Serialize};
use serde_json::{json, to_string, to_string_pretty};
pub use vector_format::{VectorEntry, VectorMetadata, VectorSet};

pub type DynError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct GenerateConfig {
    pub basic_out: PathBuf,
    pub random_out: PathBuf,
    pub svh_out: PathBuf,
    pub count: usize,
    pub seed: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct GenerateReport {
    pub basic_out: PathBuf,
    pub random_out: PathBuf,
    pub svh_out: PathBuf,
    pub basic_count: usize,
    pub random_count: usize,
}

impl Default for GenerateConfig {
    fn default() -> Self {
        Self {
            basic_out: PathBuf::from("vectors/af_mod_add_basic.json"),
            random_out: PathBuf::from("vectors/af_mod_add_random.json"),
            svh_out: PathBuf::from("vectors/af_mod_add_random.svh"),
            count: 64,
            seed: "0x1234567890ABCDEF".to_string(),
        }
    }
}

pub fn generate_mod_add_vectors(config: &GenerateConfig) -> Result<GenerateReport, DynError> {
    let basic = vec![
        VectorEntry {
            a: u64_to_hex(0),
            b: u64_to_hex(0),
            expected: u64_to_hex(add_mod(0, 0, GOLDILOCKS)),
        },
        VectorEntry {
            a: u64_to_hex(1),
            b: u64_to_hex(1),
            expected: u64_to_hex(add_mod(1, 1, GOLDILOCKS)),
        },
        VectorEntry {
            a: u64_to_hex(GOLDILOCKS - 1),
            b: u64_to_hex(1),
            expected: u64_to_hex(add_mod(GOLDILOCKS - 1, 1, GOLDILOCKS)),
        },
        VectorEntry {
            a: u64_to_hex(GOLDILOCKS - 1),
            b: u64_to_hex(GOLDILOCKS - 1),
            expected: u64_to_hex(add_mod(GOLDILOCKS - 1, GOLDILOCKS - 1, GOLDILOCKS)),
        },
    ];

    let seed = parse_hex_u64(&config.seed)?;
    let mut rng = XorShift64::new(seed);
    let mut random_vectors = Vec::with_capacity(config.count);
    for _ in 0..config.count {
        let a = rng.next_u64() % GOLDILOCKS;
        let b = rng.next_u64() % GOLDILOCKS;
        let expected = add_mod(a, b, GOLDILOCKS);
        random_vectors.push(VectorEntry {
            a: u64_to_hex(a),
            b: u64_to_hex(b),
            expected: u64_to_hex(expected),
        });
    }

    let modulus = u64_to_hex(GOLDILOCKS);
    let basic_set = VectorSet {
        metadata: VectorMetadata {
            ip: "af_mod_add".to_string(),
            modulus: modulus.clone(),
            seed,
            count: basic.len(),
            metadata_hash: metadata_digest("af_mod_add", &modulus, seed, basic.len(), &basic)?,
        },
        vectors: basic.clone(),
    };
    let random_set = VectorSet {
        metadata: VectorMetadata {
            ip: "af_mod_add".to_string(),
            modulus,
            seed,
            count: random_vectors.len(),
            metadata_hash: metadata_digest(
                "af_mod_add",
                &u64_to_hex(GOLDILOCKS),
                seed,
                random_vectors.len(),
                &random_vectors,
            )?,
        },
        vectors: random_vectors.clone(),
    };

    write_json(&config.basic_out, &basic_set)?;
    write_json(&config.random_out, &random_set)?;
    write_svh(&config.svh_out, &random_vectors)?;

    Ok(GenerateReport {
        basic_out: config.basic_out.clone(),
        random_out: config.random_out.clone(),
        svh_out: config.svh_out.clone(),
        basic_count: basic_set.vectors.len(),
        random_count: random_set.vectors.len(),
    })
}

fn parse_hex_u64(value: &str) -> Result<u64, DynError> {
    let clean = value.trim_start_matches("0x");
    Ok(u64::from_str_radix(clean, 16)?)
}

fn u64_to_hex(value: u64) -> String {
    format!("0x{value:016X}")
}

fn fnv1a_hash64(input: &str) -> u64 {
    let mut hash: u64 = 14_695_981_039_346_656_037;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash
}

fn metadata_digest(
    ip: &str,
    modulus: &str,
    seed: u64,
    count: usize,
    vectors: &[VectorEntry],
) -> Result<String, DynError> {
    let payload = json!({
        "ip": ip,
        "modulus": modulus,
        "seed": seed,
        "count": count,
        "vectors": vectors,
    });
    let encoded = to_string(&payload)?;
    Ok(format!("0x{:016X}", fnv1a_hash64(&encoded)))
}

#[derive(Clone)]
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(0x9E3779B97F4A7C15),
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 7;
        x ^= x >> 9;
        x ^= x << 8;
        self.state = x;
        self.state
    }
}

fn write_json(file: &Path, data: &VectorSet) -> Result<(), DynError> {
    ensure_parent(file)?;
    let mut json = to_string_pretty(data)?;
    json.push('\n');
    Ok(fs::write(file, json)?)
}

fn write_svh(file: &Path, vectors: &[VectorEntry]) -> Result<(), DynError> {
    ensure_parent(file)?;
    let mut out = String::new();
    out.push_str("// SPDX-License-Identifier: AGPL-3.0-or-later\n");
    out.push_str("// Auto-generated by af-vectors\n");
    out.push_str("`ifndef AF_MOD_ADD_RANDOM_SVH\n");
    out.push_str("`define AF_MOD_ADD_RANDOM_SVH\n");
    writeln!(
        out,
        "localparam int AF_MOD_ADD_RANDOM_COUNT = {};",
        vectors.len()
    )?;
    out.push_str(
        "localparam logic [63:0] AF_MOD_ADD_RANDOM_A [0:AF_MOD_ADD_RANDOM_COUNT-1] = '{\n",
    );
    for (idx, item) in vectors.iter().enumerate() {
        let sep = if idx + 1 == vectors.len() { "" } else { "," };
        writeln!(out, "    64'h{}{}", item.a.trim_start_matches("0x"), sep)?;
    }
    out.push_str("};\n\n");
    out.push_str(
        "localparam logic [63:0] AF_MOD_ADD_RANDOM_B [0:AF_MOD_ADD_RANDOM_COUNT-1] = '{\n",
    );
    for (idx, item) in vectors.iter().enumerate() {
        let sep = if idx + 1 == vectors.len() { "" } else { "," };
        writeln!(out, "    64'h{}{}", item.b.trim_start_matches("0x"), sep)?;
    }
    out.push_str("};\n\n");
    out.push_str(
        "localparam logic [63:0] AF_MOD_ADD_RANDOM_EXPECTED [0:AF_MOD_ADD_RANDOM_COUNT-1] = '{\n",
    );
    for (idx, item) in vectors.iter().enumerate() {
        let sep = if idx + 1 == vectors.len() { "" } else { "," };
        writeln!(
            out,
            "    64'h{}{}",
            item.expected.trim_start_matches("0x"),
            sep
        )?;
    }
    out.push_str("};\n`endif\n");
    Ok(fs::write(file, out)?)
}

fn ensure_parent(file: &Path) -> Result<(), DynError> {
    if let Some(parent) = file.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}
