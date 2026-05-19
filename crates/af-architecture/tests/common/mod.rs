// SPDX-License-Identifier: Apache-2.0
// Test helpers shared across the af-architecture integration tests.

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Locate the workspace root from `CARGO_MANIFEST_DIR` so tests resolve
/// `examples/...` no matter where cargo invoked them from.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

/// Recursively copy a directory tree. Mirrors `cp -R`. Symlinks become
/// regular files (good enough for test fixtures).
pub fn copy_dir_all(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).expect("create dst");
    for entry in fs::read_dir(src).expect("read src") {
        let entry = entry.expect("entry");
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let ty = entry.file_type().expect("file type");
        if ty.is_dir() {
            copy_dir_all(&from, &to);
        } else {
            fs::copy(&from, &to).expect("copy file");
        }
    }
}

/// Clone an in-tree example (`examples/<name>`) into a fresh temp dir.
pub fn clone_example(name: &str) -> TempDir {
    let src = repo_root().join("examples").join(name);
    assert!(
        src.is_dir(),
        "example fixture {} missing at {}",
        name,
        src.display()
    );
    let tmp = TempDir::new().expect("tempdir");
    copy_dir_all(&src, tmp.path());
    tmp
}
