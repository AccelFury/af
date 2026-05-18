// SPDX-License-Identifier: AGPL-3.0-or-later
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    pub a: String,
    pub b: String,
    pub expected: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSet {
    pub metadata: VectorMetadata,
    pub vectors: Vec<VectorEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorMetadata {
    pub ip: String,
    pub modulus: String,
    pub seed: u64,
    pub count: usize,
    pub metadata_hash: String,
}
