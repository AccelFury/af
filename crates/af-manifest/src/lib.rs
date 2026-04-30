// SPDX-License-Identifier: Apache-2.0
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("failed to read manifest `{path}`: {message}")]
    Read { path: PathBuf, message: String },
    #[error("failed to parse manifest `{path}`: {message}")]
    Parse { path: PathBuf, message: String },
    #[error("manifest validation failed")]
    Validation { issues: Vec<ValidationIssue> },
}

impl ManifestError {
    pub fn code(&self) -> &'static str {
        match self {
            ManifestError::Read { .. } => "AF_MANIFEST_READ_FAILED",
            ManifestError::Parse { .. } => "AF_MANIFEST_PARSE_FAILED",
            ManifestError::Validation { .. } => "AF_MANIFEST_INVALID",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            ManifestError::Read { .. } => "Check that the manifest path exists and is readable.",
            ManifestError::Parse { .. } => "Fix the TOML syntax and field types in af-core.toml.",
            ManifestError::Validation { .. } => {
                "Fix the listed manifest issues before running backend commands."
            }
        }
    }

    pub fn exit_code(&self) -> i32 {
        2
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct ValidationIssue {
    pub code: String,
    pub message: String,
    pub hint: String,
}

impl ValidationIssue {
    fn new(code: &str, message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            hint: hint.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq)]
pub struct ManifestValidationReport {
    pub valid: bool,
    pub issues: Vec<ValidationIssue>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq)]
pub struct CoreManifest {
    #[serde(
        default = "default_manifest_version",
        alias = "schema_version",
        alias = "manifest_version"
    )]
    pub af_version: String,
    pub name: String,
    pub vendor: String,
    pub library: String,
    pub core: String,
    pub version: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub metadata: Metadata,
    pub rtl: Rtl,
    #[serde(default)]
    pub sources: SourceSet,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
    #[serde(default)]
    pub ports: Vec<Port>,
    #[serde(default)]
    pub clocks: Vec<Clock>,
    #[serde(default)]
    pub resets: Vec<Reset>,
    #[serde(default)]
    pub interfaces: Vec<Interface>,
    #[serde(default)]
    pub testbenches: Vec<Testbench>,
    #[serde(default)]
    pub vectors: Vec<VectorArtifact>,
    #[serde(default)]
    pub tooling: Option<Tooling>,
    #[serde(default)]
    pub formal: Option<Formal>,
    #[serde(default)]
    pub boards: Vec<String>,
    #[serde(default)]
    pub backend_compatibility: BackendCompatibility,
    #[serde(default)]
    pub known_limitations: Vec<String>,
}

impl CoreManifest {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ManifestError> {
        let path = path.as_ref();
        let raw = std::fs::read_to_string(path).map_err(|err| ManifestError::Read {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?;
        Self::from_toml_str(&raw, path)
    }

    pub fn from_toml_str(raw: &str, origin: impl AsRef<Path>) -> Result<Self, ManifestError> {
        let manifest: Self = toml::from_str(raw).map_err(|err| ManifestError::Parse {
            path: origin.as_ref().to_path_buf(),
            message: err.to_string(),
        })?;
        let report = manifest.validate();
        if report.valid {
            Ok(manifest)
        } else {
            Err(ManifestError::Validation {
                issues: report.issues,
            })
        }
    }

    pub fn unchecked_from_toml_str(
        raw: &str,
        origin: impl AsRef<Path>,
    ) -> Result<Self, ManifestError> {
        toml::from_str(raw).map_err(|err| ManifestError::Parse {
            path: origin.as_ref().to_path_buf(),
            message: err.to_string(),
        })
    }

    pub fn validate(&self) -> ManifestValidationReport {
        let mut issues = Vec::new();

        if !matches!(self.af_version.as_str(), "0.1" | "0.2") {
            issues.push(ValidationIssue::new(
                "AF_MANIFEST_VERSION_UNSUPPORTED",
                format!("unsupported af_version `{}`", self.af_version),
                "Use af_version = \"0.1\" or af_version = \"0.2\".",
            ));
        }

        require_ident("name", &self.name, &mut issues);
        require_ident("vendor", &self.vendor, &mut issues);
        require_ident("library", &self.library, &mut issues);
        require_ident("core", &self.core, &mut issues);
        require_non_empty("version", &self.version, &mut issues);
        require_non_empty("rtl.top", &self.rtl.top, &mut issues);
        if let Some(category) = &self.category {
            require_ident("category", category, &mut issues);
        }

        if !matches!(
            self.rtl.language.as_str(),
            "systemverilog" | "verilog" | "vhdl"
        ) {
            issues.push(ValidationIssue::new(
                "AF_RTL_LANGUAGE_UNSUPPORTED",
                format!("unsupported RTL language `{}`", self.rtl.language),
                "Use one of: systemverilog, verilog, vhdl.",
            ));
        }

        if self.sources.files.is_empty() {
            issues.push(ValidationIssue::new(
                "AF_SOURCES_EMPTY",
                "sources.files must contain at least one RTL source",
                "Add one or more source files relative to the core directory.",
            ));
        }

        for path in self
            .sources
            .files
            .iter()
            .chain(self.sources.include_dirs.iter())
        {
            validate_manifest_path(path, &mut issues);
        }

        for (variant, files) in &self.rtl.variants {
            require_ident("rtl.variants", variant, &mut issues);
            if files.is_empty() {
                issues.push(ValidationIssue::new(
                    "AF_RTL_VARIANT_EMPTY",
                    format!("rtl variant `{variant}` has no files"),
                    "Declare at least one file for each RTL variant or remove the variant.",
                ));
            }
            for file in files {
                validate_manifest_path(file, &mut issues);
            }
        }

        for testbench in &self.testbenches {
            require_ident("testbenches.name", &testbench.name, &mut issues);
            require_non_empty("testbenches.top", &testbench.top, &mut issues);
            if testbench.sources.is_empty() {
                issues.push(ValidationIssue::new(
                    "AF_TESTBENCH_SOURCES_EMPTY",
                    format!("testbench `{}` has no sources", testbench.name),
                    "Declare at least one source file for the testbench or remove the testbench entry.",
                ));
            }
            for source in &testbench.sources {
                validate_manifest_path(source, &mut issues);
            }
        }

        for vector in &self.vectors {
            require_ident("vectors.name", &vector.name, &mut issues);
            require_non_empty("vectors.format", &vector.format, &mut issues);
            validate_manifest_path(&vector.path, &mut issues);
        }

        let clocks: BTreeSet<&str> = self
            .clocks
            .iter()
            .map(|clock| clock.name.as_str())
            .collect();
        let resets: BTreeSet<&str> = self
            .resets
            .iter()
            .map(|reset| reset.name.as_str())
            .collect();

        if let Some(default_clock) = &self.rtl.default_clock {
            if !clocks.contains(default_clock.as_str()) {
                issues.push(ValidationIssue::new(
                    "AF_CLOCK_UNKNOWN",
                    format!("rtl.default_clock references unknown clock `{default_clock}`"),
                    "Add the clock to [[clocks]] or update rtl.default_clock.",
                ));
            }
        }
        if let Some(default_reset) = &self.rtl.default_reset {
            if !resets.contains(default_reset.as_str()) {
                issues.push(ValidationIssue::new(
                    "AF_RESET_UNKNOWN",
                    format!("rtl.default_reset references unknown reset `{default_reset}`"),
                    "Add the reset to [[resets]] or update rtl.default_reset.",
                ));
            }
        }

        for clock in &self.clocks {
            require_ident("clocks.name", &clock.name, &mut issues);
            if matches!(clock.frequency_hz, Some(0)) {
                issues.push(ValidationIssue::new(
                    "AF_CLOCK_FREQUENCY_INVALID",
                    format!("clock `{}` has zero frequency", clock.name),
                    "Use a positive frequency_hz value or omit it if unknown.",
                ));
            }
        }

        for reset in &self.resets {
            require_ident("resets.name", &reset.name, &mut issues);
            if let Some(active) = &reset.active {
                if !matches!(active.as_str(), "high" | "low") {
                    issues.push(ValidationIssue::new(
                        "AF_RESET_ACTIVE_INVALID",
                        format!("reset `{}` has invalid active level `{active}`", reset.name),
                        "Use active = \"high\" or active = \"low\".",
                    ));
                }
            }
        }

        for port in &self.ports {
            require_ident("ports.name", &port.name, &mut issues);
            if !matches!(port.direction.as_str(), "input" | "output" | "inout") {
                issues.push(ValidationIssue::new(
                    "AF_PORT_DIRECTION_INVALID",
                    format!(
                        "port `{}` has invalid direction `{}`",
                        port.name, port.direction
                    ),
                    "Use direction = \"input\", \"output\", or \"inout\".",
                ));
            }
            if matches!(port.width, Some(0)) {
                issues.push(ValidationIssue::new(
                    "AF_PORT_WIDTH_INVALID",
                    format!("port `{}` has invalid zero width", port.name),
                    "Use a positive integer width or omit width for scalar ports.",
                ));
            }
            if let Some(clock) = &port.clock {
                if !clocks.contains(clock.as_str()) {
                    issues.push(ValidationIssue::new(
                        "AF_CLOCK_UNKNOWN",
                        format!("port `{}` references unknown clock `{clock}`", port.name),
                        "Add the clock to [[clocks]] or update the port clock field.",
                    ));
                }
            }
            if let Some(reset) = &port.reset {
                if !resets.contains(reset.as_str()) {
                    issues.push(ValidationIssue::new(
                        "AF_RESET_UNKNOWN",
                        format!("port `{}` references unknown reset `{reset}`", port.name),
                        "Add the reset to [[resets]] or update the port reset field.",
                    ));
                }
            }
        }

        for interface in &self.interfaces {
            require_ident("interfaces.name", &interface.name, &mut issues);
            require_non_empty("interfaces.kind", &interface.kind, &mut issues);
            if let Some(clock) = &interface.clock {
                if !clocks.contains(clock.as_str()) {
                    issues.push(ValidationIssue::new(
                        "AF_CLOCK_UNKNOWN",
                        format!(
                            "interface `{}` references unknown clock `{clock}`",
                            interface.name
                        ),
                        "Add the clock to [[clocks]] or update the interface clock field.",
                    ));
                }
            }
            if let Some(reset) = &interface.reset {
                if !resets.contains(reset.as_str()) {
                    issues.push(ValidationIssue::new(
                        "AF_RESET_UNKNOWN",
                        format!(
                            "interface `{}` references unknown reset `{reset}`",
                            interface.name
                        ),
                        "Add the reset to [[resets]] or update the interface reset field.",
                    ));
                }
            }
        }

        ManifestValidationReport {
            valid: issues.is_empty(),
            issues,
        }
    }

    pub fn vlnv(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.vendor, self.library, self.core, self.version
        )
    }
}

#[derive(Clone, Debug, Default, Deserialize, JsonSchema, Serialize, PartialEq)]
pub struct Metadata {
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq)]
pub struct Rtl {
    pub top: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub default_clock: Option<String>,
    #[serde(default)]
    pub default_reset: Option<String>,
    #[serde(default)]
    pub variants: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Default, Deserialize, JsonSchema, Serialize, PartialEq)]
pub struct SourceSet {
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub include_dirs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq)]
pub struct Port {
    pub name: String,
    pub direction: String,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub clock: Option<String>,
    #[serde(default)]
    pub reset: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct Clock {
    pub name: String,
    #[serde(default)]
    pub frequency_hz: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct Reset {
    pub name: String,
    #[serde(default)]
    pub active: Option<String>,
    #[serde(default)]
    pub asynchronous: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct Interface {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub clock: Option<String>,
    #[serde(default)]
    pub reset: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct Testbench {
    pub name: String,
    pub top: String,
    #[serde(default)]
    pub sources: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct VectorArtifact {
    pub name: String,
    pub format: String,
    pub path: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct Tooling {
    #[serde(default)]
    pub rust: bool,
    #[serde(default)]
    pub typescript_deno: bool,
    #[serde(default)]
    pub python: bool,
    #[serde(default)]
    pub cocotb: bool,
    #[serde(default)]
    pub fusesoc_required: bool,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct Formal {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub properties: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct BackendCompatibility {
    #[serde(default)]
    pub verilator: bool,
    #[serde(default)]
    pub fusesoc: bool,
}

impl Default for BackendCompatibility {
    fn default() -> Self {
        Self {
            verilator: true,
            fusesoc: true,
        }
    }
}

fn default_manifest_version() -> String {
    "0.1".to_string()
}

fn default_language() -> String {
    "systemverilog".to_string()
}

fn require_non_empty(field: &str, value: &str, issues: &mut Vec<ValidationIssue>) {
    if value.trim().is_empty() {
        issues.push(ValidationIssue::new(
            "AF_FIELD_EMPTY",
            format!("{field} must not be empty"),
            "Provide a non-empty value.",
        ));
    }
}

fn require_ident(field: &str, value: &str, issues: &mut Vec<ValidationIssue>) {
    require_non_empty(field, value, issues);
    if !is_identifier_like(value) {
        issues.push(ValidationIssue::new(
            "AF_IDENTIFIER_INVALID",
            format!("{field} `{value}` contains unsupported characters"),
            "Use letters, digits, underscore, dash, or dot; the first character must be a letter or underscore.",
        ));
    }
}

fn is_identifier_like(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

fn validate_manifest_path(path: &str, issues: &mut Vec<ValidationIssue>) {
    if path.trim().is_empty() {
        issues.push(ValidationIssue::new(
            "AF_PATH_EMPTY",
            "manifest path must not be empty",
            "Provide a non-empty path relative to the core directory.",
        ));
        return;
    }
    let parsed = Path::new(path);
    if parsed.is_absolute() {
        issues.push(ValidationIssue::new(
            "AF_PATH_ABSOLUTE",
            format!("absolute path `{path}` is not allowed"),
            "Use a path relative to the core directory.",
        ));
        return;
    }
    for component in parsed.components() {
        match component {
            Component::ParentDir => {
                issues.push(ValidationIssue::new(
                    "AF_PATH_TRAVERSAL",
                    format!("path traversal is not allowed in `{path}`"),
                    "Remove `..` segments from manifest paths.",
                ));
                return;
            }
            Component::Prefix(_) => {
                issues.push(ValidationIssue::new(
                    "AF_PATH_PREFIX",
                    format!("platform prefix is not allowed in `{path}`"),
                    "Use portable relative paths.",
                ));
                return;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_manifest() -> &'static str {
        r#"
af_version = "0.1"
name = "example-core"
vendor = "accelfury"
library = "ip"
core = "example_core"
version = "0.1.0"
known_limitations = ["example limitation"]

[metadata]
license = "Apache-2.0"
authors = ["AccelFury"]
description = "Example core"

[rtl]
top = "example_core"
language = "systemverilog"
default_clock = "clk"
default_reset = "rst_n"

[sources]
files = ["rtl/example_core.sv"]
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

[[testbenches]]
name = "smoke"
top = "tb_example_core"
sources = ["tb/tb_example_core.sv"]
"#
    }

    #[test]
    fn parses_valid_manifest() {
        let manifest = CoreManifest::from_toml_str(valid_manifest(), "af-core.toml").unwrap();
        assert_eq!(manifest.vlnv(), "accelfury:ip:example_core:0.1.0");
        assert!(manifest.validate().valid);
    }

    #[test]
    fn rejects_invalid_port_width() {
        let raw = valid_manifest().replace("width = 1", "width = 0");
        let err = CoreManifest::from_toml_str(&raw, "af-core.toml").unwrap_err();
        let issues = validation_issues(err);
        assert!(issues
            .iter()
            .any(|issue| issue.code == "AF_PORT_WIDTH_INVALID"));
    }

    #[test]
    fn rejects_unknown_clock_domain() {
        let raw = valid_manifest().replace("clock = \"clk\"", "clock = \"other_clk\"");
        let err = CoreManifest::from_toml_str(&raw, "af-core.toml").unwrap_err();
        let issues = validation_issues(err);
        assert!(issues.iter().any(|issue| issue.code == "AF_CLOCK_UNKNOWN"));
    }

    #[test]
    fn rejects_path_traversal() {
        let raw = valid_manifest().replace("rtl/example_core.sv", "../secret.sv");
        let err = CoreManifest::from_toml_str(&raw, "af-core.toml").unwrap_err();
        let issues = validation_issues(err);
        assert!(issues.iter().any(|issue| issue.code == "AF_PATH_TRAVERSAL"));
    }

    fn validation_issues(err: ManifestError) -> Vec<ValidationIssue> {
        match err {
            ManifestError::Validation { issues } => issues,
            other => unreachable!("expected validation error, got {other:?}"),
        }
    }
}
