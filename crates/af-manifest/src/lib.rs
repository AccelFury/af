// SPDX-License-Identifier: Apache-2.0
pub use af_complexity::PortabilityLevel;
use af_complexity::ProjectClass;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use thiserror::Error;
use toml::map::Map;
use toml::Value;

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
            ManifestError::Parse { .. } => {
                "Fix TOML syntax and schema shape. Required v0.2/v0.3 fields include af_version, name, vendor, library, core, version, [rtl], [sources], clocks, resets, ports, and relative source paths; use `af core new` for a valid scaffold or migrate legacy project metadata."
            }
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
    pub kind: Option<String>,
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
    pub stream_interfaces: Vec<StreamInterface>,
    #[serde(default)]
    pub csr: Option<PresenceBlock>,
    #[serde(default)]
    pub interrupts: Option<PresenceBlock>,
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
    pub complexity: Option<Complexity>,
    #[serde(default)]
    pub architecture: Option<Architecture>,
    #[serde(default)]
    pub reuse: Option<Reuse>,
    #[serde(default)]
    pub dependencies: Dependencies,
    #[serde(default)]
    pub resources: Resources,
    #[serde(default)]
    pub backend_variants: Vec<BackendVariant>,
    #[serde(default)]
    pub constructor: Option<Constructor>,
    #[serde(default)]
    pub known_limitations: Vec<String>,
    #[serde(default)]
    pub portability_level: Option<PortabilityLevel>,
    #[serde(default)]
    pub priority: Option<CorePriority>,
    #[serde(default)]
    pub maturity: Option<CoreMaturity>,
    #[serde(default)]
    pub verification_required: Vec<VerificationGate>,
    #[serde(default)]
    pub evidence: Option<EvidenceDeclaration>,
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
        let mut value: Value = toml::from_str(raw).map_err(|err| ManifestError::Parse {
            path: origin.as_ref().to_path_buf(),
            message: err.to_string(),
        })?;
        normalize_manifest_value(&mut value);
        let mut issues = Vec::new();
        validate_v02_required_shape(&value, &mut issues);
        if !issues.is_empty() {
            return Err(ManifestError::Validation { issues });
        }
        let manifest: Self =
            value
                .try_into()
                .map_err(|err: toml::de::Error| ManifestError::Parse {
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
        let mut value: Value = toml::from_str(raw).map_err(|err| ManifestError::Parse {
            path: origin.as_ref().to_path_buf(),
            message: err.to_string(),
        })?;
        normalize_manifest_value(&mut value);
        value
            .try_into()
            .map_err(|err: toml::de::Error| ManifestError::Parse {
                path: origin.as_ref().to_path_buf(),
                message: err.to_string(),
            })
    }

    pub fn validate(&self) -> ManifestValidationReport {
        let mut issues = Vec::new();

        if !matches!(self.af_version.as_str(), "0.1" | "0.2" | "0.3") {
            issues.push(ValidationIssue::new(
                "AF_MANIFEST_VERSION_UNSUPPORTED",
                format!("unsupported af_version `{}`", self.af_version),
                "Use af_version = \"0.1\", af_version = \"0.2\", or af_version = \"0.3\".",
            ));
        }
        if let Some(kind) = &self.kind {
            if kind != "accelfury.core" {
                issues.push(ValidationIssue::new(
                    "AF_MANIFEST_KIND_INVALID",
                    format!("unsupported manifest kind `{kind}`"),
                    "Use kind = \"accelfury.core\" for core manifests.",
                ));
            }
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
            "systemverilog" | "verilog" | "verilog-2001" | "vhdl"
        ) {
            issues.push(ValidationIssue::new(
                "AF_RTL_LANGUAGE_UNSUPPORTED",
                format!("unsupported RTL language `{}`", self.rtl.language),
                "Use one of: systemverilog, verilog, verilog-2001, vhdl.",
            ));
        }

        if self.sources.files.is_empty() {
            issues.push(ValidationIssue::new(
                "AF_SOURCES_EMPTY",
                "sources.files must contain at least one RTL source",
                "Add one or more source files relative to the core directory.",
            ));
        }
        if matches!(self.af_version.as_str(), "0.2" | "0.3") && self.clocks.is_empty() {
            issues.push(ValidationIssue::new(
                "AF_MANIFEST_V02_REQUIRED_ARRAY_EMPTY",
                "v0.2/v0.3 manifest requires at least one [[clocks]] entry",
                "Add one clock entry under [[clocks]].",
            ));
        }
        if matches!(self.af_version.as_str(), "0.2" | "0.3") && self.resets.is_empty() {
            issues.push(ValidationIssue::new(
                "AF_MANIFEST_V02_REQUIRED_ARRAY_EMPTY",
                "v0.2/v0.3 manifest requires at least one [[resets]] entry",
                "Add one reset entry under [[resets]].",
            ));
        }
        if matches!(self.af_version.as_str(), "0.2" | "0.3") && self.ports.is_empty() {
            issues.push(ValidationIssue::new(
                "AF_MANIFEST_V02_REQUIRED_ARRAY_EMPTY",
                "v0.2/v0.3 manifest requires at least one [[ports]] entry",
                "Add one port entry under [[ports]].",
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
        let source_files: BTreeSet<&str> = self.sources.files.iter().map(String::as_str).collect();
        for (path, role) in &self.sources.roles {
            if !source_files.contains(path.as_str()) {
                issues.push(ValidationIssue::new(
                    "AF_SOURCE_ROLE_ORPHANED",
                    format!("source role is declared for unknown file `{path}`"),
                    "Keep source roles aligned with source file paths.",
                ));
            }
            if !matches!(
                role.as_str(),
                "rtl" | "generated" | "testbench" | "constraint"
            ) {
                issues.push(ValidationIssue::new(
                    "AF_SOURCE_ROLE_INVALID",
                    format!("source `{path}` has unsupported role `{role}`"),
                    "Use role = \"rtl\" or role = \"generated\" for core RTL sources.",
                ));
            }
        }
        for source in &self.sources.files {
            let role = self
                .sources
                .roles
                .get(source)
                .map(String::as_str)
                .unwrap_or("rtl");
            if looks_generated_path(source) && role != "generated" {
                issues.push(ValidationIssue::new(
                    "AF_GENERATED_SOURCE_ROLE_REQUIRED",
                    format!("generated-looking source `{source}` is listed without role = \"generated\""),
                    "Generated sources must be explicitly marked as generated; handwritten RTL remains the source of hardware logic.",
                ));
            }
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
            for source in &testbench.rtl_sources {
                validate_manifest_path(source, &mut issues);
            }
        }

        for vector in &self.vectors {
            require_ident("vectors.name", &vector.name, &mut issues);
            require_non_empty("vectors.format", &vector.format, &mut issues);
            validate_manifest_path(&vector.path, &mut issues);
        }

        if let Some(complexity) = &self.complexity {
            if complexity.score > 20 {
                issues.push(ValidationIssue::new(
                    "AF_COMPLEXITY_SCORE_INVALID",
                    format!(
                        "complexity.score `{}` is outside the supported range",
                        complexity.score
                    ),
                    "Use a deterministic score from 0 to 20.",
                ));
            }
            require_non_empty("complexity.decision", &complexity.decision, &mut issues);
            if complexity.triggers.is_empty() && complexity.class != ProjectClass::SimplePortable {
                issues.push(ValidationIssue::new(
                    "AF_COMPLEXITY_TRIGGER_MISSING",
                    "non-simple complexity class requires at least one trigger",
                    "Add the decision triggers that justify the selected project class.",
                ));
            }
        }

        for dependency in &self.dependencies.cores {
            require_ident("dependencies.cores.name", &dependency.name, &mut issues);
            require_non_empty(
                "dependencies.cores.version",
                &dependency.version,
                &mut issues,
            );
            require_non_empty("dependencies.cores.role", &dependency.role, &mut issues);
        }

        for memory in &self.resources.memory {
            require_ident("resources.memory.name", &memory.name, &mut issues);
            require_non_empty("resources.memory.kind", &memory.kind, &mut issues);
            validate_backend_policy(&memory.backend_policy, &mut issues);
            if memory.width == 0 || memory.depth == 0 {
                issues.push(ValidationIssue::new(
                    "AF_RESOURCE_CONTRACT_INVALID",
                    format!("memory resource `{}` has zero width or depth", memory.name),
                    "Set positive width and depth values for each memory contract.",
                ));
            }
        }

        for dsp in &self.resources.dsp {
            require_ident("resources.dsp.name", &dsp.name, &mut issues);
            validate_backend_policy(&dsp.backend_policy, &mut issues);
            if dsp.count == 0 {
                issues.push(ValidationIssue::new(
                    "AF_RESOURCE_CONTRACT_INVALID",
                    format!("DSP resource `{}` has zero count", dsp.name),
                    "Set a positive count for each DSP contract.",
                ));
            }
        }

        for variant in &self.backend_variants {
            require_ident("backend_variants.name", &variant.name, &mut issues);
            require_ident("backend_variants.vendor", &variant.vendor, &mut issues);
            if variant.families.is_empty() {
                issues.push(ValidationIssue::new(
                    "AF_BACKEND_VARIANT_FAMILY_MISSING",
                    format!("backend variant `{}` has no families", variant.name),
                    "Add at least one target FPGA family or remove the variant.",
                ));
            }
            if !matches!(
                variant.status.as_str(),
                "supported" | "planned" | "unsupported"
            ) {
                issues.push(ValidationIssue::new(
                    "AF_BACKEND_VARIANT_STATUS_INVALID",
                    format!(
                        "backend variant `{}` has unsupported status `{}`",
                        variant.name, variant.status
                    ),
                    "Use status = \"supported\", \"planned\", or \"unsupported\".",
                ));
            }
            if matches!(variant.status.as_str(), "planned" | "unsupported")
                && self.known_limitations.is_empty()
            {
                issues.push(ValidationIssue::new(
                    "AF_BACKEND_LIMITATION_MISSING",
                    format!(
                        "backend variant `{}` is `{}` without a known limitation",
                        variant.name, variant.status
                    ),
                    "Add known_limitations so downstream users see unsupported backend boundaries.",
                ));
            }
        }

        if let Some(constructor) = &self.constructor {
            if constructor.export
                && (constructor.category.as_deref().is_none_or(str::is_empty)
                    || constructor
                        .compatibility_profile
                        .as_deref()
                        .is_none_or(str::is_empty))
            {
                issues.push(ValidationIssue::new(
                    "AF_CONSTRUCTOR_EXPORT_INCOMPLETE",
                    "constructor export requires category and compatibility_profile",
                    "Set [constructor].category and [constructor].compatibility_profile.",
                ));
            }
        }

        for parameter in &self.parameters {
            require_ident("parameters.name", &parameter.name, &mut issues);
            if parameter.value.trim().is_empty() {
                issues.push(ValidationIssue::new(
                    "AF_PARAMETER_DEFAULT_EMPTY",
                    format!("parameter `{}` has no default value", parameter.name),
                    "Set value/default for each parameter so generated wrappers are deterministic.",
                ));
            }
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
        let port_names: BTreeSet<&str> = self.ports.iter().map(|port| port.name.as_str()).collect();
        let parameter_names: BTreeSet<&str> = self
            .parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect();
        let clock_ports: BTreeSet<&str> = self
            .clocks
            .iter()
            .filter_map(|clock| clock.port.as_deref())
            .collect();
        let reset_ports: BTreeSet<&str> = self
            .resets
            .iter()
            .filter_map(|reset| reset.port.as_deref())
            .collect();

        if let Some(default_clock) = &self.rtl.default_clock {
            if !clocks.contains(default_clock.as_str())
                && !clock_ports.contains(default_clock.as_str())
            {
                issues.push(ValidationIssue::new(
                    "AF_CLOCK_UNKNOWN",
                    format!("rtl.default_clock references unknown clock or clock port `{default_clock}`"),
                    "Add the clock to [[clocks]], set clocks.port, or update rtl.default_clock.",
                ));
            }
        }
        if let Some(default_reset) = &self.rtl.default_reset {
            if !resets.contains(default_reset.as_str())
                && !reset_ports.contains(default_reset.as_str())
            {
                issues.push(ValidationIssue::new(
                    "AF_RESET_UNKNOWN",
                    format!("rtl.default_reset references unknown reset or reset port `{default_reset}`"),
                    "Add the reset to [[resets]], set resets.port, or update rtl.default_reset.",
                ));
            }
        }

        for clock in &self.clocks {
            require_ident("clocks.name", &clock.name, &mut issues);
            if let Some(port) = &clock.port {
                if !port_names.contains(port.as_str()) {
                    issues.push(ValidationIssue::new(
                        "AF_CLOCK_PORT_UNKNOWN",
                        format!("clock `{}` references unknown port `{port}`", clock.name),
                        "Add the clock port to [[ports]] or update clocks.port.",
                    ));
                }
            }
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
            if let Some(port) = &reset.port {
                if !port_names.contains(port.as_str()) {
                    issues.push(ValidationIssue::new(
                        "AF_RESET_PORT_UNKNOWN",
                        format!("reset `{}` references unknown port `{port}`", reset.name),
                        "Add the reset port to [[ports]] or update resets.port.",
                    ));
                }
            }
            if let Some(clock_domain) = &reset.clock_domain {
                if !clocks.contains(clock_domain.as_str()) {
                    issues.push(ValidationIssue::new(
                        "AF_CLOCK_DOMAIN_UNKNOWN",
                        format!(
                            "reset `{}` references unknown clock domain `{clock_domain}`",
                            reset.name
                        ),
                        "Add the clock domain to [[clocks]] or update resets.clock_domain.",
                    ));
                }
            }
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
            if let Some(width) = &port.width {
                match width {
                    PortWidth::Integer(0) => issues.push(ValidationIssue::new(
                        "AF_PORT_WIDTH_INVALID",
                        format!("port `{}` has invalid zero width", port.name),
                        "Use a positive integer width or omit width for scalar ports.",
                    )),
                    PortWidth::Parameter(parameter) => {
                        if !parameter_names.contains(parameter.as_str()) {
                            issues.push(ValidationIssue::new(
                                "AF_PORT_WIDTH_PARAMETER_UNKNOWN",
                                format!(
                                    "port `{}` width references unknown parameter `{parameter}`",
                                    port.name
                                ),
                                "Add the parameter to [[parameters]] or use an integer width.",
                            ));
                        }
                    }
                    PortWidth::Integer(_) => {}
                }
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
            if let Some(clock_domain) = &port.clock_domain {
                if !clocks.contains(clock_domain.as_str()) {
                    issues.push(ValidationIssue::new(
                        "AF_CLOCK_DOMAIN_UNKNOWN",
                        format!(
                            "port `{}` references unknown clock domain `{clock_domain}`",
                            port.name
                        ),
                        "Add the clock domain to [[clocks]] or update the port clock_domain field.",
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
            if let Some(kind) = &port.kind {
                require_non_empty("ports.kind", kind, &mut issues);
            }
        }

        for interface in &self.interfaces {
            require_ident("interfaces.name", &interface.name, &mut issues);
            require_non_empty("interfaces.kind", &interface.kind, &mut issues);
            if let Some(clock) = &interface.clock {
                if !clocks.contains(clock.as_str()) && !port_names.contains(clock.as_str()) {
                    issues.push(ValidationIssue::new(
                        "AF_CLOCK_UNKNOWN",
                        format!(
                            "interface `{}` references unknown clock or port `{clock}`",
                            interface.name
                        ),
                        "Add the clock to [[clocks]], add the port to [[ports]], or update the interface clock field.",
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
        for gate in &self.verification_required {
            if let Some(evidence) = &gate.evidence {
                validate_manifest_path(evidence, &mut issues);
            }
        }

        if let Some(ev) = &self.evidence {
            if let Some(ci) = &ev.docker_ci_cd {
                require_non_empty("evidence.docker_ci_cd.run_url", &ci.run_url, &mut issues);
                require_non_empty(
                    "evidence.docker_ci_cd.commit_sha",
                    &ci.commit_sha,
                    &mut issues,
                );
                require_non_empty(
                    "evidence.docker_ci_cd.sha256sums",
                    &ci.sha256sums,
                    &mut issues,
                );
                if !matches!(ci.conclusion.as_str(), "success" | "failure" | "cancelled") {
                    issues.push(ValidationIssue::new(
                        "AF_EVIDENCE_CONCLUSION_INVALID",
                        format!(
                            "evidence.docker_ci_cd.conclusion `{}` is unsupported",
                            ci.conclusion
                        ),
                        "Use conclusion = \"success\", \"failure\", or \"cancelled\".",
                    ));
                }
            }
            if let Some(vt) = &ev.vendor_tool {
                if !matches!(
                    vt.tool.as_str(),
                    "vivado" | "quartus" | "gowin" | "efinity" | "libero" | "radiant" | "diamond"
                ) {
                    issues.push(ValidationIssue::new(
                        "AF_EVIDENCE_VENDOR_TOOL_INVALID",
                        format!(
                            "evidence.vendor_tool.tool `{}` is unsupported",
                            vt.tool
                        ),
                        "Use a known vendor tool name (vivado, quartus, gowin, efinity, libero, radiant, diamond).",
                    ));
                }
                validate_manifest_path(&vt.report_path, &mut issues);
                require_non_empty(
                    "evidence.vendor_tool.conclusion",
                    &vt.conclusion,
                    &mut issues,
                );
            }
            if let Some(bh) = &ev.board_hardware {
                require_non_empty(
                    "evidence.board_hardware.board_id",
                    &bh.board_id,
                    &mut issues,
                );
                validate_manifest_path(&bh.report_path, &mut issues);
                require_non_empty("evidence.board_hardware.date", &bh.date, &mut issues);
            }
        }

        for interface in &self.stream_interfaces {
            require_ident("stream_interfaces.name", &interface.name, &mut issues);
            require_non_empty("stream_interfaces.kind", &interface.kind, &mut issues);
            if !clocks.contains(interface.clock_domain.as_str()) {
                issues.push(ValidationIssue::new(
                    "AF_CLOCK_DOMAIN_UNKNOWN",
                    format!(
                        "stream interface `{}` references unknown clock domain `{}`",
                        interface.name, interface.clock_domain
                    ),
                    "Add the clock domain to [[clocks]] or update stream_interfaces.clock_domain.",
                ));
            }
            for (field, port) in [
                ("data", &interface.data),
                ("valid", &interface.valid),
                ("ready", &interface.ready),
            ] {
                if !port_names.contains(port.as_str()) {
                    issues.push(ValidationIssue::new(
                        "AF_INTERFACE_PORT_UNKNOWN",
                        format!(
                            "stream interface `{}` {field} references unknown port `{port}`",
                            interface.name
                        ),
                        "Add the referenced port to [[ports]] or update the stream interface.",
                    ));
                }
            }
            if let Some(width) = &interface.data_width {
                if !parameter_names.contains(width.as_str()) && width.parse::<u32>().is_err() {
                    issues.push(ValidationIssue::new(
                        "AF_INTERFACE_WIDTH_INVALID",
                        format!(
                            "stream interface `{}` data_width `{width}` is not an integer or parameter",
                            interface.name
                        ),
                        "Use an integer or a [[parameters]].name value.",
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
    pub display_name: Option<String>,
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
    pub systemverilog_subset: Option<bool>,
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
    #[serde(default)]
    pub roles: BTreeMap<String, String>,
    #[serde(default)]
    pub file_types: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct PresenceBlock {
    #[serde(default)]
    pub present: bool,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq)]
pub struct Parameter {
    pub name: String,
    #[serde(default, alias = "default")]
    pub value: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub min: Option<String>,
    #[serde(default)]
    pub allowed: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq)]
pub struct Port {
    pub name: String,
    pub direction: String,
    #[serde(default)]
    pub width: Option<PortWidth>,
    #[serde(default)]
    pub clock: Option<String>,
    #[serde(default)]
    pub clock_domain: Option<String>,
    #[serde(default)]
    pub reset: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub active: Option<String>,
    #[serde(default)]
    pub reset_style: Option<String>,
    #[serde(default)]
    pub interface: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum PortWidth {
    Integer(u32),
    Parameter(String),
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct Clock {
    pub name: String,
    #[serde(default)]
    pub port: Option<String>,
    #[serde(default)]
    pub frequency_hz: Option<u64>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct Reset {
    pub name: String,
    #[serde(default)]
    pub port: Option<String>,
    #[serde(default)]
    pub active: Option<String>,
    #[serde(default, alias = "reset_style")]
    pub style: Option<String>,
    #[serde(default)]
    pub asynchronous: Option<bool>,
    #[serde(default)]
    pub clock_domain: Option<String>,
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
pub struct StreamInterface {
    pub name: String,
    pub kind: String,
    pub clock_domain: String,
    pub data: String,
    pub valid: String,
    pub ready: String,
    #[serde(default)]
    pub data_width: Option<String>,
    #[serde(default)]
    pub payload_semantics: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct Testbench {
    pub name: String,
    #[serde(default)]
    pub backend: Option<String>,
    pub top: String,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub rtl_sources: Vec<String>,
    #[serde(default)]
    pub expected: Option<String>,
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
    pub name: Option<String>,
    #[serde(default)]
    pub backend: Option<String>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub properties: Vec<String>,
    #[serde(default)]
    pub files: Vec<String>,
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

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct Complexity {
    pub class: ProjectClass,
    #[serde(default)]
    pub score: u8,
    #[serde(default)]
    pub decision: String,
    #[serde(default)]
    pub triggers: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct Architecture {
    #[serde(default)]
    pub style: Option<String>,
    #[serde(default)]
    pub reference_backend: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct Reuse {
    #[serde(default)]
    pub prefer_existing_microcores: bool,
}

#[derive(Clone, Debug, Default, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct Dependencies {
    #[serde(default)]
    pub cores: Vec<CoreDependency>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct CoreDependency {
    pub name: String,
    pub version: String,
    pub role: String,
}

#[derive(Clone, Debug, Default, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct Resources {
    #[serde(default)]
    pub memory: Vec<MemoryResource>,
    #[serde(default)]
    pub dsp: Vec<DspResource>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct MemoryResource {
    pub name: String,
    pub kind: String,
    pub width: u32,
    pub depth: u32,
    #[serde(default)]
    pub latency_cycles: Option<u32>,
    #[serde(default = "default_backend_policy")]
    pub backend_policy: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct DspResource {
    pub name: String,
    #[serde(default = "default_dsp_kind")]
    pub kind: String,
    pub count: u32,
    #[serde(default = "default_backend_policy")]
    pub backend_policy: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct BackendVariant {
    pub name: String,
    pub vendor: String,
    #[serde(default)]
    pub families: Vec<String>,
    pub status: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct Constructor {
    #[serde(default)]
    pub export: bool,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub compatibility_profile: Option<String>,
}

/// Roadmap priority for the universal-core registry.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum CorePriority {
    P0,
    P1,
    P2,
}

impl CorePriority {
    pub fn as_str(self) -> &'static str {
        match self {
            CorePriority::P0 => "P0",
            CorePriority::P1 => "P1",
            CorePriority::P2 => "P2",
        }
    }
}

impl std::str::FromStr for CorePriority {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "P0" => Ok(Self::P0),
            "P1" => Ok(Self::P1),
            "P2" => Ok(Self::P2),
            other => Err(format!(
                "unsupported priority `{other}` (expected P0, P1, or P2)"
            )),
        }
    }
}

/// Core-level maturity. Distinct from `BackendVariant.status` (per-vendor)
/// and from `boards[].status` (per-board).
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CoreMaturity {
    Experimental,
    Preview,
    Beta,
    Stable,
    Deprecated,
}

impl CoreMaturity {
    pub fn as_str(self) -> &'static str {
        match self {
            CoreMaturity::Experimental => "experimental",
            CoreMaturity::Preview => "preview",
            CoreMaturity::Beta => "beta",
            CoreMaturity::Stable => "stable",
            CoreMaturity::Deprecated => "deprecated",
        }
    }
}

impl std::str::FromStr for CoreMaturity {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "experimental" => Ok(Self::Experimental),
            "preview" => Ok(Self::Preview),
            "beta" => Ok(Self::Beta),
            "stable" => Ok(Self::Stable),
            "deprecated" => Ok(Self::Deprecated),
            other => Err(format!(
                "unsupported maturity `{other}` (expected experimental|preview|beta|stable|deprecated)"
            )),
        }
    }
}

/// A single declared verification gate, e.g. "formal-cdc-assumption" or
/// "board-demo". `evidence` optionally points to a file under the core
/// directory that records the gate outcome (log, report, screenshot).
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct VerificationGate {
    pub kind: VerificationKind,
    #[serde(default)]
    pub evidence: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum VerificationKind {
    Simulation,
    FormalCdcAssumption,
    FormalOccupancy,
    FormalEquivalence,
    RandomStress,
    BoardDemo,
    SynthesisReport,
}

impl VerificationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            VerificationKind::Simulation => "simulation",
            VerificationKind::FormalCdcAssumption => "formal-cdc-assumption",
            VerificationKind::FormalOccupancy => "formal-occupancy",
            VerificationKind::FormalEquivalence => "formal-equivalence",
            VerificationKind::RandomStress => "random-stress",
            VerificationKind::BoardDemo => "board-demo",
            VerificationKind::SynthesisReport => "synthesis-report",
        }
    }
}

/// Declarative evidence block. Lets a manifest record evidence URLs,
/// hashes, and tool/board/CI provenance that would otherwise be
/// reconstructed from `--build-root/reports/`. Sub-blocks are optional;
/// each one feeds a specific row in `ReusableCoreMaturity`.
///
/// Manifesto rule: declarative evidence is structured *input*, not a
/// fabricated assertion. The corresponding row only flips to
/// `supported` when the declared `commit_sha`/`conclusion` actually
/// match current state; otherwise it stays `planned`.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct EvidenceDeclaration {
    #[serde(default)]
    pub docker_ci_cd: Option<DockerCiEvidence>,
    #[serde(default)]
    pub vendor_tool: Option<VendorToolEvidence>,
    #[serde(default)]
    pub board_hardware: Option<BoardHardwareEvidence>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct DockerCiEvidence {
    pub run_url: String,
    pub commit_sha: String,
    pub sha256sums: String,
    pub conclusion: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct VendorToolEvidence {
    pub tool: String,
    pub report_path: String,
    pub conclusion: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct BoardHardwareEvidence {
    pub board_id: String,
    pub report_path: String,
    pub date: String,
}

fn default_manifest_version() -> String {
    "0.1".to_string()
}

fn default_language() -> String {
    "verilog-2001".to_string()
}

fn default_backend_policy() -> String {
    "portable".to_string()
}

fn default_dsp_kind() -> String {
    "dsp".to_string()
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

fn validate_backend_policy(policy: &str, issues: &mut Vec<ValidationIssue>) {
    if !matches!(policy, "portable" | "prefer_vendor" | "require_vendor") {
        issues.push(ValidationIssue::new(
            "AF_RESOURCE_BACKEND_POLICY_INVALID",
            format!("unsupported resource backend_policy `{policy}`"),
            "Use backend_policy = \"portable\", \"prefer_vendor\", or \"require_vendor\".",
        ));
    }
}

fn looks_generated_path(path: &str) -> bool {
    path == "generated"
        || path.starts_with("generated/")
        || path.contains("/generated/")
        || path.starts_with("build/")
        || path.contains("/build/")
}

fn validate_v02_required_shape(value: &Value, issues: &mut Vec<ValidationIssue>) {
    let Value::Table(table) = value else {
        return;
    };

    if !matches!(
        table.get("af_version").and_then(Value::as_str),
        Some("0.2" | "0.3")
    ) {
        return;
    }

    for field in ["name", "vendor", "library", "core", "version"] {
        if table
            .get(field)
            .and_then(Value::as_str)
            .is_none_or(|value| value.trim().is_empty())
        {
            issues.push(ValidationIssue::new(
                "AF_MANIFEST_V02_REQUIRED_FIELD_MISSING",
                format!("v0.2/v0.3 manifest requires root field `{field}`"),
                format!("Set `{field}` at the root of af-core.toml before migration."),
            ));
        }
    }

    let Some(rtl) = table.get("rtl").and_then(Value::as_table) else {
        issues.push(ValidationIssue::new(
            "AF_MANIFEST_V02_REQUIRED_FIELD_MISSING",
            "v0.2/v0.3 manifest requires [rtl]",
            "Add an [rtl] table with at least a `top` field.",
        ));
        return;
    };
    if rtl
        .get("top")
        .and_then(Value::as_str)
        .is_none_or(|top| top.trim().is_empty())
    {
        issues.push(ValidationIssue::new(
            "AF_MANIFEST_V02_REQUIRED_FIELD_MISSING",
            "v0.2/v0.3 manifest requires `rtl.top`",
            "Set the RTL top module name with `top = \"...\"`.",
        ));
    }

    let Some(sources) = table.get("sources").and_then(Value::as_table) else {
        issues.push(ValidationIssue::new(
            "AF_MANIFEST_V02_REQUIRED_FIELD_MISSING",
            "v0.2/v0.3 manifest requires [sources].files",
            "Add a `[sources]` table with at least one source entry in `files`.",
        ));
        return;
    };
    if sources
        .get("files")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        issues.push(ValidationIssue::new(
            "AF_MANIFEST_V02_REQUIRED_FIELD_MISSING",
            "v0.2/v0.3 manifest requires `[sources].files`",
            "Populate `[sources].files` with one or more relative source paths.",
        ));
    }

    for field in ["clocks", "resets", "ports"] {
        if table
            .get(field)
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
        {
            issues.push(ValidationIssue::new(
                "AF_MANIFEST_V02_REQUIRED_FIELD_MISSING",
                format!("v0.2/v0.3 manifest requires `{field}`"),
                format!("Add at least one `[[{field}]]` entry for v0.2/v0.3 manifests."),
            ));
        }
    }
}

fn normalize_manifest_value(value: &mut Value) {
    let Some(table) = value.as_table_mut() else {
        return;
    };

    if !table.contains_key("af_version") {
        if let Some(schema_version) = table.remove("schema_version") {
            table.insert("af_version".to_string(), schema_version);
        } else if let Some(manifest_version) = table.remove("manifest_version") {
            table.insert("af_version".to_string(), manifest_version);
        }
    }

    normalize_name_table(table);
    normalize_sources(table);
    normalize_formal(table);
    normalize_boards(table);
    normalize_backend_compatibility(table);
    normalize_known_limitations(table);
}

fn normalize_name_table(table: &mut Map<String, Value>) {
    let Some(Value::Table(name_table)) = table.get("name").cloned() else {
        return;
    };
    for key in ["vendor", "library", "core", "version"] {
        if let Some(value) = name_table.get(key).cloned() {
            table.entry(key.to_string()).or_insert(value);
        }
    }
    if !matches!(table.get("name"), Some(Value::String(_))) {
        if let Some(core) = name_table.get("core").cloned() {
            table.insert("name".to_string(), core);
        }
    }
}

fn normalize_sources(table: &mut Map<String, Value>) {
    let mut files = Vec::new();
    let mut roles = Map::new();
    let mut file_types = Map::new();

    if let Some(Value::Array(entries)) = table.get("sources").cloned() {
        for entry in entries {
            let Value::Table(entry) = entry else {
                continue;
            };
            let Some(path) = entry.get("path").and_then(Value::as_str) else {
                continue;
            };
            files.push(Value::String(path.to_string()));
            if let Some(role) = entry.get("role").and_then(Value::as_str) {
                roles.insert(path.to_string(), Value::String(role.to_string()));
            }
            if let Some(file_type) = entry.get("file_type").and_then(Value::as_str) {
                file_types.insert(path.to_string(), Value::String(file_type.to_string()));
            }
        }
        let mut source_table = Map::new();
        source_table.insert("files".to_string(), Value::Array(files));
        source_table.insert("include_dirs".to_string(), Value::Array(Vec::new()));
        source_table.insert("roles".to_string(), Value::Table(roles));
        source_table.insert("file_types".to_string(), Value::Table(file_types));
        table.insert("sources".to_string(), Value::Table(source_table));
    }

    let include_dirs = table.remove("include_dirs");
    let Some(Value::Array(include_dirs)) = include_dirs else {
        return;
    };
    let source_table = table
        .entry("sources".to_string())
        .or_insert_with(|| Value::Table(Map::new()));
    let Some(source_table) = source_table.as_table_mut() else {
        return;
    };
    let dirs = include_dirs
        .into_iter()
        .filter_map(|entry| match entry {
            Value::String(path) => Some(Value::String(path)),
            Value::Table(table) => table
                .get("path")
                .and_then(Value::as_str)
                .map(|path| Value::String(path.to_string())),
            _ => None,
        })
        .collect::<Vec<_>>();
    source_table.insert("include_dirs".to_string(), Value::Array(dirs));
}

fn normalize_formal(table: &mut Map<String, Value>) {
    let Some(Value::Array(entries)) = table.get("formal").cloned() else {
        return;
    };
    let mut out = Map::new();
    let mut files = Vec::new();
    let mut enabled = false;
    let mut name = None;
    let mut backend = None;
    for entry in entries {
        let Value::Table(entry) = entry else {
            continue;
        };
        enabled |= entry
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if name.is_none() {
            name = entry
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_string);
        }
        if backend.is_none() {
            backend = entry
                .get("backend")
                .and_then(Value::as_str)
                .map(str::to_string);
        }
        if let Some(Value::Array(entry_files)) = entry.get("files") {
            files.extend(
                entry_files
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|path| Value::String(path.to_string())),
            );
        }
    }
    out.insert("enabled".to_string(), Value::Boolean(enabled));
    out.insert("files".to_string(), Value::Array(files));
    if let Some(name) = name {
        out.insert("name".to_string(), Value::String(name));
    }
    if let Some(backend) = backend {
        out.insert("backend".to_string(), Value::String(backend));
    }
    table.insert("formal".to_string(), Value::Table(out));
}

fn normalize_boards(table: &mut Map<String, Value>) {
    let Some(Value::Array(entries)) = table.get("boards").cloned() else {
        return;
    };
    let boards = entries
        .into_iter()
        .filter_map(|entry| match entry {
            Value::String(name) => Some(Value::String(name)),
            Value::Table(table) => table
                .get("name")
                .and_then(Value::as_str)
                .map(|name| Value::String(name.to_string())),
            _ => None,
        })
        .collect::<Vec<_>>();
    table.insert("boards".to_string(), Value::Array(boards));
}

fn normalize_backend_compatibility(table: &mut Map<String, Value>) {
    let Some(Value::Array(entries)) = table.get("backend_compatibility").cloned() else {
        return;
    };
    let mut out = Map::new();
    for entry in entries {
        let Value::Table(entry) = entry else {
            continue;
        };
        let Some(backend) = entry.get("backend").and_then(Value::as_str) else {
            continue;
        };
        let supported = matches!(
            entry.get("status").and_then(Value::as_str),
            Some("supported") | Some("planned")
        );
        out.insert(backend.to_string(), Value::Boolean(supported));
    }
    table.insert("backend_compatibility".to_string(), Value::Table(out));
}

fn normalize_known_limitations(table: &mut Map<String, Value>) {
    let Some(Value::Array(entries)) = table.get("known_limitations").cloned() else {
        return;
    };
    let limitations = entries
        .into_iter()
        .filter_map(|entry| match entry {
            Value::String(text) => Some(Value::String(text)),
            Value::Table(table) => table
                .get("text")
                .and_then(Value::as_str)
                .map(|text| Value::String(text.to_string())),
            _ => None,
        })
        .collect::<Vec<_>>();
    table.insert("known_limitations".to_string(), Value::Array(limitations));
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

    #[test]
    fn parses_expanded_v01_manifest_shape() {
        let manifest = CoreManifest::from_toml_str(
            r#"
schema_version = "0.1"
kind = "accelfury.core"

[name]
vendor = "accelfury"
library = "audio"
core = "af-pdm-rx"
version = "0.1.0"

[metadata]
license = "Apache-2.0"
display_name = "AccelFury PDM RX"

[rtl]
top = "af_pdm_rx"
language = "verilog-2001"
default_clock = "clk"
default_reset = "rst_n"

[[sources]]
path = "rtl/af_pdm_rx.v"
file_type = "verilogSource"
role = "rtl"

[[include_dirs]]
path = "rtl/include"

[[parameters]]
name = "WORD_BITS"
kind = "integer"
default = "32"

[[ports]]
name = "clk"
direction = "input"
width = 1
kind = "clock"
clock_domain = "sys"

[[ports]]
name = "rst_n"
direction = "input"
width = 1
kind = "reset"
clock_domain = "sys"

[[ports]]
name = "sample_word_o"
direction = "output"
width = "WORD_BITS"
kind = "data"
clock_domain = "sys"

[[ports]]
name = "sample_valid_o"
direction = "output"
width = 1
clock_domain = "sys"

[[ports]]
name = "sample_ready_i"
direction = "input"
width = 1
clock_domain = "sys"

[[clocks]]
name = "sys"
port = "clk"
frequency_hz = 27000000

[[resets]]
name = "sys_rst_n"
port = "rst_n"
active = "low"
style = "async"
clock_domain = "sys"

[[stream_interfaces]]
name = "raw_stream"
kind = "valid_ready"
clock_domain = "sys"
data = "sample_word_o"
valid = "sample_valid_o"
ready = "sample_ready_i"
data_width = "WORD_BITS"

[[backend_compatibility]]
backend = "verilator"
status = "supported"

[[known_limitations]]
id = "LIM-001"
text = "Raw PDM only."
"#,
            "af-core.toml",
        )
        .unwrap();
        assert_eq!(manifest.af_version, "0.1");
        assert_eq!(manifest.kind.as_deref(), Some("accelfury.core"));
        assert_eq!(manifest.vlnv(), "accelfury:audio:af-pdm-rx:0.1.0");
        assert_eq!(manifest.sources.files, vec!["rtl/af_pdm_rx.v"]);
        assert_eq!(manifest.sources.include_dirs, vec!["rtl/include"]);
        assert_eq!(manifest.known_limitations, vec!["Raw PDM only."]);
        assert_eq!(manifest.stream_interfaces[0].data, "sample_word_o");
    }

    #[test]
    fn rejects_missing_required_v02_fields() {
        let raw = r#"
af_version = "0.2"
name = "example-core"
vendor = "accelfury"
library = "ip"
core = "example_core"
version = "0.1.0"

[rtl]
top = "example_core"

[sources]
files = ["rtl/example_core.sv"]
"#;
        let err = CoreManifest::from_toml_str(raw, "af-core.toml").unwrap_err();
        let issues = validation_issues(err);

        let required_codes = issues
            .iter()
            .filter(|issue| issue.code == "AF_MANIFEST_V02_REQUIRED_FIELD_MISSING")
            .map(|issue| issue.message.as_str())
            .collect::<Vec<_>>();

        assert!(required_codes
            .iter()
            .any(|message| message.contains("`clocks`")));
        assert!(required_codes
            .iter()
            .any(|message| message.contains("`resets`")));
        assert!(required_codes
            .iter()
            .any(|message| message.contains("`ports`")));
    }

    #[test]
    fn parses_v03_complexity_resources_and_constructor() {
        let manifest = CoreManifest::from_toml_str(
            r#"
af_version = "0.3"
name = "complex-demo"
vendor = "accelfury"
library = "ip"
core = "complex_demo"
version = "0.1.0"
known_limitations = ["xilinx backend is planned"]

[complexity]
class = "complex-vendor-aware"
score = 8
decision = "memory banking requires vendor backends"
triggers = ["memory_banking", "vendor_dsp_backend_required"]

[architecture]
style = "portable_contract_with_vendor_backends"
reference_backend = "generic"

[reuse]
prefer_existing_microcores = true

[rtl]
top = "complex_demo"
language = "verilog-2001"

[sources]
files = ["rtl/complex_demo.v"]

[[clocks]]
name = "clk"
port = "clk"

[[resets]]
name = "rst_n"
port = "rst_n"
active = "low"

[[ports]]
name = "clk"
direction = "input"
width = 1

[[ports]]
name = "rst_n"
direction = "input"
width = 1

[[resources.memory]]
name = "work_ram"
kind = "ram_1r1w"
width = 64
depth = 4096
backend_policy = "prefer_vendor"

[[resources.dsp]]
name = "mul"
count = 8
backend_policy = "require_vendor"

[[backend_variants]]
name = "xilinx_ultrascale_plus"
vendor = "xilinx"
families = ["ultrascale-plus"]
status = "planned"

[constructor]
export = true
category = "compute"
compatibility_profile = "af_stream_v1"
"#,
            "af-core.toml",
        )
        .unwrap();
        assert_eq!(manifest.af_version, "0.3");
        assert_eq!(
            manifest.complexity.as_ref().unwrap().class,
            ProjectClass::ComplexVendorAware
        );
        assert_eq!(manifest.resources.memory[0].backend_policy, "prefer_vendor");
        assert!(manifest.constructor.as_ref().unwrap().export);
        assert!(manifest.validate().valid);
    }

    #[test]
    fn parses_manifesto_axes() {
        let manifest = CoreManifest::from_toml_str(
            r#"
af_version = "0.3"
name = "af-reset-sync"
vendor = "accelfury"
library = "utility"
core = "af_reset_sync"
version = "0.1.0"
portability_level = "U0"
priority = "P0"
maturity = "preview"

[rtl]
top = "af_reset_sync"
language = "verilog-2001"

[sources]
files = ["rtl/af_reset_sync.v"]

[[clocks]]
name = "clk"
port = "clk"

[[resets]]
name = "src_rst_n"
port = "src_rst_n"
active = "low"
style = "async"
clock_domain = "clk"

[[ports]]
name = "clk"
direction = "input"
width = 1

[[ports]]
name = "src_rst_n"
direction = "input"
width = 1

[[verification_required]]
kind = "formal-cdc-assumption"
description = "Two-stage synchronizer reset removal"

[[verification_required]]
kind = "simulation"
evidence = "reports/smoke.log"
"#,
            "af-core.toml",
        )
        .unwrap();
        assert_eq!(manifest.portability_level, Some(PortabilityLevel::U0));
        assert_eq!(manifest.priority, Some(CorePriority::P0));
        assert_eq!(manifest.maturity, Some(CoreMaturity::Preview));
        assert_eq!(manifest.verification_required.len(), 2);
        assert_eq!(
            manifest.verification_required[0].kind,
            VerificationKind::FormalCdcAssumption
        );
        assert_eq!(
            manifest.verification_required[1].evidence.as_deref(),
            Some("reports/smoke.log")
        );
        assert!(manifest.validate().valid);
    }

    #[test]
    fn rejects_absolute_verification_evidence_path() {
        let raw = r#"
af_version = "0.3"
name = "af-reset-sync"
vendor = "accelfury"
library = "utility"
core = "af_reset_sync"
version = "0.1.0"

[rtl]
top = "af_reset_sync"
language = "verilog-2001"

[sources]
files = ["rtl/af_reset_sync.v"]

[[clocks]]
name = "clk"
port = "clk"

[[resets]]
name = "src_rst_n"
port = "src_rst_n"
active = "low"

[[ports]]
name = "clk"
direction = "input"
width = 1

[[ports]]
name = "src_rst_n"
direction = "input"
width = 1

[[verification_required]]
kind = "simulation"
evidence = "/etc/passwd"
"#;
        let err = CoreManifest::from_toml_str(raw, "af-core.toml").unwrap_err();
        let issues = validation_issues(err);
        assert!(issues.iter().any(|issue| issue.code == "AF_PATH_ABSOLUTE"));
    }

    #[test]
    fn parses_evidence_declaration() {
        let manifest = CoreManifest::from_toml_str(
            r#"
af_version = "0.3"
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

[[clocks]]
name = "clk"
port = "clk"

[[resets]]
name = "rst_n"
port = "rst_n"
active = "low"

[[ports]]
name = "clk"
direction = "input"
width = 1

[[ports]]
name = "rst_n"
direction = "input"
width = 1

[evidence.docker_ci_cd]
run_url = "https://github.com/accelfury/af/actions/runs/12345"
commit_sha = "abc1234deadbeef"
sha256sums = "abc1234deadbeef0"
conclusion = "success"

[evidence.vendor_tool]
tool = "vivado"
report_path = "reports/vivado/synth.json"
conclusion = "success"

[evidence.board_hardware]
board_id = "tang-nano-20k"
report_path = "reports/board/tang_smoke.json"
date = "2026-05-17"
"#,
            "af-core.toml",
        )
        .unwrap();
        let ev = manifest.evidence.expect("evidence block");
        let ci = ev.docker_ci_cd.unwrap();
        assert_eq!(ci.commit_sha, "abc1234deadbeef");
        assert_eq!(ci.conclusion, "success");
        let vt = ev.vendor_tool.unwrap();
        assert_eq!(vt.tool, "vivado");
        assert_eq!(vt.report_path, "reports/vivado/synth.json");
        let bh = ev.board_hardware.unwrap();
        assert_eq!(bh.board_id, "tang-nano-20k");
    }

    #[test]
    fn rejects_evidence_with_absolute_report_path() {
        let raw = r#"
af_version = "0.3"
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

[[clocks]]
name = "clk"
port = "clk"

[[resets]]
name = "rst_n"
port = "rst_n"
active = "low"

[[ports]]
name = "clk"
direction = "input"
width = 1

[[ports]]
name = "rst_n"
direction = "input"
width = 1

[evidence.vendor_tool]
tool = "vivado"
report_path = "/etc/passwd"
conclusion = "success"
"#;
        let err = CoreManifest::from_toml_str(raw, "af-core.toml").unwrap_err();
        let issues = validation_issues(err);
        assert!(issues.iter().any(|issue| issue.code == "AF_PATH_ABSOLUTE"));
    }

    #[test]
    fn rejects_unknown_vendor_tool_in_evidence() {
        let raw = r#"
af_version = "0.3"
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

[[clocks]]
name = "clk"
port = "clk"

[[resets]]
name = "rst_n"
port = "rst_n"
active = "low"

[[ports]]
name = "clk"
direction = "input"
width = 1

[[ports]]
name = "rst_n"
direction = "input"
width = 1

[evidence.vendor_tool]
tool = "weirdsoft"
report_path = "reports/wat.json"
conclusion = "success"
"#;
        let err = CoreManifest::from_toml_str(raw, "af-core.toml").unwrap_err();
        let issues = validation_issues(err);
        assert!(issues
            .iter()
            .any(|issue| issue.code == "AF_EVIDENCE_VENDOR_TOOL_INVALID"));
    }

    fn validation_issues(err: ManifestError) -> Vec<ValidationIssue> {
        match err {
            ManifestError::Validation { issues } => issues,
            other => unreachable!("expected validation error, got {other:?}"),
        }
    }
}
