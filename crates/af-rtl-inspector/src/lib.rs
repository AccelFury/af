// SPDX-License-Identifier: Apache-2.0
use af_manifest::{CoreManifest, PortWidth};
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
            check_manifest_port_widths(&mut report, manifest, header);
            check_clock_reset_bindings(&mut report, manifest);
            check_portable_port_style(&mut report, manifest, header);
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
    if !is_portable_verilog_language(manifest) {
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

    for keyword in [
        "logic",
        "interface",
        "modport",
        "package",
        "import",
        "typedef",
        "enum",
        "struct",
        "class",
        "program",
        "clocking",
        "property",
        "sequence",
        "always_ff",
        "always_comb",
        "always_latch",
    ] {
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
    // Hard PHY blocks (DDR/PCIe/MIPI/SerDes) must be flagged before the
    // generic vendor-marker loop. They are not portable RTL at any
    // portability level — manifesto U4 (replacement is not reasonable;
    // only abstraction/wrapper/mock applies).
    for marker in [
        // DDR PHY
        "ddr_phy",
        "ddrphy",
        "lpddr",
        "ddr3",
        "ddr4",
        "ddr5",
        "mig_",
        "phy_ddr",
        // PCIe hard IP
        "pcie_phy",
        "pcie3",
        "pcie4",
        "pcie5",
        "xpcs",
        // MIPI
        "mipi_dphy",
        "mipi_csi",
        "mipi_dsi",
        "dphy",
        "cphy",
        // SerDes hard IP (high-speed transceiver primitives)
        "gtx_",
        "gty_",
        "gth_",
        "gtp_",
        "serdes",
        "xceiver",
        "lvds_serdes",
    ] {
        if contains_portability_marker(&lower, marker) {
            failed = true;
            report.issues.push(RtlIssue::error(
                "AF_PORTABLE_HARD_PHY_BLOCK",
                format!(
                    "portable Verilog source contains hard-PHY marker `{marker}` — PHY/hard-IP blocks are not portable RTL"
                ),
                "Hard PHY blocks (DDR/PCIe/MIPI/SerDes) cannot be reimplemented as portable RTL. They are interface/wrapper/mock material only (manifesto U4). Move to vendor/<vendor>/ as a thin wrapper around the vendor primitive and reclassify the core as complex-vendor-aware with portability_level = U3 or U4.",
            ));
        }
    }

    for marker in [
        "xpm_",
        "ramb",
        "fifo_generator",
        "fifo18",
        "fifo36",
        "fdre",
        "oddr",
        "iddr",
        "altsyncram",
        "scfifo",
        "dcfifo",
        "lpm_",
        "altera_",
        "intel_",
        "altpll",
        "mmcm",
        "dcm",
        "clk_wiz",
        "clock_wizard",
        "_pll",
        "pll_",
        "rpll",
        "epll",
        "dpll",
        "clkdiv",
        "bufg",
        "bufio",
        "gowin_",
        "spx9",
        "dpx9",
        "sdpx9",
        "ram16sdp",
    ] {
        if contains_portability_marker(&lower, marker) {
            failed = true;
            report.issues.push(RtlIssue::error(
                "AF_PORTABLE_VENDOR_OR_CLOCK_MARKER",
                format!("portable Verilog source contains forbidden marker `{marker}`"),
                "Keep vendor primitives, hard macros, PLLs, clock dividers, and board-specific adaptation outside the generic core.",
            ));
        }
    }

    for marker in [
        "axi", "axi_", "_axi", "s_axi", "m_axi", "axis_", "awvalid", "awready", "awaddr", "wvalid",
        "wready", "wdata", "wstrb", "bvalid", "bready", "arvalid", "arready", "araddr", "rvalid",
        "rready", "rdata", "tvalid", "tready", "tdata", "tlast", "tkeep", "tstrb",
    ] {
        if contains_axi_marker(&lower, marker) {
            failed = true;
            report.issues.push(RtlIssue::error(
                "AF_PORTABLE_AXI_ONLY_MARKER",
                format!("portable Verilog source contains AXI-specific marker `{marker}`"),
                "Keep AXI adaptation in an optional wrapper around portable core ports.",
            ));
        }
    }

    if has_unguarded_initial(&stripped, source_text) {
        failed = true;
        report.issues.push(RtlIssue::error(
            "AF_PORTABLE_IMPLICIT_RESET",
            "portable Verilog source uses `initial` outside a simulation guard",
            "Drive synthesizable resets through the declared reset port. Wrap `initial` blocks in `synthesis translate_off`/`translate_on`, `\\`ifndef SYNTHESIS`, or `\\`ifdef SIMULATION`.",
        ));
    }

    if let Some(marker) = first_encrypted_pragma(&stripped) {
        failed = true;
        report.issues.push(RtlIssue::error(
            "AF_PORTABLE_ENCRYPTED_NETLIST",
            format!("portable Verilog source contains encrypted-IP marker `{marker}`"),
            "Encrypted netlist envelopes (pragma protect, `protect begin_protected`) are not portable. Reimplement the core in open RTL or move it to a vendor backend.",
        ));
    }

    for source in &manifest.sources.files {
        let lower_path = source.to_ascii_lowercase();
        for extension in [".edn", ".dcp", ".xci", ".qsys", ".ipx", ".qxp", ".sdc"] {
            if lower_path.ends_with(extension) {
                failed = true;
                report.issues.push(RtlIssue::error(
                    "AF_PORTABLE_ENCRYPTED_NETLIST",
                    format!(
                        "portable core source `{source}` uses vendor-only extension `{extension}`"
                    ),
                    "Vendor netlists, constraints, and IP envelopes must live in a vendor backend; portable cores ship only open RTL.",
                ));
                break;
            }
        }
    }

    report.checks.insert(
        "portable_verilog_policy".to_string(),
        if failed { "fail" } else { "pass" }.to_string(),
    );
}

fn has_unguarded_initial(stripped: &str, raw: &str) -> bool {
    // Guards live inside comments and preprocessor lines, so check the raw
    // source (before comment stripping). The check is intentionally lenient:
    // any recognized guard disables the rule rather than producing false
    // positives without a real Verilog parser.
    let lower_raw = raw.to_ascii_lowercase();
    if lower_raw.contains("synthesis translate_off")
        || lower_raw.contains("`ifndef synthesis")
        || lower_raw.contains("`ifdef simulation")
        || lower_raw.contains("`ifndef formal")
    {
        return false;
    }
    contains_identifier(stripped, "initial")
}

fn first_encrypted_pragma(stripped: &str) -> Option<&'static str> {
    let lower = stripped.to_ascii_lowercase();
    [
        "pragma protect",
        "protect begin_protected",
        "protect end_protected",
        "pragma protect_begin",
    ]
    .into_iter()
    .find(|marker| lower.contains(marker))
}

fn check_portable_port_style(
    report: &mut RtlInspectionReport,
    manifest: &CoreManifest,
    header: &str,
) {
    if !is_portable_verilog_language(manifest) {
        report
            .checks
            .insert("portable_port_style".to_string(), "skip".to_string());
        return;
    }

    let Some(declarations) = module_port_declarations(header) else {
        report
            .checks
            .insert("portable_port_style".to_string(), "fail".to_string());
        report.issues.push(RtlIssue::error(
            "AF_PORTABLE_PORT_STYLE",
            "portable Verilog top module port list could not be parsed",
            "Use a Verilog-2001 ANSI module header with one explicit direction and net type per port.",
        ));
        return;
    };

    let mut failed = false;
    for port in &manifest.ports {
        let Some(declaration) = declarations
            .iter()
            .find(|declaration| contains_identifier(declaration, &port.name))
        else {
            continue;
        };

        if !has_explicit_port_style(declaration, &port.direction) {
            failed = true;
            report.issues.push(RtlIssue::error(
                "AF_PORTABLE_PORT_STYLE",
                format!(
                    "portable Verilog port `{}` must declare direction and wire/reg type explicitly",
                    port.name
                ),
                "Use one declaration per port, for example `input wire clk` or `output reg done`.",
            ));
        }
    }

    report.checks.insert(
        "portable_port_style".to_string(),
        if failed { "fail" } else { "pass" }.to_string(),
    );
}

fn is_portable_verilog_language(manifest: &CoreManifest) -> bool {
    matches!(manifest.rtl.language.as_str(), "verilog" | "verilog-2001")
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

fn check_manifest_port_widths(
    report: &mut RtlInspectionReport,
    manifest: &CoreManifest,
    header: &str,
) {
    let Some(declarations) = module_port_declarations(header) else {
        report
            .checks
            .insert("port_widths_match".to_string(), "skip".to_string());
        return;
    };

    let mut failed = false;
    for port in &manifest.ports {
        let Some(manifest_width) = &port.width else {
            continue;
        };
        let Some(declaration) = declarations
            .iter()
            .find(|declaration| declaration_declares_port(declaration, &port.name))
        else {
            continue;
        };
        let rtl_width = rtl_width_from_declaration(declaration, manifest);
        if !width_contract_matches(manifest_width, &rtl_width, manifest) {
            failed = true;
            report.issues.push(RtlIssue::error(
                "AF_PORT_WIDTH_MISMATCH",
                format!(
                    "manifest port `{}` width `{}` does not match RTL declaration width `{}`",
                    port.name,
                    manifest_width_text(manifest_width),
                    rtl_width.display
                ),
                "Update [[ports]].width to the RTL parameter expression or integer width; do not collapse parameterized buses to width = 1.",
            ));
        }
    }

    report.checks.insert(
        "port_widths_match".to_string(),
        if failed { "fail" } else { "pass" }.to_string(),
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

fn contains_portability_marker(source_text: &str, marker: &str) -> bool {
    source_text.contains(marker)
}

fn contains_axi_marker(source_text: &str, marker: &str) -> bool {
    if marker.contains('_') {
        source_text.contains(marker)
    } else {
        contains_identifier(source_text, marker)
    }
}

fn module_port_declarations(header: &str) -> Option<Vec<String>> {
    let end = header.rfind(");")?;
    let before_end = &header[..end];
    let start = before_end.rfind('(')?;
    let declarations = before_end[start + 1..]
        .split(',')
        .map(str::trim)
        .filter(|declaration| !declaration.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    Some(declarations)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RtlWidth {
    display: String,
    normalized_expr: Option<String>,
    default_bits: Option<u64>,
}

fn rtl_width_from_declaration(declaration: &str, manifest: &CoreManifest) -> RtlWidth {
    let Some(start) = declaration.find('[') else {
        return RtlWidth {
            display: "1".to_string(),
            normalized_expr: Some("1".to_string()),
            default_bits: Some(1),
        };
    };
    let Some(end_offset) = declaration[start + 1..].find(']') else {
        return RtlWidth {
            display: "unknown".to_string(),
            normalized_expr: None,
            default_bits: None,
        };
    };
    let raw = declaration[start + 1..start + 1 + end_offset].trim();
    let display = format!("[{raw}]");
    let Some((msb, lsb)) = raw.split_once(':') else {
        return RtlWidth {
            display,
            normalized_expr: None,
            default_bits: None,
        };
    };
    let msb = msb.trim();
    let lsb = lsb.trim();
    if let (Ok(msb_int), Ok(lsb_int)) = (msb.parse::<i64>(), lsb.parse::<i64>()) {
        let width = (msb_int - lsb_int).unsigned_abs() + 1;
        return RtlWidth {
            display,
            normalized_expr: Some(width.to_string()),
            default_bits: Some(width),
        };
    }
    if lsb == "0" {
        if let Some(param) = msb.strip_suffix("-1").map(str::trim) {
            let expr = normalize_width_expr(param);
            return RtlWidth {
                display,
                normalized_expr: Some(expr.clone()),
                default_bits: eval_width_expr(&expr, manifest),
            };
        }
        if is_identifier(msb) {
            let expr = normalize_width_expr(&format!("{msb}+1"));
            return RtlWidth {
                display,
                normalized_expr: Some(expr.clone()),
                default_bits: eval_width_expr(&expr, manifest),
            };
        }
    }
    RtlWidth {
        display,
        normalized_expr: Some(normalize_width_expr(raw)),
        default_bits: eval_width_expr(raw, manifest),
    }
}

fn width_contract_matches(
    manifest_width: &PortWidth,
    rtl_width: &RtlWidth,
    manifest: &CoreManifest,
) -> bool {
    match manifest_width {
        PortWidth::Integer(value) => {
            rtl_width.default_bits == Some(*value as u64)
                || rtl_width.normalized_expr.as_deref() == Some(&value.to_string())
        }
        PortWidth::Parameter(expr) => {
            let normalized = normalize_width_expr(expr);
            rtl_width.normalized_expr.as_deref() == Some(normalized.as_str())
                || eval_width_expr(&normalized, manifest) == rtl_width.default_bits
        }
    }
}

fn manifest_width_text(width: &PortWidth) -> String {
    match width {
        PortWidth::Integer(value) => value.to_string(),
        PortWidth::Parameter(expr) => expr.clone(),
    }
}

fn declaration_declares_port(declaration: &str, port_name: &str) -> bool {
    declaration
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
        .filter(|token| !token.is_empty())
        .last()
        == Some(port_name)
}

fn normalize_width_expr(expr: &str) -> String {
    expr.chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect()
}

fn eval_width_expr(expr: &str, manifest: &CoreManifest) -> Option<u64> {
    let expr = normalize_width_expr(expr);
    if let Ok(value) = expr.parse::<u64>() {
        return Some(value);
    }
    let mut total: i128 = 0;
    let mut sign: i128 = 1;
    let mut token = String::new();
    let mut saw_operand = false;
    for ch in expr.chars().chain(std::iter::once('+')) {
        if matches!(ch, '+' | '-') {
            if token.is_empty() {
                return None;
            }
            let value = parameter_or_integer_value(&token, manifest)? as i128;
            total += sign * value;
            saw_operand = true;
            token.clear();
            sign = if ch == '-' { -1 } else { 1 };
            continue;
        }
        if ch.is_ascii_alphanumeric() || ch == '_' {
            token.push(ch);
            continue;
        }
        return None;
    }
    if saw_operand && total >= 0 {
        Some(total as u64)
    } else {
        None
    }
}

fn parameter_or_integer_value(token: &str, manifest: &CoreManifest) -> Option<u64> {
    if let Ok(value) = token.parse::<u64>() {
        return Some(value);
    }
    manifest
        .parameters
        .iter()
        .find(|parameter| parameter.name == token)
        .and_then(|parameter| parameter.value.parse::<u64>().ok())
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn has_explicit_port_style(declaration: &str, direction: &str) -> bool {
    let declaration = declaration.to_ascii_lowercase();
    let direction = direction.to_ascii_lowercase();
    let has_direction = contains_identifier(&declaration, &direction);
    let has_net_type = match direction.as_str() {
        "input" | "inout" => contains_identifier(&declaration, "wire"),
        "output" => {
            contains_identifier(&declaration, "wire") || contains_identifier(&declaration, "reg")
        }
        _ => false,
    };
    has_direction && has_net_type
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
language = "systemverilog"

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

[[clocks]]
name = "clk"
port = "clk"

[[resets]]
name = "test_reset"
port = "clk"
active = "high"

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
    fn reports_manifest_width_mismatch_for_parameterized_bus_collapsed_to_one() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(
            dir.path().join("rtl/demo.v"),
            r#"`default_nettype none
module demo #(
  parameter DATA_BITS = 32
) (
  input wire clk,
  input wire [DATA_BITS-1:0] data
);
endmodule
`default_nettype wire
"#,
        )
        .unwrap();
        let manifest = CoreManifest::from_toml_str(
            r#"
af_version = "0.2"
name = "demo"
vendor = "accelfury"
library = "ip"
core = "demo"
version = "0.1.0"

[rtl]
top = "demo"
language = "verilog-2001"

[sources]
files = ["rtl/demo.v"]

[[parameters]]
name = "DATA_BITS"
value = "32"

[[clocks]]
name = "clk"
port = "clk"

[[resets]]
name = "rst"
port = "clk"

[[ports]]
name = "clk"
direction = "input"
width = 1

[[ports]]
name = "data"
direction = "input"
width = 1
"#,
            "af-core.toml",
        )
        .unwrap();
        let report = inspect_core(dir.path(), &manifest).unwrap();
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "AF_PORT_WIDTH_MISMATCH"));
    }

    #[test]
    fn accepts_parameter_and_simple_expression_widths() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(
            dir.path().join("rtl/demo.v"),
            r#"`default_nettype none
module demo #(
  parameter DATA_BITS = 32,
  parameter FIFO_ADDR_BITS = 4
) (
  input wire clk,
  input wire [DATA_BITS-1:0] data,
  output wire [FIFO_ADDR_BITS:0] level
);
endmodule
`default_nettype wire
"#,
        )
        .unwrap();
        let manifest = CoreManifest::from_toml_str(
            r#"
af_version = "0.2"
name = "demo"
vendor = "accelfury"
library = "ip"
core = "demo"
version = "0.1.0"

[rtl]
top = "demo"
language = "verilog-2001"

[sources]
files = ["rtl/demo.v"]

[[parameters]]
name = "DATA_BITS"
value = "32"

[[parameters]]
name = "FIFO_ADDR_BITS"
value = "4"

[[clocks]]
name = "clk"
port = "clk"

[[resets]]
name = "rst"
port = "clk"

[[ports]]
name = "clk"
direction = "input"
width = 1

[[ports]]
name = "data"
direction = "input"
width = "DATA_BITS"

[[ports]]
name = "level"
direction = "output"
width = "FIFO_ADDR_BITS + 1"
"#,
            "af-core.toml",
        )
        .unwrap();
        let report = inspect_core(dir.path(), &manifest).unwrap();
        assert!(
            !report
                .issues
                .iter()
                .any(|issue| issue.code == "AF_PORT_WIDTH_MISMATCH"),
            "{:#?}",
            report.issues
        );
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
    fn verilog_policy_rejects_vendor_axi_and_pll_markers() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(
            dir.path().join("rtl/demo.v"),
            r#"`default_nettype none
module demo (
  input wire clk,
  input wire enable,
  output wire done
);
  wire ramb18e1_data;
  wire s_axi_awvalid;
  wire u_pll_locked;
  assign done = enable & ramb18e1_data & s_axi_awvalid & u_pll_locked;
endmodule
`default_nettype wire
"#,
        )
        .unwrap();
        let report = inspect_core(dir.path(), &manifest_with_ports("demo")).unwrap();
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "AF_PORTABLE_VENDOR_OR_CLOCK_MARKER"));
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "AF_PORTABLE_AXI_ONLY_MARKER"));
    }

    #[test]
    fn verilog_policy_rejects_unguarded_initial() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(
            dir.path().join("rtl/demo.v"),
            r#"`default_nettype none
module demo (
  input wire clk,
  input wire enable,
  output reg done
);
  initial begin
    done = 1'b0;
  end
  always @(posedge clk) begin
    if (enable) done <= 1'b1;
  end
endmodule
`default_nettype wire
"#,
        )
        .unwrap();
        let report = inspect_core(dir.path(), &manifest_with_ports("demo")).unwrap();
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "AF_PORTABLE_IMPLICIT_RESET"));
    }

    #[test]
    fn verilog_policy_allows_guarded_initial() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(
            dir.path().join("rtl/demo.v"),
            r#"`default_nettype none
module demo (
  input wire clk,
  input wire enable,
  output reg done
);
  // synthesis translate_off
  initial begin
    done = 1'b0;
  end
  // synthesis translate_on
  always @(posedge clk) begin
    if (enable) done <= 1'b1;
  end
endmodule
`default_nettype wire
"#,
        )
        .unwrap();
        let report = inspect_core(dir.path(), &manifest_with_ports("demo")).unwrap();
        assert!(!report
            .issues
            .iter()
            .any(|issue| issue.code == "AF_PORTABLE_IMPLICIT_RESET"));
    }

    #[test]
    fn verilog_policy_rejects_pragma_protect() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(
            dir.path().join("rtl/demo.v"),
            r#"`default_nettype none
`pragma protect begin_protected
module demo (
  input wire clk,
  input wire enable,
  output wire done
);
  assign done = enable;
endmodule
`pragma protect end_protected
`default_nettype wire
"#,
        )
        .unwrap();
        let report = inspect_core(dir.path(), &manifest_with_ports("demo")).unwrap();
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "AF_PORTABLE_ENCRYPTED_NETLIST"));
    }

    #[test]
    fn verilog_policy_rejects_ddr_phy_marker() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(
            dir.path().join("rtl/demo.v"),
            r#"`default_nettype none
module demo (
  input wire clk,
  input wire enable,
  output wire done
);
  wire ddr4_calib_done;
  wire phy_ddr_ready;
  assign done = enable & ddr4_calib_done & phy_ddr_ready;
endmodule
`default_nettype wire
"#,
        )
        .unwrap();
        let report = inspect_core(dir.path(), &manifest_with_ports("demo")).unwrap();
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "AF_PORTABLE_HARD_PHY_BLOCK"
                && issue.message.contains("ddr4")));
    }

    #[test]
    fn verilog_policy_rejects_pcie_serdes_marker() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(
            dir.path().join("rtl/demo.v"),
            r#"`default_nettype none
module demo (
  input wire clk,
  input wire enable,
  output wire done
);
  wire gty_locked;
  wire serdes_rx_valid;
  assign done = enable & gty_locked & serdes_rx_valid;
endmodule
`default_nettype wire
"#,
        )
        .unwrap();
        let report = inspect_core(dir.path(), &manifest_with_ports("demo")).unwrap();
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "AF_PORTABLE_HARD_PHY_BLOCK"
                && (issue.message.contains("gty_") || issue.message.contains("serdes"))));
    }

    #[test]
    fn verilog_policy_rejects_mipi_dphy_marker() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(
            dir.path().join("rtl/demo.v"),
            r#"`default_nettype none
module demo (
  input wire clk,
  input wire enable,
  output wire done
);
  wire mipi_dphy_lock;
  wire mipi_csi_valid;
  assign done = enable & mipi_dphy_lock & mipi_csi_valid;
endmodule
`default_nettype wire
"#,
        )
        .unwrap();
        let report = inspect_core(dir.path(), &manifest_with_ports("demo")).unwrap();
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "AF_PORTABLE_HARD_PHY_BLOCK"
                && (issue.message.contains("mipi_dphy") || issue.message.contains("mipi_csi"))));
    }

    #[test]
    fn verilog_policy_rejects_vendor_netlist_extensions() {
        let manifest_with_dcp = CoreManifest::from_toml_str(
            r#"
af_version = "0.2"
name = "demo"
vendor = "accelfury"
library = "ip"
core = "demo"
version = "0.1.0"

[rtl]
top = "demo"
language = "verilog"

[sources]
files = ["rtl/demo.v", "rtl/blackbox.dcp"]

[[clocks]]
name = "clk"
port = "clk"

[[resets]]
name = "test_reset"
port = "clk"
active = "high"

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
"#,
            "af-core.toml",
        )
        .unwrap();
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(
            dir.path().join("rtl/demo.v"),
            "`default_nettype none\nmodule demo(input wire clk, input wire enable, output wire done); assign done = enable; endmodule\n`default_nettype wire\n",
        )
        .unwrap();
        fs::write(dir.path().join("rtl/blackbox.dcp"), "<binary>\n").unwrap();
        let report = inspect_core(dir.path(), &manifest_with_dcp).unwrap();
        assert!(report.issues.iter().any(|issue| {
            issue.code == "AF_PORTABLE_ENCRYPTED_NETLIST" && issue.message.contains(".dcp")
        }));
    }

    #[test]
    fn verilog_policy_rejects_implicit_port_style() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(
            dir.path().join("rtl/demo.v"),
            r#"`default_nettype none
module demo (
  input clk,
  input wire enable,
  output done
);
endmodule
`default_nettype wire
"#,
        )
        .unwrap();
        let report = inspect_core(dir.path(), &manifest_with_ports("demo")).unwrap();
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "AF_PORTABLE_PORT_STYLE"));
    }

    #[test]
    fn verilog_policy_allows_parameter_generate_and_inferred_ram() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(
            dir.path().join("rtl/demo.v"),
            r#"`default_nettype none
module demo
#(
  parameter WIDTH = 8,
  parameter DEPTH = 16
)
(
  input wire clk,
  input wire enable,
  output reg done
);
  reg [WIDTH-1:0] mem [0:DEPTH-1];

  generate
    if (DEPTH > 0) begin : g_has_storage
      always @(posedge clk) begin
        if (enable) begin
          done <= mem[0][0];
        end
      end
    end
  endgenerate
endmodule
`default_nettype wire
"#,
        )
        .unwrap();
        let report = inspect_core(dir.path(), &manifest_with_ports("demo")).unwrap();
        assert!(!report.has_errors(), "{:#?}", report.issues);
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
