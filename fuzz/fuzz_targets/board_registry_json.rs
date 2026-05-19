// SPDX-License-Identifier: Apache-2.0
#![no_main]

use af_board_db::load_registry_boards;
use libfuzzer_sys::fuzz_target;
use std::fs;

fuzz_target!(|data: &[u8]| {
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(_) => return,
    };
    let path = dir.path().join("boards.registry.json");
    if fs::write(&path, data).is_ok() {
        let _ = load_registry_boards(&path);
    }
});
