// SPDX-License-Identifier: Apache-2.0
#![no_main]

use af_manifest::CoreManifest;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let raw = String::from_utf8_lossy(data);
    let _ = CoreManifest::from_toml_str(&raw, "fuzz/af-core.toml");
});
