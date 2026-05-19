// SPDX-License-Identifier: Apache-2.0
#![no_main]

use af_manifest::CoreManifest;
use af_rtl_inspector::inspect_core;
use libfuzzer_sys::fuzz_target;
use std::fs;

const MANIFEST: &str = r#"
af_version = "0.1"
name = "fuzz-core"
vendor = "accelfury"
library = "ip"
core = "fuzz_core"
version = "0.0.0"
known_limitations = ["fuzz target only"]

[metadata]
license = "Apache-2.0"
authors = ["AccelFury"]
description = "Fuzz target core"

[rtl]
top = "fuzz_core"
language = "verilog-2001"
default_clock = "clk"
default_reset = "rst_n"

[sources]
files = ["rtl/fuzz_core.v"]
include_dirs = ["rtl/include"]

[[clocks]]
name = "clk"
frequency_hz = 50_000_000

[[resets]]
name = "rst_n"
active = "low"
asynchronous = true

[[ports]]
name = "clk"
direction = "input"
width = 1
clock = "clk"
"#;

fuzz_target!(|data: &[u8]| {
    let manifest = match CoreManifest::from_toml_str(MANIFEST, "af-core.toml") {
        Ok(manifest) => manifest,
        Err(_) => return,
    };
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(_) => return,
    };
    let rtl_dir = dir.path().join("rtl");
    if fs::create_dir_all(rtl_dir.join("include")).is_err() {
        return;
    }
    let source = String::from_utf8_lossy(data);
    if fs::write(rtl_dir.join("fuzz_core.v"), source.as_bytes()).is_ok() {
        let _ = inspect_core(dir.path(), &manifest);
    }
});
