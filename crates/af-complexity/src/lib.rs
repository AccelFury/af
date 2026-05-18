// SPDX-License-Identifier: Apache-2.0
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use thiserror::Error;
use toml::Value;

pub const GENERATED_BY: &str = "AccelFury IP Toolchain";

#[derive(
    Clone, Copy, Debug, Deserialize, Eq, JsonSchema, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectClass {
    SimplePortable,
    CompositePortable,
    ComplexVendorAware,
    SystemPlatform,
    ProductStack,
}

impl ProjectClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SimplePortable => "simple-portable",
            Self::CompositePortable => "composite-portable",
            Self::ComplexVendorAware => "complex-vendor-aware",
            Self::SystemPlatform => "system-platform",
            Self::ProductStack => "product-stack",
        }
    }

    /// Manifesto-aligned portability axis (parallel to `ProjectClass`).
    ///
    /// `SimplePortable` cores typically sit at U0 or U1 (fully portable RTL or
    /// portable via inference). `CompositePortable` spans U1–U2 (thin vendor
    /// wrappers). `ComplexVendorAware` spans U2–U3 (single specification with
    /// vendor backend). `SystemPlatform` is U3. `ProductStack` is U4 (no
    /// portable replacement; only abstraction/wrapper/mock applies).
    pub fn portability_levels(self) -> &'static [PortabilityLevel] {
        match self {
            Self::SimplePortable => &[PortabilityLevel::U0, PortabilityLevel::U1],
            Self::CompositePortable => &[PortabilityLevel::U1, PortabilityLevel::U2],
            Self::ComplexVendorAware => &[PortabilityLevel::U2, PortabilityLevel::U3],
            Self::SystemPlatform => &[PortabilityLevel::U3],
            Self::ProductStack => &[PortabilityLevel::U4],
        }
    }

    pub fn required_artifacts(self) -> Vec<String> {
        match self {
            Self::SimplePortable => vec!["af-core.toml", "rtl/"],
            Self::CompositePortable => vec!["af-core.toml", "rtl/", "docs/", "tests/"],
            Self::ComplexVendorAware => {
                vec![
                    "af-core.toml",
                    "af-arch.toml",
                    "rtl/common/",
                    "vendor/",
                    "constructor/",
                ]
            }
            Self::SystemPlatform => vec!["af-project.toml", "cores/", "platforms/", "constraints/"],
            Self::ProductStack => vec!["af-product.toml", "packages/", "constructor_catalog/"],
        }
        .into_iter()
        .map(str::to_string)
        .collect()
    }
}

impl fmt::Display for ProjectClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ProjectClass {
    type Err = ProjectClassParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "simple-portable" => Ok(Self::SimplePortable),
            "composite-portable" => Ok(Self::CompositePortable),
            "complex-vendor-aware" => Ok(Self::ComplexVendorAware),
            "system-platform" => Ok(Self::SystemPlatform),
            "product-stack" => Ok(Self::ProductStack),
            _ => Err(ProjectClassParseError {
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[error("unsupported project class `{value}`")]
pub struct ProjectClassParseError {
    pub value: String,
}

/// Manifesto-aligned portability axis (U0..U4). Parallel to `ProjectClass`.
///
/// U0 — fully portable RTL. U1 — portable through inference (e.g. RAM/FIFO
/// inference). U2 — portable RTL plus thin vendor wrappers. U3 — single
/// specification with vendor-specific backend. U4 — replacement is not
/// reasonable; only abstraction/wrapper/mock makes sense.
#[derive(
    Clone, Copy, Debug, Deserialize, Eq, JsonSchema, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(rename_all = "UPPERCASE")]
pub enum PortabilityLevel {
    U0,
    U1,
    U2,
    U3,
    U4,
}

impl PortabilityLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::U0 => "U0",
            Self::U1 => "U1",
            Self::U2 => "U2",
            Self::U3 => "U3",
            Self::U4 => "U4",
        }
    }
}

impl fmt::Display for PortabilityLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PortabilityLevel {
    type Err = PortabilityLevelParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "U0" => Ok(Self::U0),
            "U1" => Ok(Self::U1),
            "U2" => Ok(Self::U2),
            "U3" => Ok(Self::U3),
            "U4" => Ok(Self::U4),
            _ => Err(PortabilityLevelParseError {
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[error("unsupported portability level `{value}` (expected U0..U4)")]
pub struct PortabilityLevelParseError {
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ClassificationReport {
    pub generated_by: String,
    pub project_class: ProjectClass,
    pub score: u8,
    pub triggers: Vec<String>,
    pub recommended_template: String,
    pub required_artifacts: Vec<String>,
    pub warnings: Vec<String>,
    /// Manifesto portability axis candidates that typically pair with the
    /// inferred `project_class`. Source of truth: `ProjectClass::portability_levels()`.
    #[serde(default)]
    pub candidate_portability_levels: Vec<String>,
}

#[derive(Debug, Error)]
pub enum ComplexityError {
    #[error("failed to read `{path}`: {message}")]
    Read { path: PathBuf, message: String },
    #[error("failed to parse `{path}`: {message}")]
    Parse { path: PathBuf, message: String },
    #[error("spec file `{path}` is empty or whitespace-only")]
    EmptySpec { path: PathBuf },
}

impl ComplexityError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Read { .. } => "AF_COMPLEXITY_INPUT_READ_FAILED",
            Self::Parse { .. } => "AF_COMPLEXITY_INPUT_PARSE_FAILED",
            Self::EmptySpec { .. } => "AF_COMPLEXITY_SPEC_EMPTY",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            Self::Read { .. } => "Pass a readable project directory, manifest, or spec file.",
            Self::Parse { .. } => {
                "Fix TOML syntax or use --from-spec for free-form requirements text."
            }
            Self::EmptySpec { .. } => {
                "Provide a non-empty requirements/specification document so the classifier has signal; an empty spec cannot be classified honestly."
            }
        }
    }

    pub fn exit_code(&self) -> i32 {
        2
    }
}

pub fn classify_path(path: impl AsRef<Path>) -> Result<ClassificationReport, ComplexityError> {
    let path = path.as_ref();
    if path.is_file() {
        let raw = read_to_string(path)?;
        if path.file_name().and_then(|name| name.to_str()) == Some("af-core.toml") {
            return classify_manifest_str(&raw, path);
        }
        return Ok(classify_spec_text(&raw));
    }

    let mut evidence = String::new();
    let mut manifest_report = None;
    for file in [
        path.join("af-core.toml"),
        path.join("af-project.toml"),
        path.join("af-product.toml"),
    ] {
        if file.is_file() {
            let raw = read_to_string(&file)?;
            evidence.push_str(&raw);
            evidence.push('\n');
            manifest_report = Some(classify_manifest_str(&raw, &file)?);
        }
    }
    collect_tree_evidence(path, &mut evidence, 0);

    let text_report = classify_spec_text(&evidence);
    Ok(match manifest_report {
        Some(manifest_report) => merge_reports(manifest_report, text_report),
        None => text_report,
    })
}

pub fn classify_manifest_str(
    raw: &str,
    origin: impl AsRef<Path>,
) -> Result<ClassificationReport, ComplexityError> {
    let origin = origin.as_ref();
    let value: Value = toml::from_str(raw).map_err(|err| ComplexityError::Parse {
        path: origin.to_path_buf(),
        message: err.to_string(),
    })?;
    Ok(classify_toml_value(&value, raw))
}

pub fn classify_spec_file(path: impl AsRef<Path>) -> Result<ClassificationReport, ComplexityError> {
    let path = path.as_ref();
    let raw = read_to_string(path)?;
    if raw.trim().is_empty() {
        return Err(ComplexityError::EmptySpec {
            path: path.to_path_buf(),
        });
    }
    Ok(classify_spec_text(&raw))
}

pub fn classify_spec_text(text: &str) -> ClassificationReport {
    let mut model = ScoreModel::default();
    score_text(&mut model, text);
    model.finish(None)
}

fn classify_toml_value(value: &Value, raw: &str) -> ClassificationReport {
    let mut model = ScoreModel::default();
    score_text(&mut model, raw);

    let explicit_class = get_path_str(value, &["complexity", "class"])
        .or_else(|| get_path_str(value, &["project", "class"]))
        .and_then(|class| class.parse::<ProjectClass>().ok());
    let explicit_score = get_path_integer(value, &["complexity", "score"]).map(|score| score as u8);
    if let Some(score) = explicit_score {
        model.score = model.score.max(score);
    }

    if value.get("af-product").is_some() || value.get("product").is_some() {
        model.add(10, "product_catalog");
    }
    if value.get("project").is_some() || value.get("platforms").is_some() {
        model.add(8, "platform_layer");
    }
    if array_len(value, "clocks") > 1 {
        model.add(2, "multi_clock");
    }
    if array_len(value, "stream_interfaces") > 1 || array_len(value, "interfaces") > 1 {
        model.add(2, "multiple_interfaces");
    }
    if get_path(value, &["dependencies", "cores"]).is_some() {
        model.add(2, "core_dependencies");
    }
    if get_path(value, &["resources", "memory"]).is_some() {
        model.add(2, "memory_banking");
    }
    if get_path(value, &["resources", "dsp"]).is_some() {
        model.add(2, "vendor_dsp_backend_required");
    }
    if value.get("backend_variants").is_some() {
        model.add(3, "vendor_backend_matrix");
    }
    if get_path_bool(value, &["constructor", "export"]) == Some(true)
        || value.get("constructor_catalog").is_some()
    {
        model.add(2, "online_constructor");
    }

    model.finish(explicit_class)
}

fn merge_reports(left: ClassificationReport, right: ClassificationReport) -> ClassificationReport {
    if right.project_class > left.project_class {
        let mut warnings = merge_warnings(left.warnings, right.warnings);
        warnings.sort();
        warnings.dedup();
        return ClassificationReport {
            generated_by: GENERATED_BY.to_string(),
            project_class: right.project_class,
            score: left.score.max(right.score),
            triggers: merge_strings(left.triggers, right.triggers),
            recommended_template: right.project_class.as_str().to_string(),
            required_artifacts: right.project_class.required_artifacts(),
            warnings,
            candidate_portability_levels: candidate_portability_strings(right.project_class),
        };
    }
    let project_class = left.project_class;
    ClassificationReport {
        generated_by: GENERATED_BY.to_string(),
        project_class,
        score: left.score.max(right.score),
        triggers: merge_strings(left.triggers, right.triggers),
        recommended_template: project_class.as_str().to_string(),
        required_artifacts: project_class.required_artifacts(),
        warnings: merge_warnings(left.warnings, right.warnings),
        candidate_portability_levels: candidate_portability_strings(project_class),
    }
}

fn merge_warnings(left: Vec<String>, right: Vec<String>) -> Vec<String> {
    let left_has_inferred = left
        .iter()
        .any(|warning| warning.contains("AF_COMPLEXITY_CLASS_INFERRED"));
    let right = right
        .into_iter()
        .filter(|warning| left_has_inferred || !warning.contains("AF_COMPLEXITY_CLASS_INFERRED"))
        .collect::<Vec<_>>();
    merge_strings(left, right)
}

#[derive(Default)]
struct ScoreModel {
    score: u8,
    triggers: BTreeSet<String>,
}

impl ScoreModel {
    fn add(&mut self, score: u8, trigger: &str) {
        self.score = self.score.saturating_add(score);
        self.triggers.insert(trigger.to_string());
    }

    fn finish(self, explicit_class: Option<ProjectClass>) -> ClassificationReport {
        let inferred = class_from_score(self.score, &self.triggers);
        let mut warnings = Vec::new();
        let project_class = match explicit_class {
            Some(class) if class < inferred => {
                warnings.push(format!(
                    "AF_COMPLEXITY_UNDERMODELED: manifest class `{}` is below inferred `{}`",
                    class, inferred
                ));
                inferred
            }
            Some(class) => class,
            None => {
                warnings.push("AF_COMPLEXITY_CLASS_INFERRED: run `af project classify` before committing a complex accelerator template".to_string());
                inferred
            }
        };
        let score = self.score.max(default_score(project_class));
        let triggers = self.triggers.into_iter().collect::<Vec<_>>();
        ClassificationReport {
            generated_by: GENERATED_BY.to_string(),
            project_class,
            score,
            triggers,
            recommended_template: project_class.as_str().to_string(),
            required_artifacts: project_class.required_artifacts(),
            warnings,
            candidate_portability_levels: candidate_portability_strings(project_class),
        }
    }
}

fn candidate_portability_strings(class: ProjectClass) -> Vec<String> {
    class
        .portability_levels()
        .iter()
        .map(|level| level.as_str().to_string())
        .collect()
}

fn class_from_score(score: u8, triggers: &BTreeSet<String>) -> ProjectClass {
    if triggers.contains("product_catalog") {
        ProjectClass::ProductStack
    } else if triggers.contains("platform_layer") {
        ProjectClass::SystemPlatform
    } else if score >= 6
        || triggers.contains("vendor_backend_matrix")
        || triggers.contains("vendor_dsp_backend_required")
    {
        ProjectClass::ComplexVendorAware
    } else if score >= 3
        || triggers.contains("core_dependencies")
        || triggers.contains("multiple_interfaces")
    {
        ProjectClass::CompositePortable
    } else {
        ProjectClass::SimplePortable
    }
}

fn default_score(class: ProjectClass) -> u8 {
    match class {
        ProjectClass::SimplePortable => 1,
        ProjectClass::CompositePortable => 3,
        ProjectClass::ComplexVendorAware => 6,
        ProjectClass::SystemPlatform => 9,
        ProjectClass::ProductStack => 12,
    }
}

fn score_text(model: &mut ScoreModel, text: &str) {
    let lower = text.to_ascii_lowercase();
    for (trigger, score, needles) in [
        (
            "multi_clock",
            2,
            &["multi_clock", "multi clock", "clock domain", "cdc"][..],
        ),
        (
            "memory_banking",
            2,
            &[
                "memory_banking",
                "memory banking",
                "banked memory",
                "bram",
                "uram",
                "hbm",
                "ram_1r1w",
            ],
        ),
        (
            "vendor_dsp_backend_required",
            2,
            &[
                "vendor_dsp",
                "vendor dsp",
                "dsp48",
                "dsp block",
                "multiplier tree",
            ],
        ),
        (
            "vendor_clocking",
            2,
            &["pll", "mmcm", "clock wizard", "hard ip"],
        ),
        (
            "vendor_backend_matrix",
            3,
            &[
                "backend_variants",
                "vendor/",
                "xilinx",
                "intel",
                "gowin",
                "lattice",
            ],
        ),
        (
            "platform_layer",
            8,
            &[
                "af-project.toml",
                "pcie",
                "ddr controller",
                "ddr4",
                "ddr5",
                "lpddr",
                "serdes",
                "hbm",
            ],
        ),
        (
            "product_catalog",
            10,
            &["af-product.toml", "product-stack", "constructor_catalog"],
        ),
        (
            "online_constructor",
            2,
            &["constructor", "online constructor"],
        ),
        (
            "core_dependencies",
            2,
            &["dependencies.cores", "microcore", "reuse"],
        ),
        (
            "multiple_interfaces",
            2,
            &["axi", "avalon", "wishbone", "ready_valid", "valid_ready"],
        ),
    ] {
        if needles.iter().any(|needle| lower.contains(needle)) {
            model.add(score, trigger);
        }
    }
}

fn collect_tree_evidence(path: &Path, evidence: &mut String, depth: usize) {
    if depth > 4 {
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten().take(256) {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        evidence.push_str(name);
        evidence.push('\n');
        if path.is_dir() && !matches!(name, ".git" | "target" | ".af-build") {
            collect_tree_evidence(&path, evidence, depth + 1);
        } else if matches!(
            path.extension().and_then(|ext| ext.to_str()),
            Some("toml" | "md" | "v" | "sv")
        ) {
            if let Ok(text) = fs::read_to_string(&path) {
                evidence.push_str(&text);
                evidence.push('\n');
            }
        }
    }
}

fn get_path<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn get_path_str(value: &Value, path: &[&str]) -> Option<String> {
    get_path(value, path)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn get_path_bool(value: &Value, path: &[&str]) -> Option<bool> {
    get_path(value, path).and_then(Value::as_bool)
}

fn get_path_integer(value: &Value, path: &[&str]) -> Option<i64> {
    get_path(value, path).and_then(Value::as_integer)
}

fn array_len(value: &Value, key: &str) -> usize {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default()
}

fn merge_strings(left: Vec<String>, right: Vec<String>) -> Vec<String> {
    let mut values = left.into_iter().chain(right).collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn read_to_string(path: &Path) -> Result<String, ComplexityError> {
    fs::read_to_string(path).map_err(|err| ComplexityError::Read {
        path: path.to_path_buf(),
        message: err.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_spec_emits_candidate_portability_levels() {
        let report = classify_spec_text("single portable counter");
        assert_eq!(report.project_class, ProjectClass::SimplePortable);
        assert_eq!(
            report.candidate_portability_levels,
            vec!["U0".to_string(), "U1".to_string()]
        );
    }

    #[test]
    fn portability_levels_cover_every_project_class() {
        use PortabilityLevel::*;
        assert_eq!(
            ProjectClass::SimplePortable.portability_levels(),
            &[U0, U1] as &[_]
        );
        assert_eq!(
            ProjectClass::CompositePortable.portability_levels(),
            &[U1, U2] as &[_]
        );
        assert_eq!(
            ProjectClass::ComplexVendorAware.portability_levels(),
            &[U2, U3] as &[_]
        );
        assert_eq!(
            ProjectClass::SystemPlatform.portability_levels(),
            &[U3] as &[_]
        );
        assert_eq!(
            ProjectClass::ProductStack.portability_levels(),
            &[U4] as &[_]
        );
    }

    #[test]
    fn portability_level_roundtrip() {
        for raw in ["U0", "U1", "U2", "U3", "U4"] {
            let level: PortabilityLevel = raw.parse().unwrap();
            assert_eq!(level.as_str(), raw);
        }
        assert!("u0".parse::<PortabilityLevel>().is_err());
        assert!("U5".parse::<PortabilityLevel>().is_err());
    }

    #[test]
    fn class_strings_are_stable() {
        assert_eq!(
            ProjectClass::ComplexVendorAware.as_str(),
            "complex-vendor-aware"
        );
        assert_eq!(
            "product-stack".parse::<ProjectClass>().unwrap(),
            ProjectClass::ProductStack
        );
    }

    #[test]
    fn detects_complex_vendor_aware_spec() {
        let report =
            classify_spec_text("multi clock NTT with memory banking and vendor DSP backend");
        assert_eq!(report.project_class, ProjectClass::ComplexVendorAware);
        assert!(report.triggers.contains(&"memory_banking".to_string()));
    }

    #[test]
    fn classifies_all_project_classes_from_specs() {
        for (spec, expected) in [
            ("single portable counter", ProjectClass::SimplePortable),
            (
                "ready_valid reusable microcore pipeline with dependencies",
                ProjectClass::CompositePortable,
            ),
            (
                "multi clock NTT with memory banking and vendor DSP backend",
                ProjectClass::ComplexVendorAware,
            ),
            (
                "PCIe HBM accelerator with board integration constraints",
                ProjectClass::SystemPlatform,
            ),
            (
                "product-stack constructor_catalog package catalog",
                ProjectClass::ProductStack,
            ),
        ] {
            let report = classify_spec_text(spec);
            assert_eq!(report.project_class, expected, "{spec}");
            assert_eq!(report.recommended_template, expected.as_str());
            assert_eq!(report.required_artifacts, expected.required_artifacts());
        }
    }

    #[test]
    fn classify_spec_file_rejects_empty_spec() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.md");
        std::fs::write(&path, "").unwrap();
        match classify_spec_file(&path) {
            Err(ComplexityError::EmptySpec { path: reported }) => {
                assert_eq!(reported, path);
            }
            other => panic!("expected EmptySpec, got {other:?}"),
        }
    }

    #[test]
    fn classify_spec_file_rejects_whitespace_only_spec() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ws.md");
        std::fs::write(&path, "   \n\t\n   ").unwrap();
        assert!(matches!(
            classify_spec_file(&path),
            Err(ComplexityError::EmptySpec { .. })
        ));
    }

    #[test]
    fn classify_spec_file_accepts_meaningful_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ok.md");
        std::fs::write(&path, "single portable counter").unwrap();
        let report = classify_spec_file(&path).unwrap();
        assert_eq!(report.project_class, ProjectClass::SimplePortable);
    }

    #[test]
    fn warns_when_manifest_is_under_modeled() {
        let report = classify_manifest_str(
            r#"
af_version = "0.3"
[complexity]
class = "simple-portable"

[[resources.memory]]
name = "work_ram"
kind = "ram_1r1w"
width = 64
depth = 4096
backend_policy = "require_vendor"

[[backend_variants]]
name = "xilinx_ultrascale_plus"
vendor = "xilinx"
families = ["ultrascale-plus"]
status = "planned"
"#,
            "af-core.toml",
        )
        .unwrap();
        assert_eq!(report.project_class, ProjectClass::ComplexVendorAware);
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("AF_COMPLEXITY_UNDERMODELED")));
    }
}
