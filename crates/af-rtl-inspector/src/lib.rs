// SPDX-License-Identifier: Apache-2.0
use af_manifest::CoreManifest;
use af_security::{safe_join, SecurityError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RtlInspectorError {
    #[error(transparent)]
    Security(#[from] SecurityError),
    #[error("failed to read RTL file `{path}`: {message}")]
    Read { path: PathBuf, message: String },
}

impl RtlInspectorError {
    pub fn code(&self) -> &'static str {
        match self {
            RtlInspectorError::Security(err) => err.code(),
            RtlInspectorError::Read { .. } => "AF_RTL_READ_FAILED",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            RtlInspectorError::Security(err) => err.hint(),
            RtlInspectorError::Read { .. } => "Check that the declared RTL file is readable.",
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            RtlInspectorError::Security(err) => err.exit_code(),
            RtlInspectorError::Read { .. } => 2,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum RtlIssueSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RtlIssue {
    pub severity: RtlIssueSeverity,
    pub code: String,
    pub message: String,
    pub hint: String,
}

impl RtlIssue {
    fn error(code: &str, message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            severity: RtlIssueSeverity::Error,
            code: code.to_string(),
            message: message.into(),
            hint: hint.into(),
        }
    }

    fn warning(code: &str, message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            severity: RtlIssueSeverity::Warning,
            code: code.to_string(),
            message: message.into(),
            hint: hint.into(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct RtlInspectionReport {
    pub scanned_files: Vec<PathBuf>,
    #[serde(default)]
    pub checks: BTreeMap<String, String>,
    pub issues: Vec<RtlIssue>,
}

impl RtlInspectionReport {
    pub fn has_errors(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == RtlIssueSeverity::Error)
    }

    pub fn warnings(&self) -> Vec<String> {
        self.issues
            .iter()
            .filter(|issue| issue.severity == RtlIssueSeverity::Warning)
            .map(|issue| issue.message.clone())
            .collect()
    }
}

pub fn inspect_core(
    core_dir: impl AsRef<Path>,
    manifest: &CoreManifest,
) -> Result<RtlInspectionReport, RtlInspectorError> {
    let core_dir = core_dir.as_ref();
    let mut report = RtlInspectionReport::default();
    let mut source_text = String::new();

    for include_dir in &manifest.sources.include_dirs {
        let path = safe_join(core_dir, include_dir)?;
        if !path.is_dir() {
            report.issues.push(RtlIssue::warning(
                "AF_INCLUDE_DIR_MISSING",
                format!("include directory `{include_dir}` does not exist"),
                "Create the include directory or remove it from sources.include_dirs.",
            ));
        }
    }

    for source in &manifest.sources.files {
        let path = safe_join(core_dir, source)?;
        if !path.is_file() {
            report.issues.push(RtlIssue::error(
                "AF_SOURCE_MISSING",
                format!("source file `{source}` does not exist"),
                "Create the file or update sources.files in af-core.toml.",
            ));
            continue;
        }
        report.scanned_files.push(path.clone());
        let text = fs::read_to_string(&path).map_err(|err| RtlInspectorError::Read {
            path,
            message: err.to_string(),
        })?;
        source_text.push_str(&text);
        source_text.push('\n');
    }

    for testbench in &manifest.testbenches {
        for source in &testbench.sources {
            let path = safe_join(core_dir, source)?;
            if !path.is_file() {
                report.issues.push(RtlIssue::error(
                    "AF_TESTBENCH_SOURCE_MISSING",
                    format!(
                        "testbench `{}` source file `{source}` does not exist",
                        testbench.name
                    ),
                    "Create the file or update the testbench sources list.",
                ));
            }
        }
        for source in &testbench.rtl_sources {
            let path = safe_join(core_dir, source)?;
            if !path.is_file() {
                report.issues.push(RtlIssue::error(
                    "AF_TESTBENCH_RTL_SOURCE_MISSING",
                    format!(
                        "testbench `{}` rtl source file `{source}` does not exist",
                        testbench.name
                    ),
                    "Create the file or update the testbench rtl_sources list.",
                ));
            }
        }
    }

    if !source_text.is_empty() {
        let top_header = top_declaration_header(&source_text, manifest);
        if top_header.is_none() {
            report
                .checks
                .insert("top_module_presence".to_string(), "fail".to_string());
            report.issues.push(RtlIssue::error(
                "AF_TOP_MISSING",
                format!(
                    "top `{}` was not found in declared RTL sources",
                    manifest.rtl.top
                ),
                "Ensure rtl.top matches a module/entity declared in sources.files.",
            ));
        } else {
            report
                .checks
                .insert("top_module_presence".to_string(), "pass".to_string());
        }

        if let Some(header) = top_header.as_deref() {
            check_manifest_ports_in_header(&mut report, manifest, header);
            check_clock_reset_bindings(&mut report, manifest);
        }
        check_portable_verilog_policy(&mut report, manifest, &source_text);
    }

    Ok(report)
}

fn check_portable_verilog_policy(
    report: &mut RtlInspectionReport,
    manifest: &CoreManifest,
    source_text: &str,
) {
    if !matches!(manifest.rtl.language.as_str(), "verilog" | "verilog-2001") {
        report
            .checks
            .insert("portable_verilog_policy".to_string(), "skip".to_string());
        return;
    }

    let stripped = strip_comments(source_text);
    let mut failed = false;

    if !stripped.contains("`default_nettype none") {
        failed = true;
        report.issues.push(RtlIssue::error(
            "AF_PORTABLE_DEFAULT_NETTYPE_MISSING",
            "Verilog source does not declare `default_nettype none`",
            "Add `default_nettype none` around portable Verilog RTL to prevent implicit nets.",
        ));
    }

    for keyword in ["logic", "interface", "package", "always_ff", "always_comb"] {
        if contains_identifier(&stripped, keyword) {
            failed = true;
            report.issues.push(RtlIssue::error(
                "AF_PORTABLE_SYSTEMVERILOG_CONSTRUCT",
                format!("portable Verilog source contains SystemVerilog construct `{keyword}`"),
                "Move SystemVerilog constructs to wrappers or rewrite the base RTL as Verilog-2001.",
            ));
        }
    }

    let lower = stripped.to_ascii_lowercase();
    for marker in [
        "xpm_",
        "ramb",
        "fifo18",
        "fifo36",
        "altsyncram",
        "scfifo",
        "lpm_",
        "altpll",
        "mmcm",
        "pll",
        "clkdiv",
        "bufg",
        "gowin_",
    ] {
        if lower.contains(marker) {
            failed = true;
            report.issues.push(RtlIssue::error(
                "AF_PORTABLE_VENDOR_OR_CLOCK_MARKER",
                format!("portable Verilog source contains forbidden marker `{marker}`"),
                "Keep vendor primitives, hard macros, PLLs, clock dividers, and board-specific adaptation outside the generic core.",
            ));
        }
    }

    if contains_identifier(&lower, "axi") {
        failed = true;
        report.issues.push(RtlIssue::error(
            "AF_PORTABLE_AXI_ONLY_MARKER",
            "portable Verilog source contains AXI-specific marker `axi`",
            "Keep AXI adaptation in an optional wrapper around portable core ports.",
        ));
    }

    report.checks.insert(
        "portable_verilog_policy".to_string(),
        if failed { "fail" } else { "pass" }.to_string(),
    );
}

fn top_declaration_header(source_text: &str, manifest: &CoreManifest) -> Option<String> {
    match manifest.rtl.language.as_str() {
        "vhdl" => {
            if contains_token_sequence(source_text, "entity", &manifest.rtl.top) {
                Some(source_text.to_string())
            } else {
                None
            }
        }
        _ => module_header(source_text, &manifest.rtl.top),
    }
}

fn check_manifest_ports_in_header(
    report: &mut RtlInspectionReport,
    manifest: &CoreManifest,
    header: &str,
) {
    if manifest.ports.is_empty() {
        report
            .checks
            .insert("ports_manifest_match".to_string(), "skip".to_string());
        return;
    }
    let mut missing = Vec::new();
    for port in &manifest.ports {
        if !contains_identifier(header, &port.name) {
            missing.push(port.name.clone());
            report.issues.push(RtlIssue::error(
                "AF_PORT_MISSING",
                format!(
                    "manifest port `{}` was not found in top `{}` declaration",
                    port.name, manifest.rtl.top
                ),
                "Update [[ports]] or the top module declaration so they agree.",
            ));
        }
    }
    report.checks.insert(
        "ports_manifest_match".to_string(),
        if missing.is_empty() { "pass" } else { "fail" }.to_string(),
    );
}

fn check_clock_reset_bindings(report: &mut RtlInspectionReport, manifest: &CoreManifest) {
    let port_names = manifest
        .ports
        .iter()
        .map(|port| port.name.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let mut failed = false;

    for clock in &manifest.clocks {
        let bound_port = clock.port.as_deref().unwrap_or(clock.name.as_str());
        if !port_names.contains(bound_port) {
            failed = true;
            report.issues.push(RtlIssue::error(
                "AF_CLOCK_PORT_UNBOUND",
                format!(
                    "clock `{}` is not bound to a declared port `{bound_port}`",
                    clock.name
                ),
                "Set clocks.port or add a matching clock port to [[ports]].",
            ));
        }
    }
    for reset in &manifest.resets {
        let bound_port = reset.port.as_deref().unwrap_or(reset.name.as_str());
        if !port_names.contains(bound_port) {
            failed = true;
            report.issues.push(RtlIssue::error(
                "AF_RESET_PORT_UNBOUND",
                format!(
                    "reset `{}` is not bound to a declared port `{bound_port}`",
                    reset.name
                ),
                "Set resets.port or add a matching reset port to [[ports]].",
            ));
        }
    }

    report.checks.insert(
        "clock_reset_policy".to_string(),
        if failed { "fail" } else { "pass" }.to_string(),
    );
}

fn module_header(source_text: &str, top: &str) -> Option<String> {
    let stripped = strip_line_comments(source_text);
    let module_pattern = format!("module {top}");
    let start = stripped.find(&module_pattern)?;
    let rest = &stripped[start..];
    let end = rest.find(");").map(|idx| idx + 2).unwrap_or(rest.len());
    Some(rest[..end.min(rest.len())].to_string())
}

fn strip_line_comments(source_text: &str) -> String {
    source_text
        .lines()
        .map(|line| line.split_once("//").map(|(code, _)| code).unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_comments(source_text: &str) -> String {
    let line_stripped = strip_line_comments(source_text);
    let mut out = String::with_capacity(line_stripped.len());
    let mut chars = line_stripped.chars().peekable();
    let mut in_block = false;
    while let Some(ch) = chars.next() {
        if in_block {
            if ch == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block = false;
            }
            continue;
        }
        if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            in_block = true;
            continue;
        }
        out.push(ch);
    }
    out
}

fn contains_identifier(source_text: &str, ident: &str) -> bool {
    source_text
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
        .any(|token| token == ident)
}

fn contains_token_sequence(source_text: &str, first: &str, second: &str) -> bool {
    let mut previous = "";
    for token in source_text
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
        .filter(|token| !token.is_empty())
    {
        if previous == first && token == second {
            return true;
        }
        previous = token;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn manifest(top: &str) -> CoreManifest {
        CoreManifest::from_toml_str(
            &format!(
                r#"
af_version = "0.1"
name = "demo"
vendor = "accelfury"
library = "ip"
core = "demo"
version = "0.1.0"

[rtl]
top = "{top}"

[sources]
files = ["rtl/demo.sv"]

[[clocks]]
name = "clk"

[[resets]]
name = "rst_n"

[[ports]]
name = "clk"
direction = "input"
width = 1

[[ports]]
name = "rst_n"
direction = "input"
width = 1
"#
            ),
            "af-core.toml",
        )
        .unwrap()
    }

    fn manifest_with_ports(top: &str) -> CoreManifest {
        CoreManifest::from_toml_str(
            &format!(
                r#"
af_version = "0.2"
name = "demo"
vendor = "accelfury"
library = "ip"
core = "demo"
version = "0.1.0"

[rtl]
top = "{top}"
language = "verilog"

[sources]
files = ["rtl/demo.v"]

[[ports]]
name = "clk"
direction = "input"
width = 1

[[ports]]
name = "enable"
direction = "input"
width = 1

[[ports]]
name = "done"
direction = "output"
width = 1
"#
            ),
            "af-core.toml",
        )
        .unwrap()
    }

    #[test]
    fn finds_declared_top() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(
            dir.path().join("rtl/demo.sv"),
            "module demo(input logic clk, input logic rst_n); endmodule\n",
        )
        .unwrap();
        let report = inspect_core(dir.path(), &manifest("demo")).unwrap();
        assert!(!report.has_errors());
    }

    #[test]
    fn reports_missing_source() {
        let dir = tempdir().unwrap();
        let report = inspect_core(dir.path(), &manifest("demo")).unwrap();
        assert!(report.has_errors());
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "AF_SOURCE_MISSING"));
    }

    #[test]
    fn reports_missing_top() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(dir.path().join("rtl/demo.sv"), "module other; endmodule\n").unwrap();
        let report = inspect_core(dir.path(), &manifest("demo")).unwrap();
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "AF_TOP_MISSING"));
    }

    #[test]
    fn matches_ports_after_leading_comment_and_net_types() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(
            dir.path().join("rtl/demo.v"),
            r#"// SPDX-License-Identifier: Apache-2.0
`default_nettype none

module demo (
  input wire clk,
  input wire enable,
  output reg done
);
endmodule

`default_nettype wire
"#,
        )
        .unwrap();
        let report = inspect_core(dir.path(), &manifest_with_ports("demo")).unwrap();
        assert!(!report.has_errors(), "{:#?}", report.issues);
    }

    #[test]
    fn verilog_policy_requires_default_nettype_none() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(
            dir.path().join("rtl/demo.v"),
            "module demo(input wire clk, input wire enable, output wire done); endmodule\n",
        )
        .unwrap();
        let report = inspect_core(dir.path(), &manifest_with_ports("demo")).unwrap();
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "AF_PORTABLE_DEFAULT_NETTYPE_MISSING"));
    }

    #[test]
    fn verilog_policy_rejects_systemverilog_constructs() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(
            dir.path().join("rtl/demo.v"),
            r#"`default_nettype none
module demo (
  input logic clk,
  input logic enable,
  output logic done
);
  always_ff @(posedge clk) begin
  end
endmodule
`default_nettype wire
"#,
        )
        .unwrap();
        let report = inspect_core(dir.path(), &manifest_with_ports("demo")).unwrap();
        assert!(report.issues.iter().any(|issue| {
            issue.code == "AF_PORTABLE_SYSTEMVERILOG_CONSTRUCT"
                && issue.message.contains("always_ff")
        }));
    }

    #[test]
    fn reports_manifest_port_missing_from_header() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(
            dir.path().join("rtl/demo.sv"),
            "module demo(input logic clk); endmodule\n",
        )
        .unwrap();
        let report = inspect_core(dir.path(), &manifest("demo")).unwrap();
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "AF_PORT_MISSING"));
    }
}
