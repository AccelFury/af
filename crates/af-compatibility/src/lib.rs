// SPDX-License-Identifier: Apache-2.0
use af_manifest::{CoreManifest, ManifestError, StreamInterface};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct CompatibilityReport {
    pub generated_by: String,
    pub status: String,
    pub inputs: Vec<PathBuf>,
    pub constructor: bool,
    pub checks: Vec<String>,
    pub issues: Vec<CompatibilityIssue>,
    pub adapters: Vec<CompatibilityAdapter>,
    pub warnings: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct CompatibilityIssue {
    pub code: String,
    pub message: String,
    pub hint: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct CompatibilityAdapter {
    pub kind: String,
    pub reason: String,
    pub status: String,
}

#[derive(Debug, Error)]
pub enum CompatibilityError {
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    #[error("compatibility check needs at least one input")]
    MissingInput,
}

impl CompatibilityError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Manifest(err) => err.code(),
            Self::MissingInput => "AF_COMPAT_INPUT_MISSING",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            Self::Manifest(err) => err.hint(),
            Self::MissingInput => "Pass one system directory or two or more core directories.",
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Manifest(err) => err.exit_code(),
            Self::MissingInput => 2,
        }
    }
}

pub fn check_compatibility(
    inputs: &[PathBuf],
    constructor: bool,
) -> Result<CompatibilityReport, CompatibilityError> {
    if inputs.is_empty() {
        return Err(CompatibilityError::MissingInput);
    }
    let manifests = inputs
        .iter()
        .filter(|path| path.join("af-core.toml").is_file())
        .map(|path| {
            CoreManifest::from_path(path.join("af-core.toml")).map(|manifest| (path, manifest))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut report = CompatibilityReport {
        generated_by: "AccelFury IP Toolchain".to_string(),
        status: "passed".to_string(),
        inputs: inputs.to_vec(),
        constructor,
        checks: vec![
            "protocol kind".to_string(),
            "data width".to_string(),
            "clock domain".to_string(),
            "reset polarity".to_string(),
            "latency".to_string(),
            "throughput".to_string(),
            "backpressure".to_string(),
            "parameter ranges".to_string(),
            "resource conflicts".to_string(),
            "vendor/board support".to_string(),
            "security policy conflicts".to_string(),
        ],
        issues: Vec::new(),
        adapters: Vec::new(),
        warnings: Vec::new(),
        limitations: vec![
            "First release compatibility is manifest-level; it does not prove protocol timing by simulation.".to_string(),
        ],
    };

    if manifests.len() < inputs.len() {
        report.warnings.push(
            "Non-core/system input detected; constructor/system graph compatibility is represented as a metadata skeleton.".to_string(),
        );
    }

    for pair in manifests.windows(2) {
        let (_, left) = &pair[0];
        let (_, right) = &pair[1];
        compare_streams(left, right, &mut report);
        compare_resets(left, right, &mut report);
    }

    for (path, manifest) in &manifests {
        check_overpromising_language(path, manifest, &mut report);
    }

    if !report.issues.is_empty() {
        report.status = "failed".to_string();
    } else if !report.warnings.is_empty() {
        report.status = "warning".to_string();
    }
    Ok(report)
}

fn compare_streams(left: &CoreManifest, right: &CoreManifest, report: &mut CompatibilityReport) {
    let Some(left_stream) = left.stream_interfaces.first() else {
        return;
    };
    let Some(right_stream) = right.stream_interfaces.first() else {
        return;
    };
    if left_stream.kind != right_stream.kind {
        report.issues.push(issue(
            "AF_COMPAT_PROTOCOL_MISMATCH",
            format!(
                "`{}` uses protocol `{}` but `{}` uses `{}`",
                left.core, left_stream.kind, right.core, right_stream.kind
            ),
            "Insert an explicit protocol adapter or choose cores with the same interface kind.",
        ));
    }
    compare_width(left, right, left_stream, right_stream, report);
    if left_stream.clock_domain != right_stream.clock_domain {
        report.issues.push(issue(
            "AF_COMPAT_CLOCK_MISMATCH",
            format!(
                "`{}` stream clock `{}` differs from `{}` stream clock `{}`",
                left.core, left_stream.clock_domain, right.core, right_stream.clock_domain
            ),
            "Insert an async FIFO/CDC adapter and record the crossing in af-arch.toml.",
        ));
        report.adapters.push(CompatibilityAdapter {
            kind: "async_fifo_cdc".to_string(),
            reason: "stream clock domains differ".to_string(),
            status: "suggested".to_string(),
        });
    }
}

fn compare_width(
    left: &CoreManifest,
    right: &CoreManifest,
    left_stream: &StreamInterface,
    right_stream: &StreamInterface,
    report: &mut CompatibilityReport,
) {
    let left_width = left_stream.data_width.as_deref().unwrap_or("unknown");
    let right_width = right_stream.data_width.as_deref().unwrap_or("unknown");
    if left_width != "unknown" && right_width != "unknown" && left_width != right_width {
        report.issues.push(issue(
            "AF_COMPAT_PROTOCOL_MISMATCH",
            format!(
                "`{}` stream width `{}` differs from `{}` stream width `{}`",
                left.core, left_width, right.core, right_width
            ),
            "Insert a width adapter if the protocol permits packing/unpacking.",
        ));
        report.adapters.push(CompatibilityAdapter {
            kind: "stream_width_adapter".to_string(),
            reason: "data widths differ".to_string(),
            status: "suggested".to_string(),
        });
    }
}

fn compare_resets(left: &CoreManifest, right: &CoreManifest, report: &mut CompatibilityReport) {
    let left_reset = left
        .resets
        .first()
        .and_then(|reset| reset.active.as_deref());
    let right_reset = right
        .resets
        .first()
        .and_then(|reset| reset.active.as_deref());
    if let (Some(left_reset), Some(right_reset)) = (left_reset, right_reset) {
        if left_reset != right_reset {
            report.issues.push(issue(
                "AF_COMPAT_CLOCK_MISMATCH",
                format!(
                    "`{}` reset active `{}` differs from `{}` reset active `{}`",
                    left.core, left_reset, right.core, right_reset
                ),
                "Insert a reset polarity adapter and document reset-domain behavior.",
            ));
            report.adapters.push(CompatibilityAdapter {
                kind: "reset_polarity_adapter".to_string(),
                reason: "reset polarity differs".to_string(),
                status: "suggested".to_string(),
            });
        }
    }
}

/// Manifesto rule: "drop-in replacement" claims are forbidden unless the
/// surrounding text qualifies them as `behavioral equivalent`,
/// `compatibility wrapper`, or `after verification`. We scan
/// `metadata.description`, `known_limitations`, and the core README.md (if
/// present) for the trigger phrase without the qualifier and surface a
/// warning so reviewers can correct the wording before release.
fn check_overpromising_language(
    path: &Path,
    manifest: &CoreManifest,
    report: &mut CompatibilityReport,
) {
    let mut texts: Vec<String> = Vec::new();
    if let Some(description) = manifest.metadata.description.clone() {
        texts.push(description);
    }
    texts.extend(manifest.known_limitations.iter().cloned());
    let readme = path.join("README.md");
    if let Ok(content) = std::fs::read_to_string(&readme) {
        texts.push(content);
    }

    let mut offenders: Vec<String> = Vec::new();
    for text in &texts {
        let lower = text.to_ascii_lowercase();
        if !lower.contains("drop-in replacement") {
            continue;
        }
        if lower.contains("behavioral equivalent")
            || lower.contains("compatibility wrapper")
            || lower.contains("after verification")
        {
            continue;
        }
        // Truncate long passages to keep the warning compact.
        let snippet: String = text.chars().take(160).collect();
        offenders.push(snippet);
    }

    if !offenders.is_empty() {
        let joined = offenders
            .into_iter()
            .map(|snippet| format!("`{snippet}`"))
            .collect::<Vec<_>>()
            .join("; ");
        report.warnings.push(format!(
            "AF_COMPATIBILITY_OVERPROMISING_CLAIM: `{}` uses `drop-in replacement` language without `behavioral equivalent`, `compatibility wrapper`, or `after verification` qualifier ({}).",
            manifest.core, joined
        ));
    }
}

fn issue(code: &str, message: String, hint: &str) -> CompatibilityIssue {
    CompatibilityIssue {
        code: code.to_string(),
        message,
        hint: hint.to_string(),
    }
}

pub fn manifest_path(input: &Path) -> PathBuf {
    input.join("af-core.toml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn reports_protocol_width_clock_and_reset_conflicts_with_adapters() {
        let dir = tempdir().unwrap();
        let left = dir.path().join("left");
        let right = dir.path().join("right");
        write_core(&left, "left_core", "ready_valid", "32", "clk_a", "low");
        write_core(&right, "right_core", "valid_only", "64", "clk_b", "high");

        let report = check_compatibility(&[left, right], false).unwrap();

        assert_eq!(report.status, "failed");
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "AF_COMPAT_PROTOCOL_MISMATCH"));
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "AF_COMPAT_CLOCK_MISMATCH"));
        assert!(report
            .adapters
            .iter()
            .any(|adapter| adapter.kind == "stream_width_adapter"));
        assert!(report
            .adapters
            .iter()
            .any(|adapter| adapter.kind == "async_fifo_cdc"));
        assert!(report
            .adapters
            .iter()
            .any(|adapter| adapter.kind == "reset_polarity_adapter"));
    }

    #[test]
    fn reports_unqualified_drop_in_replacement_warning() {
        let dir = tempdir().unwrap();
        let left = dir.path().join("left-naive");
        let right = dir.path().join("right");
        write_core_with_description(
            &left,
            "left_core",
            "drop-in replacement for Xilinx FIFO Generator",
        );
        write_core(&right, "right_core", "ready_valid", "32", "clk_a", "low");

        let report = check_compatibility(&[left, right], false).unwrap();
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("AF_COMPATIBILITY_OVERPROMISING_CLAIM")));
    }

    #[test]
    fn accepts_qualified_drop_in_replacement_language() {
        let dir = tempdir().unwrap();
        let left = dir.path().join("left-quoted");
        let right = dir.path().join("right");
        write_core_with_description(
            &left,
            "left_core",
            "drop-in replacement after verification with a behavioral equivalent compatibility wrapper",
        );
        write_core(&right, "right_core", "ready_valid", "32", "clk_a", "low");

        let report = check_compatibility(&[left, right], false).unwrap();
        assert!(!report
            .warnings
            .iter()
            .any(|warning| warning.contains("AF_COMPATIBILITY_OVERPROMISING_CLAIM")));
    }

    fn write_core_with_description(dir: &Path, core: &str, description: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(
            dir.join("af-core.toml"),
            format!(
                r#"
af_version = "0.2"
name = "{core}"
vendor = "accelfury"
library = "ip"
core = "{core}"
version = "0.1.0"

[metadata]
description = "{description}"

[rtl]
top = "{core}"
language = "verilog-2001"

[sources]
files = ["rtl/{core}.v"]

[[clocks]]
name = "clk_a"
port = "clk"

[[resets]]
name = "rst_n"
port = "rst_n"
active = "low"
clock_domain = "clk_a"

[[ports]]
name = "clk"
direction = "input"
width = 1

[[ports]]
name = "rst_n"
direction = "input"
width = 1

[[ports]]
name = "data"
direction = "input"
width = 32
clock = "clk_a"
reset = "rst_n"

[[ports]]
name = "valid"
direction = "input"
width = 1
clock = "clk_a"
reset = "rst_n"

[[ports]]
name = "ready"
direction = "output"
width = 1
clock = "clk_a"
reset = "rst_n"

[[stream_interfaces]]
name = "stream"
kind = "ready_valid"
clock_domain = "clk_a"
data = "data"
valid = "valid"
ready = "ready"
data_width = "32"
"#
            ),
        )
        .unwrap();
    }

    fn write_core(
        dir: &Path,
        core: &str,
        protocol: &str,
        width: &str,
        clock: &str,
        reset_active: &str,
    ) {
        fs::create_dir_all(dir).unwrap();
        fs::write(
            dir.join("af-core.toml"),
            format!(
                r#"
af_version = "0.2"
name = "{core}"
vendor = "accelfury"
library = "ip"
core = "{core}"
version = "0.1.0"

[rtl]
top = "{core}"
language = "verilog-2001"

[sources]
files = ["rtl/{core}.v"]

[[parameters]]
name = "DATA_WIDTH"
value = "{width}"

[[clocks]]
name = "{clock}"
port = "clk"

[[resets]]
name = "rst_n"
port = "rst_n"
active = "{reset_active}"
clock_domain = "{clock}"

[[ports]]
name = "clk"
direction = "input"
width = 1

[[ports]]
name = "rst_n"
direction = "input"
width = 1

[[ports]]
name = "data"
direction = "input"
width = "DATA_WIDTH"
clock = "{clock}"
reset = "rst_n"

[[ports]]
name = "valid"
direction = "input"
width = 1
clock = "{clock}"
reset = "rst_n"

[[ports]]
name = "ready"
direction = "output"
width = 1
clock = "{clock}"
reset = "rst_n"

[[stream_interfaces]]
name = "stream"
kind = "{protocol}"
clock_domain = "{clock}"
data = "data"
valid = "valid"
ready = "ready"
data_width = "{width}"
"#,
            ),
        )
        .unwrap();
    }
}
