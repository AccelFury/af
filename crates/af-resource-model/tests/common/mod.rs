// SPDX-License-Identifier: Apache-2.0
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

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

pub fn clone_example(name: &str) -> TempDir {
    let src = repo_root().join("examples").join(name);
    let tmp = TempDir::new().expect("tempdir");
    copy_dir_all(&src, tmp.path());
    tmp
}

/// Append a TOML fragment to `af-core.toml` of the cloned project.
pub fn append_to_manifest(project: &Path, fragment: &str) {
    let manifest = project.join("af-core.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(fragment);
    fs::write(&manifest, text).unwrap();
}
