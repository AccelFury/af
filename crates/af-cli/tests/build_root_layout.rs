// SPDX-License-Identifier: Apache-2.0
//
// Hard-rule #2 (CLAUDE.md): build artifacts live under `--build-root`
// (default `.af-build/`). Never write outside it.
//
// This test exercises commands that touch the filesystem and pins:
//
// 1. `af doctor --json --build-root <tmp>` writes log files under
//    `<tmp>/logs/` and nothing outside `<tmp>`.
// 2. `af clean` without `--yes` ⇒ `AF_CLEAN_CONFIRMATION_REQUIRED`,
//    `<tmp>` untouched.
// 3. `af clean --yes` ⇒ `<tmp>` removed; sibling paths untouched.
// 4. `af clean --yes` on a non-existent path ⇒ `status: passed`,
//    `removed: false`.
// 5. `--build-root` resolution: explicit `--build-root <path>` is
//    honored over default `.af-build/`.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn repo_root() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn af() -> Command {
    Command::cargo_bin("af").expect("cargo bin `af`")
}

fn count_files(dir: &std::path::Path) -> usize {
    if !dir.is_dir() {
        return 0;
    }
    let mut n = 0;
    fn walk(dir: &std::path::Path, n: &mut usize) {
        for e in fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, n);
            } else {
                *n += 1;
            }
        }
    }
    walk(dir, &mut n);
    n
}

#[test]
fn doctor_writes_logs_under_build_root() {
    let tmp = TempDir::new().unwrap();
    let sibling = TempDir::new().unwrap();
    let sibling_before = count_files(sibling.path());

    let out = af()
        .args([
            "--json",
            "--build-root",
            tmp.path().to_str().unwrap(),
            "doctor",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success(), "doctor must succeed");

    let logs_dir = tmp.path().join("logs");
    assert!(
        logs_dir.is_dir(),
        "doctor must create <build-root>/logs/: {}",
        logs_dir.display()
    );
    let logs_count = count_files(&logs_dir);
    assert!(
        logs_count >= 1,
        "doctor must write ≥1 log file under <build-root>/logs/"
    );

    // Sibling dir untouched.
    assert_eq!(
        count_files(sibling.path()),
        sibling_before,
        "doctor must not write outside its --build-root"
    );
}

#[test]
fn clean_without_yes_refuses_with_envelope() {
    let tmp = TempDir::new().unwrap();
    // Pre-populate so `clean` has something to refuse.
    fs::create_dir_all(tmp.path().join("logs")).unwrap();
    fs::write(tmp.path().join("logs/marker.log"), b"present").unwrap();

    let out = af()
        .args([
            "--json",
            "--build-root",
            tmp.path().to_str().unwrap(),
            "clean",
        ])
        .output()
        .expect("execute");
    assert!(!out.status.success(), "clean without --yes must fail");
    assert_eq!(out.status.code(), Some(2));
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert_eq!(
        value["code"].as_str(),
        Some("AF_CLEAN_CONFIRMATION_REQUIRED")
    );
    // File must still exist.
    assert!(tmp.path().join("logs/marker.log").exists());
}

#[test]
fn clean_yes_removes_build_root_and_only_that() {
    let tmp = TempDir::new().unwrap();
    let sibling = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("logs")).unwrap();
    fs::write(tmp.path().join("logs/marker.log"), b"present").unwrap();
    fs::write(sibling.path().join("untouched.txt"), b"safe").unwrap();

    let out = af()
        .args([
            "--json",
            "--build-root",
            tmp.path().to_str().unwrap(),
            "clean",
            "--yes",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success(), "clean --yes must succeed");
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert_eq!(value["status"].as_str(), Some("passed"));
    assert_eq!(value["removed"].as_bool(), Some(true));

    assert!(
        !tmp.path().exists(),
        "tmp build-root must be removed after clean --yes"
    );
    assert!(
        sibling.path().join("untouched.txt").exists(),
        "sibling dir must be untouched"
    );
}

#[test]
fn clean_yes_on_nonexistent_path_reports_already_clean() {
    let parent = TempDir::new().unwrap();
    let never = parent.path().join("never-existed");
    assert!(!never.exists());

    let out = af()
        .args([
            "--json",
            "--build-root",
            never.to_str().unwrap(),
            "clean",
            "--yes",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success());
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert_eq!(value["status"].as_str(), Some("passed"));
    assert_eq!(value["removed"].as_bool(), Some(false));
}

#[test]
fn manifest_validate_does_not_write_to_build_root() {
    // Pure-Rust validation must not touch the build root.
    let tmp = TempDir::new().unwrap();
    let manifest = repo_root()
        .join("examples")
        .join("af-mod-add")
        .join("af-core.toml");
    let out = af()
        .args([
            "--json",
            "--build-root",
            tmp.path().to_str().unwrap(),
            "manifest",
            "validate",
            manifest.to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    assert!(out.status.success(), "manifest validate must succeed");
    // The build root may be created (empty directory) but must not be
    // populated with arbitrary files for a pure-Rust validation.
    let n = count_files(tmp.path());
    assert!(
        n == 0,
        "manifest validate must not write files to build-root (found {n})"
    );
}

#[test]
fn explicit_build_root_overrides_default() {
    // Run from a temp working dir to guarantee `.af-build` (the default)
    // would land in cwd. Pass --build-root <tmp> and assert nothing
    // appeared in cwd.
    let cwd = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let out = af()
        .current_dir(cwd.path())
        .args([
            "--json",
            "--build-root",
            br.path().to_str().unwrap(),
            "doctor",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success());
    let cwd_files = count_files(cwd.path());
    assert_eq!(
        cwd_files, 0,
        "cwd must remain empty when --build-root is specified explicitly"
    );
    assert!(
        br.path().join("logs").is_dir(),
        "specified --build-root must receive doctor's logs"
    );
}
