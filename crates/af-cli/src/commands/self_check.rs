// SPDX-License-Identifier: Apache-2.0
//
// `af self check` handler and the helpers it needs to load
// `af-selfcheck.toml` and resolve optional targets.

use crate::{read_toml_file, write_json_file, CliError, CliOutput};
use af_core::{check_core, CoreError};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize)]
struct SelfCheckConfig {
    #[serde(default)]
    targets: Vec<SelfCheckTarget>,
}

#[derive(Debug, Deserialize, Serialize)]
struct SelfCheckTarget {
    name: String,
    path: PathBuf,
    #[serde(default)]
    path_env: Option<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default = "default_self_check_required")]
    required: bool,
    #[serde(default = "default_self_check_checks")]
    checks: Vec<String>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Serialize)]
struct SelfCheckReport {
    schema_version: &'static str,
    kind: &'static str,
    status: String,
    config: PathBuf,
    reports: Vec<PathBuf>,
    targets: Vec<SelfCheckTargetResult>,
    warnings: Vec<String>,
    failures: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SelfCheckTargetResult {
    name: String,
    path: PathBuf,
    path_env: Option<String>,
    source: Option<String>,
    required: bool,
    checks: Vec<String>,
    status: String,
    core: Option<String>,
    scanned_files: Vec<PathBuf>,
    warnings: Vec<String>,
    message: Option<String>,
}

fn default_self_check_required() -> bool {
    true
}

fn default_self_check_checks() -> Vec<String> {
    vec!["core-check".to_string()]
}

pub fn self_check(
    config_path: &Path,
    include_optional: bool,
    target_filter: &[String],
    build_root: &Path,
) -> Result<CliOutput, CliError> {
    let config: SelfCheckConfig = read_toml_file(config_path)?;
    if config.targets.is_empty() {
        return Err(CliError::new(
            "AF_SELF_CHECK_TARGETS_EMPTY",
            format!(
                "self-check config `{}` has no targets",
                config_path.display()
            ),
            "Add at least one [[targets]] entry with name, path, and checks.",
            2,
        ));
    }

    let config_base = config_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let requested: std::collections::BTreeSet<String> =
        target_filter.iter().map(|item| item.to_string()).collect();
    let mut results = Vec::new();
    let mut warnings = Vec::new();
    let mut failures = Vec::new();

    for target in &config.targets {
        if !requested.is_empty() && !requested.contains(&target.name) {
            continue;
        }
        let path = resolve_self_check_path(config_base, target);
        let should_check = target.required
            || include_optional
            || !requested.is_empty()
            || path.join("af-core.toml").is_file();
        if !should_check {
            results.push(SelfCheckTargetResult {
                name: target.name.clone(),
                path,
                path_env: target.path_env.clone(),
                source: target.source.clone(),
                required: target.required,
                checks: target.checks.clone(),
                status: "skipped".to_string(),
                core: None,
                scanned_files: Vec::new(),
                warnings: vec!["optional target is not present locally".to_string()],
                message: Some(
                    "optional target skipped; pass --include-optional to report missing optional paths"
                        .to_string(),
                ),
            });
            continue;
        }

        let result = run_self_check_target(target, &path);
        if result.status == "failed" {
            let message = result
                .message
                .clone()
                .unwrap_or_else(|| "target self-check failed".to_string());
            if target.required {
                failures.push(format!("{}: {message}", target.name));
            } else {
                warnings.push(format!("optional target {}: {message}", target.name));
            }
        } else if result.status == "skipped" {
            warnings.push(format!("target {} skipped", target.name));
        }
        results.push(result);
    }

    if !requested.is_empty() && results.is_empty() {
        return Err(CliError::new(
            "AF_SELF_CHECK_TARGET_UNKNOWN",
            format!("no self-check targets matched {:?}", target_filter),
            "Use target names from af-selfcheck.toml.",
            2,
        ));
    }

    let status = if failures.is_empty() {
        if warnings.is_empty() {
            "passed"
        } else {
            "warning"
        }
    } else {
        "failed"
    };
    let report_path = build_root.join("reports/self-check.json");
    let report = SelfCheckReport {
        schema_version: "0.1",
        kind: "accelfury.self_check",
        status: status.to_string(),
        config: config_path.to_path_buf(),
        reports: vec![report_path.clone()],
        targets: results,
        warnings,
        failures,
    };
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            CliError::new(
                "AF_SELF_CHECK_REPORT_DIR_FAILED",
                format!(
                    "failed to create self-check report directory `{}`: {err}",
                    parent.display()
                ),
                "Check build-root permissions or choose a writable --build-root.",
                5,
            )
        })?;
    }
    write_json_file(&report_path, &report)?;

    if status == "failed" {
        return Err(CliError::new(
            "AF_SELF_CHECK_FAILED",
            "one or more required self-check targets failed",
            "Inspect .af-build/reports/self-check.json and fix the failing target.",
            7,
        )
        .with_details(&report));
    }

    Ok(CliOutput {
        human: format!(
            "self check {status}: {} targets (report: {})",
            report.targets.len(),
            report_path.display()
        ),
        json: json!(report),
    })
}

fn resolve_self_check_path(config_base: &Path, target: &SelfCheckTarget) -> PathBuf {
    if let Some(path_env) = &target.path_env {
        if let Some(value) = std::env::var_os(path_env) {
            if !value.is_empty() {
                return PathBuf::from(value);
            }
        }
    }
    if target.path.is_absolute() {
        target.path.clone()
    } else {
        config_base.join(&target.path)
    }
}

fn run_self_check_target(target: &SelfCheckTarget, path: &Path) -> SelfCheckTargetResult {
    let warnings = Vec::new();
    if !target
        .checks
        .iter()
        .any(|check| check == "core-check" || check == "core_check")
    {
        return SelfCheckTargetResult {
            name: target.name.clone(),
            path: path.to_path_buf(),
            path_env: target.path_env.clone(),
            source: target.source.clone(),
            required: target.required,
            checks: target.checks.clone(),
            status: "failed".to_string(),
            core: None,
            scanned_files: Vec::new(),
            warnings,
            message: Some("self-check target declares no supported checks".to_string()),
        };
    }

    match check_core(path) {
        Ok(report) => SelfCheckTargetResult {
            name: target.name.clone(),
            path: path.to_path_buf(),
            path_env: target.path_env.clone(),
            source: target.source.clone(),
            required: target.required,
            checks: target.checks.clone(),
            status: "passed".to_string(),
            core: Some(report.manifest.vlnv()),
            scanned_files: report.inspection.scanned_files,
            warnings: report.warnings,
            message: None,
        },
        Err(CoreError::CheckFailed { report }) => SelfCheckTargetResult {
            name: target.name.clone(),
            path: path.to_path_buf(),
            path_env: target.path_env.clone(),
            source: target.source.clone(),
            required: target.required,
            checks: target.checks.clone(),
            status: "failed".to_string(),
            core: Some(report.manifest.vlnv()),
            scanned_files: report.inspection.scanned_files.clone(),
            warnings: report.warnings.clone(),
            message: Some("core check failed".to_string()),
        },
        Err(err) => SelfCheckTargetResult {
            name: target.name.clone(),
            path: path.to_path_buf(),
            path_env: target.path_env.clone(),
            source: target.source.clone(),
            required: target.required,
            checks: target.checks.clone(),
            status: "failed".to_string(),
            core: None,
            scanned_files: Vec::new(),
            warnings,
            message: Some(err.to_string()),
        },
    }
}
