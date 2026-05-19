// SPDX-License-Identifier: Apache-2.0
#![no_main]

use af_security::{normalize_relative_path, safe_join};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let path = String::from_utf8_lossy(data);
    let _ = normalize_relative_path(&path);
    let _ = safe_join("/tmp/af-fuzz-base", &path);
});
