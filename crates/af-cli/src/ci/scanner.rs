// SPDX-License-Identifier: Apache-2.0

use crate::ci::config::CiPathsConfig;
use regex::Regex;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::{fs, mem};

#[derive(Debug, Clone)]
pub struct RepoScan {
    pub repo_root: PathBuf,
    pub rtl_files: Vec<PathBuf>,
    pub tb_files: Vec<PathBuf>,
    pub sim_files: Vec<PathBuf>,
    pub formal_files: Vec<PathBuf>,
    pub board_files: Vec<PathBuf>,
    pub sim_makefiles: Vec<PathBuf>,
    pub constraints: Vec<PathBuf>,
    pub workflows: Vec<PathBuf>,
    pub top_candidates: Vec<String>,
    pub board_top_candidates: Vec<String>,
    pub has_make_test_target: bool,
    pub has_sby: bool,
}

#[derive(Debug)]
pub struct ScanInput {
    pub repo_root: PathBuf,
    pub paths: CiPathsConfig,
}

impl RepoScan {
    pub fn top_candidates(&self) -> &[String] {
        &self.top_candidates
    }

    fn new(repo_root: PathBuf) -> Self {
        Self {
            repo_root,
            rtl_files: Vec::new(),
            tb_files: Vec::new(),
            sim_files: Vec::new(),
            formal_files: Vec::new(),
            board_files: Vec::new(),
            sim_makefiles: Vec::new(),
            constraints: Vec::new(),
            workflows: Vec::new(),
            top_candidates: Vec::new(),
            board_top_candidates: Vec::new(),
            has_make_test_target: false,
            has_sby: false,
        }
    }
}

pub fn scan_repo(repo_root: &Path, paths: &CiPathsConfig) -> RepoScan {
    let mut scan = RepoScan::new(repo_root.to_path_buf());
    scan.scan_paths(repo_root, paths);
    scan
}

impl RepoScan {
    fn scan_paths(&mut self, repo_root: &Path, paths: &CiPathsConfig) {
        scan_file_list(
            repo_root,
            &paths.rtl,
            &mut self.rtl_files,
            &mut self.top_candidates,
        );
        scan_file_list(repo_root, &paths.tb, &mut self.tb_files, &mut Vec::new());
        scan_file_list(repo_root, &paths.sim, &mut self.sim_files, &mut Vec::new());
        scan_file_list(
            repo_root,
            &paths.formal,
            &mut self.formal_files,
            &mut Vec::new(),
        );
        scan_file_list(
            repo_root,
            &paths.boards,
            &mut self.board_files,
            &mut self.board_top_candidates,
        );
        detect_sim_makefiles(
            repo_root,
            &paths.sim,
            &mut self.sim_makefiles,
            &mut self.has_make_test_target,
        );
        scan_workflows(repo_root, &mut self.workflows);
        scan_constraints(
            repo_root,
            &mut self.constraints,
            &mut self.has_sby,
            &mut self.formal_files,
        );

        dedup_path_lists(&mut self.rtl_files);
        dedup_path_lists(&mut self.tb_files);
        dedup_path_lists(&mut self.sim_files);
        dedup_path_lists(&mut self.formal_files);
        dedup_path_lists(&mut self.board_files);
        dedup_path_lists(&mut self.sim_makefiles);
        dedup_path_lists(&mut self.constraints);
        dedup_path_lists(&mut self.workflows);
        dedup_string_lists(&mut self.board_top_candidates);
        self.top_candidates.sort();
        self.top_candidates.dedup();
        self.board_top_candidates.sort();
        self.board_top_candidates.dedup();
    }
}

fn scan_file_list(
    repo_root: &Path,
    roots: &[String],
    files: &mut Vec<PathBuf>,
    top_candidates: &mut Vec<String>,
) {
    for root in roots {
        let target = repo_root.join(root);
        collect_source_files(&target, files, top_candidates);
    }
}

fn detect_sim_makefiles(
    repo_root: &Path,
    sim_roots: &[String],
    makefiles: &mut Vec<PathBuf>,
    has_make_test_target: &mut bool,
) {
    for root in sim_roots {
        let dir = repo_root.join(root);
        let makefile = dir.join("Makefile");
        if !makefile.is_file() {
            continue;
        }
        *has_make_test_target =
            parse_make_test_target(&makefile).unwrap_or(false) || *has_make_test_target;
        makefiles.push(makefile);
    }
}

fn parse_make_test_target(makefile: &Path) -> Result<bool, std::io::Error> {
    let text = fs::read_to_string(makefile)?;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("test:") || trimmed.starts_with("test :") {
            return Ok(true);
        }
    }
    Ok(false)
}

fn scan_workflows(repo_root: &Path, out: &mut Vec<PathBuf>) {
    let workflows = repo_root.join(".github/workflows");
    if !workflows.is_dir() {
        return;
    }
    if let Ok(entries) = fs::read_dir(workflows) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
                if ext == "yml" || ext == "yaml" {
                    out.push(path);
                }
            }
        }
    }
}

fn scan_constraints(
    repo_root: &Path,
    constraints: &mut Vec<PathBuf>,
    has_sby: &mut bool,
    formal_files: &mut Vec<PathBuf>,
) {
    let mut stack = vec![repo_root.join("boards"), repo_root.join(".")];
    while let Some(current) = stack.pop() {
        if !current.is_dir() {
            continue;
        }
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
                continue;
            };
            match ext {
                "cst" | "lpf" | "pcf" | "xdc" | "sdc" | "qsf" => {
                    constraints.push(path);
                }
                "sby" => {
                    *has_sby = true;
                    formal_files.push(path);
                }
                _ => {}
            }
        }
    }
}

fn collect_source_files(root: &Path, files: &mut Vec<PathBuf>, top_candidates: &mut Vec<String>) {
    if !root.is_dir() {
        return;
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if !path.is_file() {
                continue;
            }
            let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
                continue;
            };
            if !matches!(ext, "v" | "sv" | "vhd" | "vhdl") {
                continue;
            }
            files.push(path.clone());
            if matches!(ext, "v" | "sv") {
                if let Ok(source) = fs::read_to_string(&path) {
                    collect_module_names(&source, top_candidates);
                }
            }
        }
    }
}

fn collect_module_names(source: &str, out: &mut Vec<String>) {
    let stripped = strip_comments(source);
    let re = Regex::new(r"(?m)^\s*module\s+([A-Za-z_]\w*)\b").expect("module regex must compile");
    for capture in re.captures_iter(&stripped) {
        if let Some(name) = capture.get(1) {
            out.push(name.as_str().to_string());
        }
    }
}

fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut iter = source.chars().peekable();
    let mut in_block_comment = false;

    while let Some(ch) = iter.next() {
        if in_block_comment {
            if ch == '*' && iter.peek() == Some(&'/') {
                let _ = iter.next();
                in_block_comment = false;
            }
            continue;
        }

        if ch == '/' && iter.peek() == Some(&'/') {
            for inner in iter.by_ref() {
                if inner == '\n' {
                    out.push('\n');
                    break;
                }
            }
            continue;
        }
        if ch == '/' && iter.peek() == Some(&'*') {
            let _ = iter.next();
            in_block_comment = true;
            continue;
        }
        out.push(ch);
    }
    out
}

fn dedup_path_lists(paths: &mut Vec<PathBuf>) {
    let mut set = BTreeSet::new();
    let mut next = Vec::new();
    for path in mem::take(paths) {
        if set.insert(path.clone()) {
            next.push(path);
        }
    }
    *paths = next;
}

fn dedup_string_lists(paths: &mut Vec<String>) {
    let mut set = BTreeSet::new();
    let mut next = Vec::new();
    for path in mem::take(paths) {
        if set.insert(path.clone()) {
            next.push(path);
        }
    }
    *paths = next;
}
