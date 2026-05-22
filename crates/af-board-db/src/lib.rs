// SPDX-License-Identifier: Apache-2.0
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use toml::{map::Map, Value};

#[derive(Debug, Error)]
pub enum BoardDbError {
    #[error("failed to read board profile `{path}`: {message}")]
    Read { path: PathBuf, message: String },
    #[error("failed to parse board profile `{path}`: {message}")]
    Parse { path: PathBuf, message: String },
    #[error("board profile validation failed")]
    Validation { issues: Vec<BoardIssue> },
}

impl BoardDbError {
    pub fn code(&self) -> &'static str {
        match self {
            BoardDbError::Read { .. } => "AF_BOARD_READ_FAILED",
            BoardDbError::Parse { .. } => "AF_BOARD_PARSE_FAILED",
            BoardDbError::Validation { .. } => "AF_BOARD_INVALID",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            BoardDbError::Read { .. } => {
                "Check that the board profile path exists and is readable."
            }
            BoardDbError::Parse { .. } => "Fix the TOML syntax and field types in af-board.toml.",
            BoardDbError::Validation { .. } => "Fix the listed board profile issues.",
        }
    }

    pub fn exit_code(&self) -> i32 {
        2
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BoardIssue {
    pub code: String,
    pub message: String,
    pub hint: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BoardProfile {
    #[serde(default)]
    pub schema_version: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub fpga: String,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub pins: Vec<BoardPin>,
    #[serde(default)]
    pub resources: Vec<BoardResource>,
    #[serde(default)]
    pub caveats: Vec<BoardCaveat>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BoardPin {
    pub name: String,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub function: Option<String>,
    #[serde(default)]
    pub verified: Option<bool>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BoardResource {
    pub name: String,
    #[serde(default)]
    pub count: Option<u32>,
    #[serde(default)]
    pub verified: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BoardCaveat {
    pub id: String,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BoardAlias {
    pub alias: String,
    pub canonical: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BoardAliasesFile {
    #[serde(default)]
    pub aliases: Vec<BoardAlias>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RegistryBoardsFile {
    pub boards: Vec<BoardEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RegistryFamiliesFile {
    pub families: Vec<FamilyEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RegistryToolchainsFile {
    pub toolchains: Vec<ToolchainEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BoardEntry {
    pub board_id: String,
    pub display_name: String,
    pub vendor: String,
    pub fpga_family: String,
    pub fpga_part_if_known_or_template: String,
    pub logic_size_class: String,
    pub dsp_class: String,
    pub memory_class: String,
    pub high_speed_io_class: String,
    pub default_toolchain: String,
    #[serde(default)]
    pub alternative_toolchains: Vec<String>,
    pub constraint_format: String,
    pub board_dir: String,
    pub exact_pinout_status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision_source_locator: Option<String>,
    pub safe_for_beginner: bool,
    #[serde(default)]
    pub suggested_ip_classes: Vec<String>,
    #[serde(default)]
    pub excluded_ip_classes: Vec<String>,
    #[serde(default)]
    pub notes: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ToolchainEntry {
    pub id: String,
    pub display_name: String,
    pub command: String,
    #[serde(default)]
    pub constraint_formats: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct FamilyEntry {
    pub vendor: String,
    pub family: String,
    pub resources: serde_json::Value,
    #[serde(default)]
    pub supported_constraint_formats: Vec<String>,
    #[serde(default)]
    pub suggested_toolchains: Vec<String>,
    #[serde(default)]
    pub portability_notes: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RegistryCheckReport {
    pub valid: bool,
    pub board_count: usize,
    pub alias_count: usize,
    pub issues: Vec<BoardIssue>,
}

const CONSTRAINT_FILE_BY_FORMAT: &[(&str, &str)] = &[
    ("cst", "pins.cst"),
    ("pcf", "pins.pcf"),
    ("lpf", "constraints.lpf"),
    ("qsf", "project.qsf"),
    ("sdc", "timing.sdc"),
    ("xdc", "constraints.xdc"),
    ("pdc", "constraints.pdc"),
];

impl BoardProfile {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, BoardDbError> {
        let path = path.as_ref();
        let raw = std::fs::read_to_string(path).map_err(|err| BoardDbError::Read {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?;
        let mut value: Value = toml::from_str(&raw).map_err(|err| BoardDbError::Parse {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?;
        normalize_board_value(&mut value);
        let profile: Self =
            value
                .try_into()
                .map_err(|err: toml::de::Error| BoardDbError::Parse {
                    path: path.to_path_buf(),
                    message: err.to_string(),
                })?;
        let issues = profile.validate();
        if issues.is_empty() {
            Ok(profile)
        } else {
            Err(BoardDbError::Validation { issues })
        }
    }

    pub fn validate(&self) -> Vec<BoardIssue> {
        let mut issues = Vec::new();
        if let Some(kind) = &self.kind {
            if kind != "accelfury.board" {
                issues.push(issue(
                    "AF_BOARD_KIND_INVALID",
                    format!("unsupported board kind `{kind}`"),
                    "Use kind = \"accelfury.board\" for board manifests.",
                ));
            }
        }
        for pin in &self.pins {
            if pin.location.is_some() && pin.verified.is_none() {
                issues.push(BoardIssue {
                    code: "AF_BOARD_PIN_VERIFICATION_MISSING".to_string(),
                    message: format!(
                        "pin `{}` has a location claim without explicit verified flag",
                        pin.name
                    ),
                    hint: "Set verified = true with evidence, or verified = false for unconfirmed claims."
                        .to_string(),
                });
            }
        }
        for resource in &self.resources {
            if resource.verified.is_none() {
                issues.push(issue(
                    "AF_BOARD_RESOURCE_VERIFICATION_MISSING",
                    format!("resource `{}` lacks explicit verified flag", resource.name),
                    "Set verified = true with evidence, or verified = false for unconfirmed resources.",
                ));
            }
        }
        issues
    }
}

pub fn list_boards(root: impl AsRef<Path>) -> Result<Vec<BoardEntry>, BoardDbError> {
    load_registry_boards(root.as_ref().join("registries/boards.registry.json"))
}

pub fn check_board_profile(path: impl AsRef<Path>) -> Result<BoardProfile, BoardDbError> {
    BoardProfile::from_path(path)
}

pub fn load_registry_boards(path: impl AsRef<Path>) -> Result<Vec<BoardEntry>, BoardDbError> {
    let file: RegistryBoardsFile = read_json(path.as_ref())?;
    Ok(file.boards)
}

pub fn load_board_aliases(path: impl AsRef<Path>) -> Result<Vec<BoardAlias>, BoardDbError> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file: BoardAliasesFile = read_json(path)?;
    Ok(file.aliases)
}

pub fn resolve_board_id<'a>(board_id: &'a str, aliases: &'a [BoardAlias]) -> &'a str {
    aliases
        .iter()
        .find(|alias| alias.alias == board_id)
        .map(|alias| alias.canonical.as_str())
        .unwrap_or(board_id)
}

pub fn check_registry(root: impl AsRef<Path>) -> Result<RegistryCheckReport, BoardDbError> {
    let root = root.as_ref();
    let boards = load_registry_boards(root.join("registries/boards.registry.json"))?;
    let aliases = load_board_aliases(root.join("registries/board_aliases.json"))?;
    let mut issues = Vec::new();
    let mut ids = BTreeSet::new();

    for board in &boards {
        if !ids.insert(board.board_id.clone()) {
            issues.push(issue(
                "AF_BOARD_DUPLICATE_ID",
                format!("duplicate board id `{}`", board.board_id),
                "Keep one canonical entry per board id.",
            ));
        }
        validate_board_entry(root, board, &mut issues);
    }

    let board_ids: BTreeSet<&str> = boards.iter().map(|board| board.board_id.as_str()).collect();
    let mut alias_ids = BTreeSet::new();
    for alias in &aliases {
        if !alias_ids.insert(alias.alias.as_str()) {
            issues.push(issue(
                "AF_BOARD_ALIAS_DUPLICATE",
                format!("duplicate board alias `{}`", alias.alias),
                "Keep one alias entry per legacy id.",
            ));
        }
        if board_ids.contains(alias.alias.as_str()) {
            issues.push(issue(
                "AF_BOARD_ALIAS_COLLIDES",
                format!("alias `{}` collides with a canonical board id", alias.alias),
                "Rename or remove the alias so canonical ids remain unambiguous.",
            ));
        }
        if !board_ids.contains(alias.canonical.as_str()) {
            issues.push(issue(
                "AF_BOARD_ALIAS_TARGET_UNKNOWN",
                format!(
                    "alias `{}` targets unknown canonical board `{}`",
                    alias.alias, alias.canonical
                ),
                "Point aliases only at boards present in registries/boards.registry.json.",
            ));
        }
    }

    Ok(RegistryCheckReport {
        valid: issues.is_empty(),
        board_count: boards.len(),
        alias_count: aliases.len(),
        issues,
    })
}

pub fn render_board_matrix(root: impl AsRef<Path>) -> Result<String, BoardDbError> {
    let boards = load_registry_boards(root.as_ref().join("registries/boards.registry.json"))?;
    let mut out = String::from("# AccelFury Board Matrix\n\n");
    out.push_str("Generated from `registries/boards.registry.json` by `af board matrix`.\n\n");
    out.push_str("| Board | Vendor | Family | Toolchain | Constraints | Status |\n");
    out.push_str("| --- | --- | --- | --- | --- | --- |\n");
    for board in boards {
        out.push_str(&format!(
            "| `{}` | {} | `{}` | `{}` | `{}` | {} |\n",
            board.board_id,
            board.vendor,
            board.fpga_family,
            board.default_toolchain,
            board.constraint_format,
            board.exact_pinout_status
        ));
    }
    Ok(out)
}

fn validate_board_entry(root: &Path, board: &BoardEntry, issues: &mut Vec<BoardIssue>) {
    if board.board_id.trim().is_empty() {
        issues.push(issue(
            "AF_BOARD_ID_EMPTY",
            "board id must not be empty",
            "Provide a stable canonical board id.",
        ));
    }
    let dir = root.join(&board.board_dir);
    require_dir(&dir, "AF_BOARD_DIR_MISSING", issues);
    require_file(&dir.join("README.md"), "AF_BOARD_README_MISSING", issues);
    require_file(&dir.join("bringup.md"), "AF_BOARD_BRINGUP_MISSING", issues);
    require_file(
        &dir.join("board.status.json"),
        "AF_BOARD_STATUS_MISSING",
        issues,
    );
    require_dir(
        &dir.join("constraints"),
        "AF_BOARD_CONSTRAINT_DIR_MISSING",
        issues,
    );
    require_file(
        &dir.join("constraints/README.md"),
        "AF_BOARD_CONSTRAINT_README_MISSING",
        issues,
    );

    match constraint_file_name(&board.constraint_format) {
        Some(file) => require_file(
            &dir.join("constraints").join(file),
            "AF_BOARD_CONSTRAINT_FILE_MISSING",
            issues,
        ),
        None => issues.push(issue(
            "AF_BOARD_CONSTRAINT_FORMAT_UNSUPPORTED",
            format!(
                "board `{}` uses unsupported constraint format `{}`",
                board.board_id, board.constraint_format
            ),
            "Use one of the registered constraint formats.",
        )),
    }

    let has_sv_top = dir.join("top/af_board_top.sv").is_file();
    let has_v_top = dir.join("top/af_board_top.v").is_file();
    if !has_sv_top && !has_v_top {
        issues.push(issue(
            "AF_BOARD_TOP_MISSING",
            format!("board `{}` has no top wrapper", board.board_id),
            "Add top/af_board_top.sv or top/af_board_top.v.",
        ));
    }
}

fn read_json<T>(path: &Path) -> Result<T, BoardDbError>
where
    T: for<'de> Deserialize<'de>,
{
    let raw = fs::read_to_string(path).map_err(|err| BoardDbError::Read {
        path: path.to_path_buf(),
        message: err.to_string(),
    })?;
    serde_json::from_str(&raw).map_err(|err| BoardDbError::Parse {
        path: path.to_path_buf(),
        message: err.to_string(),
    })
}

fn constraint_file_name(format: &str) -> Option<&'static str> {
    CONSTRAINT_FILE_BY_FORMAT
        .iter()
        .find(|(candidate, _)| *candidate == format)
        .map(|(_, file)| *file)
}

fn require_file(path: &Path, code: &str, issues: &mut Vec<BoardIssue>) {
    if !path.is_file() {
        issues.push(issue(
            code,
            format!("missing required file `{}`", path.display()),
            "Restore the board template file or remove the registry entry.",
        ));
    }
}

fn require_dir(path: &Path, code: &str, issues: &mut Vec<BoardIssue>) {
    if !path.is_dir() {
        issues.push(issue(
            code,
            format!("missing required directory `{}`", path.display()),
            "Restore the board template directory or remove the registry entry.",
        ));
    }
}

fn issue(code: &str, message: impl Into<String>, hint: impl Into<String>) -> BoardIssue {
    BoardIssue {
        code: code.to_string(),
        message: message.into(),
        hint: hint.into(),
    }
}

fn normalize_board_value(value: &mut Value) {
    let Some(table) = value.as_table_mut() else {
        return;
    };
    if let Some(Value::String(schema_version)) = table.get("schema_version").cloned() {
        table
            .entry("schema_version".to_string())
            .or_insert(Value::String(schema_version));
    }
    if let Some(Value::Table(name)) = table.get("name").cloned() {
        if let Some(id) = name.get("id").cloned() {
            table.entry("id".to_string()).or_insert(id);
        }
        if let Some(display_name) = name.get("display_name").cloned() {
            if !matches!(table.get("name"), Some(Value::String(_))) {
                table.insert("name".to_string(), display_name);
            }
        }
    }
    if let Some(Value::Table(fpga)) = table.get("fpga").cloned() {
        if let Some(vendor) = fpga.get("vendor").cloned() {
            table.entry("vendor".to_string()).or_insert(vendor);
        }
        let family = fpga
            .get("family")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let part = fpga.get("part").and_then(Value::as_str).unwrap_or(family);
        if !matches!(table.get("fpga"), Some(Value::String(_))) {
            table.insert(
                "fpga".to_string(),
                Value::String(format!("{family} {part}")),
            );
        }
    }
    normalize_clock_resource(table);
}

fn normalize_clock_resource(table: &mut Map<String, Value>) {
    let Some(Value::Table(clock)) = table.get("clock").cloned() else {
        return;
    };
    let Some(Value::Table(default)) = clock.get("default").cloned() else {
        return;
    };
    let Some(name) = default.get("name").and_then(Value::as_str) else {
        return;
    };
    let verified = default
        .get("verified")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut resource = Map::new();
    resource.insert("name".to_string(), Value::String(name.to_string()));
    resource.insert("count".to_string(), Value::Integer(1));
    resource.insert("verified".to_string(), Value::Boolean(verified));
    let resources = table
        .entry("resources".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Value::Array(resources) = resources {
        resources.push(Value::Table(resource));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_pin_verification_flag() {
        let profile = BoardProfile {
            schema_version: None,
            kind: None,
            id: "demo".to_string(),
            name: "Demo".to_string(),
            vendor: "Demo".to_string(),
            fpga: "FPGA".to_string(),
            notes: Vec::new(),
            pins: vec![BoardPin {
                name: "LED0".to_string(),
                location: Some("A1".to_string()),
                function: Some("led".to_string()),
                verified: None,
                source: None,
            }],
            resources: Vec::new(),
            caveats: Vec::new(),
        };
        let issues = profile.validate();
        assert_eq!(issues[0].code, "AF_BOARD_PIN_VERIFICATION_MISSING");
    }

    #[test]
    fn mvp_tang_profiles_are_valid() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("workspace root");
        for relative in [
            "boards/tang-nano-20k/af-board.toml",
            "boards/tang-primer-20k/af-board.toml",
        ] {
            let profile = BoardProfile::from_path(root.join(relative)).expect(relative);
            assert!(profile
                .pins
                .iter()
                .all(|pin| pin.location.is_none() || pin.verified == Some(false)));
        }
    }
}
