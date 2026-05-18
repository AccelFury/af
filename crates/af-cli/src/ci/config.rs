// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CiProjectConfig {
    pub name: String,
    pub hdl: String,
    #[serde(default = "default_ci_provider")]
    pub ci_provider: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CiPathsConfig {
    #[serde(default = "default_paths_rtl")]
    pub rtl: Vec<String>,
    #[serde(default = "default_tb_paths")]
    pub tb: Vec<String>,
    #[serde(default = "default_sim_paths")]
    pub sim: Vec<String>,
    #[serde(default = "default_formal_paths")]
    pub formal: Vec<String>,
    #[serde(default = "default_board_paths")]
    pub boards: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CiCoreConfig {
    pub top: String,
    #[serde(default = "default_include_dirs")]
    pub include_dirs: Vec<String>,
    #[serde(default = "default_source_globs")]
    pub source_globs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CiSimulationConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_sim_kind")]
    pub kind: String,
    #[serde(default = "default_sim_command")]
    pub command: String,
    #[serde(default = "default_pass_pattern")]
    pub pass_pattern: String,
    #[serde(default = "default_fail_pattern")]
    pub fail_pattern: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CiYosysConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_yosys_mode")]
    pub mode: String,
    #[serde(default = "default_yosys_family")]
    pub family: String,
    #[serde(default)]
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CiArtifactsConfig {
    #[serde(default = "default_artifact_root")]
    pub root: String,
    #[serde(default = "default_true")]
    pub generate_sha256sums: bool,
    #[serde(default = "default_true")]
    pub store_tool_versions: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CiPolicyConfig {
    #[serde(default = "default_true")]
    pub fail_closed: bool,
    #[serde(default = "default_true")]
    pub no_vendor_tools_in_public_ci: bool,
    #[serde(default = "default_true")]
    pub artifact_allowlist_only: bool,
    #[serde(default = "default_true")]
    pub no_unknown_script_execution: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CiBoardConfig {
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub family: String,
    pub top: String,
    pub device: String,
    #[serde(default)]
    pub nextpnr_family: String,
    #[serde(default)]
    pub pack_device: String,
    pub constraints: String,
    #[serde(default)]
    pub source_globs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CiConfig {
    pub project: CiProjectConfig,
    #[serde(default)]
    pub paths: CiPathsConfig,
    pub core: CiCoreConfig,
    #[serde(default)]
    pub simulation: CiSimulationConfig,
    #[serde(default)]
    pub yosys: CiYosysConfig,
    #[serde(default)]
    pub artifacts: CiArtifactsConfig,
    #[serde(default)]
    pub policy: CiPolicyConfig,
    #[serde(default)]
    pub boards: Vec<CiBoardConfig>,
}

#[derive(Debug)]
pub struct ConfigBuilder {
    pub project: String,
    pub hdl: String,
    pub rtl_path: String,
    pub top: Option<String>,
    pub ci_provider: String,
    pub sim_command: Option<String>,
    pub make_test_detected: bool,
}

impl ConfigBuilder {
    pub fn build(self) -> CiConfig {
        let top = self.top.unwrap_or_else(|| "core_top".to_string());
        CiConfig {
            project: CiProjectConfig {
                name: self.project,
                hdl: self.hdl,
                ci_provider: if self.ci_provider.is_empty() {
                    default_ci_provider()
                } else {
                    self.ci_provider
                },
            },
            paths: CiPathsConfig::default(),
            core: CiCoreConfig {
                top,
                include_dirs: vec![self.rtl_path.clone()],
                source_globs: vec![
                    format!("{}/**/*.v", self.rtl_path),
                    format!("{}/**/*.sv", self.rtl_path),
                    format!("{}/**/*.vhd", self.rtl_path),
                    format!("{}/**/*.vhdl", self.rtl_path),
                ],
            },
            simulation: CiSimulationConfig {
                enabled: self.make_test_detected,
                kind: default_sim_kind(),
                command: self.sim_command.unwrap_or_else(default_sim_command),
                pass_pattern: default_pass_pattern(),
                fail_pattern: default_fail_pattern(),
            },
            yosys: CiYosysConfig::default(),
            artifacts: CiArtifactsConfig::default(),
            policy: CiPolicyConfig::default(),
            boards: Vec::new(),
        }
    }
}

impl CiConfig {
    pub fn update_top(&mut self, top: impl Into<String>) {
        self.core.top = top.into();
    }

    pub fn artifacts_root(&self) -> &str {
        &self.artifacts.root
    }

    pub fn add_or_replace_board(&mut self, board: CiBoardConfig) {
        if let Some(existing) = self
            .boards
            .iter_mut()
            .find(|entry| entry.name == board.name)
        {
            *existing = board;
        } else {
            self.boards.push(board);
        }
    }

    pub fn has_board(&self, name: &str) -> bool {
        self.boards.iter().any(|board| board.name == name)
    }

    pub fn sorted_artifacts_root(&self) -> &str {
        &self.artifacts.root
    }
}

impl Default for CiPathsConfig {
    fn default() -> Self {
        Self {
            rtl: default_paths_rtl(),
            tb: default_tb_paths(),
            sim: default_sim_paths(),
            formal: default_formal_paths(),
            boards: default_board_paths(),
        }
    }
}

impl Default for CiSimulationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            kind: default_sim_kind(),
            command: default_sim_command(),
            pass_pattern: default_pass_pattern(),
            fail_pattern: default_fail_pattern(),
        }
    }
}

impl Default for CiYosysConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: default_yosys_mode(),
            family: default_yosys_family(),
            extra_args: Vec::new(),
        }
    }
}

impl Default for CiArtifactsConfig {
    fn default() -> Self {
        Self {
            root: default_artifact_root(),
            generate_sha256sums: true,
            store_tool_versions: true,
        }
    }
}

impl Default for CiPolicyConfig {
    fn default() -> Self {
        Self {
            fail_closed: true,
            no_vendor_tools_in_public_ci: true,
            artifact_allowlist_only: true,
            no_unknown_script_execution: true,
        }
    }
}

impl Default for CiConfig {
    fn default() -> Self {
        Self {
            project: CiProjectConfig {
                name: "af_project".to_string(),
                hdl: "verilog-2001".to_string(),
                ci_provider: default_ci_provider(),
            },
            paths: CiPathsConfig::default(),
            core: CiCoreConfig {
                top: "core_top".to_string(),
                include_dirs: vec!["rtl".to_string()],
                source_globs: vec!["rtl/**/*.v".to_string(), "rtl/**/*.sv".to_string()],
            },
            simulation: CiSimulationConfig::default(),
            yosys: CiYosysConfig::default(),
            artifacts: CiArtifactsConfig::default(),
            policy: CiPolicyConfig::default(),
            boards: Vec::new(),
        }
    }
}

pub fn from_toml_str(text: &str) -> Result<CiConfig, String> {
    toml::from_str(text).map_err(|err| format!("failed to parse af-ci config: {err}"))
}

pub fn from_toml_file(path: &Path) -> Result<CiConfig, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read `{}`: {err}", path.display()))?;
    from_toml_str(&text)
}

pub fn to_toml_string(config: &CiConfig) -> Result<String, String> {
    let body = toml::to_string_pretty(config)
        .map_err(|err| format!("failed to serialize af-ci config: {err}"))?;
    Ok(format!("# Generated by AccelFury IP Toolchain\n{body}"))
}

fn default_true() -> bool {
    true
}

fn default_ci_provider() -> String {
    "github".into()
}

fn default_paths_rtl() -> Vec<String> {
    vec!["rtl".into()]
}

fn default_tb_paths() -> Vec<String> {
    vec!["tb".into()]
}

fn default_sim_paths() -> Vec<String> {
    vec!["sim".into()]
}

fn default_formal_paths() -> Vec<String> {
    vec!["formal".into()]
}

fn default_board_paths() -> Vec<String> {
    vec!["boards".into()]
}

fn default_include_dirs() -> Vec<String> {
    vec!["rtl".into()]
}

fn default_source_globs() -> Vec<String> {
    vec![
        "rtl/**/*.v".to_string(),
        "rtl/**/*.sv".to_string(),
        "rtl/**/*.vhd".to_string(),
        "rtl/**/*.vhdl".to_string(),
    ]
}

fn default_sim_kind() -> String {
    "make".into()
}

fn default_sim_command() -> String {
    "cd sim && make test".into()
}

fn default_pass_pattern() -> String {
    "^PASS ".into()
}

fn default_fail_pattern() -> String {
    "^FAIL".into()
}

fn default_yosys_mode() -> String {
    "elab_and_synth_json".into()
}

fn default_yosys_family() -> String {
    "generic".into()
}

fn default_artifact_root() -> String {
    "artifacts/openfpga-ci".into()
}
