// SPDX-License-Identifier: Apache-2.0
use af_backend_fusesoc::{write_core, FuseSocError};
use af_backend_litex::generate_litex_skeleton;
use af_core::{check_core, CoreError};
use af_manifest::CoreManifest;
use af_security::{safe_join, SecurityError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum WrapperTarget {
    FuseSoc,
    LiteX,
    IpXact,
}

impl WrapperTarget {
    pub fn parse(input: &str) -> Result<Self, WrapperGenError> {
        match input {
            "fusesoc" => Ok(Self::FuseSoc),
            "litex" => Ok(Self::LiteX),
            "ipxact" => Ok(Self::IpXact),
            other => Err(WrapperGenError::UnsupportedTarget {
                target: other.to_string(),
            }),
        }
    }
}

#[derive(Debug, Error)]
pub enum WrapperGenError {
    #[error("unsupported wrapper target `{target}`")]
    UnsupportedTarget { target: String },
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error(transparent)]
    FuseSoc(#[from] FuseSocError),
    #[error(transparent)]
    Security(#[from] SecurityError),
    #[error("failed to write wrapper artifact `{path}`: {message}")]
    Write { path: PathBuf, message: String },
}

impl WrapperGenError {
    pub fn code(&self) -> &'static str {
        match self {
            WrapperGenError::UnsupportedTarget { .. } => "AF_WRAPPER_TARGET_UNSUPPORTED",
            WrapperGenError::Core(err) => err.code(),
            WrapperGenError::FuseSoc(err) => err.code(),
            WrapperGenError::Security(err) => err.code(),
            WrapperGenError::Write { .. } => "AF_WRAPPER_WRITE_FAILED",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            WrapperGenError::UnsupportedTarget { .. } => {
                "Use --target fusesoc, --target litex, or --target ipxact."
            }
            WrapperGenError::Core(err) => err.hint(),
            WrapperGenError::FuseSoc(err) => err.hint(),
            WrapperGenError::Security(err) => err.hint(),
            WrapperGenError::Write { .. } => {
                "Check filesystem permissions and the selected build root."
            }
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            WrapperGenError::UnsupportedTarget { .. } => 2,
            WrapperGenError::Core(err) => err.exit_code(),
            WrapperGenError::FuseSoc(err) => err.exit_code(),
            WrapperGenError::Security(err) => err.exit_code(),
            WrapperGenError::Write { .. } => 5,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct WrapperReport {
    pub target: WrapperTarget,
    pub artifacts: Vec<PathBuf>,
    pub warnings: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct IpXactSkeleton {
    pub file_name: String,
    pub content: String,
    pub warnings: Vec<String>,
    pub limitations: Vec<String>,
}

pub fn generate_wrapper(
    core_dir: impl AsRef<Path>,
    build_root: impl AsRef<Path>,
    target: WrapperTarget,
    board: Option<&str>,
) -> Result<WrapperReport, WrapperGenError> {
    let core_report = check_core(core_dir)?;
    let build_root = build_root.as_ref();
    match target {
        WrapperTarget::FuseSoc => {
            let output_dir = build_root.join("fusesoc");
            let artifact = write_core(&core_report.manifest, output_dir)?;
            Ok(WrapperReport {
                target,
                artifacts: vec![artifact.path],
                warnings: core_report.warnings,
                limitations: core_report.limitations,
            })
        }
        WrapperTarget::LiteX => {
            let output_dir = build_root.join("litex");
            fs::create_dir_all(&output_dir).map_err(|err| WrapperGenError::Write {
                path: output_dir.clone(),
                message: err.to_string(),
            })?;
            let skeleton = generate_litex_skeleton(&core_report.manifest, board);
            let output = safe_join(&output_dir, &skeleton.file_name)?;
            fs::write(&output, &skeleton.content).map_err(|err| WrapperGenError::Write {
                path: output.clone(),
                message: err.to_string(),
            })?;
            let mut warnings = core_report.warnings;
            warnings.extend(skeleton.warnings);
            let mut limitations = core_report.limitations;
            limitations.extend(skeleton.limitations);
            Ok(WrapperReport {
                target,
                artifacts: vec![output],
                warnings,
                limitations,
            })
        }
        WrapperTarget::IpXact => {
            let output_dir = build_root.join("ipxact");
            fs::create_dir_all(&output_dir).map_err(|err| WrapperGenError::Write {
                path: output_dir.clone(),
                message: err.to_string(),
            })?;
            let skeleton = generate_ipxact_skeleton(&core_report.manifest, board);
            let output = safe_join(&output_dir, &skeleton.file_name)?;
            fs::write(&output, &skeleton.content).map_err(|err| WrapperGenError::Write {
                path: output.clone(),
                message: err.to_string(),
            })?;
            let mut warnings = core_report.warnings;
            warnings.extend(skeleton.warnings);
            let mut limitations = core_report.limitations;
            limitations.extend(skeleton.limitations);
            Ok(WrapperReport {
                target,
                artifacts: vec![output],
                warnings,
                limitations,
            })
        }
    }
}

pub fn generate_ipxact_skeleton(manifest: &CoreManifest, board: Option<&str>) -> IpXactSkeleton {
    let board = board.unwrap_or("unbound-board");
    let file_name = format!(
        "{}_{}_{}.xml",
        sanitize_xml_identifier(&manifest.vendor),
        sanitize_xml_identifier(&manifest.library),
        sanitize_xml_identifier(&manifest.core),
    );

    let mut ports = String::new();
    for port in &manifest.ports {
        ports.push_str(&format!(
            "    <spirit:busInterface>\n      <spirit:name>{}</spirit:name>\n      <spirit:description>Port {}</spirit:description>\n      <spirit:portName>{}</spirit:portName>\n      <spirit:wire>\n        <spirit:direction>{}</spirit:direction>\n      </spirit:wire>\n    </spirit:busInterface>\n",
            port.name,
            port.name,
            port.name,
            port.direction.to_lowercase()
        ));
    }
    if ports.is_empty() {
        ports.push_str("    <spirit:busInterface/>\n");
    }

    let top_source = manifest
        .sources
        .files
        .first()
        .map(String::as_str)
        .unwrap_or("rtl/unbound.sv");

    let content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!-- Generated by AccelFury IP Toolchain -->
<spirit:component xmlns:spirit="http://www.spiritconsortium.org/XMLSchema/SPIRIT/1.5">
  <spirit:vendor>{vendor}</spirit:vendor>
  <spirit:library>{library}</spirit:library>
  <spirit:name>{core}</spirit:name>
  <spirit:version>{version}</spirit:version>
  <spirit:description>IP-XACT skeleton generated by AccelFury wrapper generator.</spirit:description>
  <spirit:busInterfaces>
{ports}  </spirit:busInterfaces>
  <spirit:model>
    <spirit:views>
      <spirit:view>
        <spirit:name>accelfury-default</spirit:name>
        <spirit:envIdentifier>RTL</spirit:envIdentifier>
        <spirit:modelName>{top}</spirit:modelName>
        <spirit:fileSetRef>
        <spirit:name>{core}_files</spirit:name>
        </spirit:fileSetRef>
      </spirit:view>
    </spirit:views>
  </spirit:model>
  <spirit:fileSets>
    <spirit:fileSet>
      <spirit:name>{core}_files</spirit:name>
      <spirit:file>
        <spirit:name>{top_source}</spirit:name>
      </spirit:file>
    </spirit:fileSet>
  </spirit:fileSets>
  <spirit:integration>
    <spirit:description>Board target: {board}</spirit:description>
  </spirit:integration>
</spirit:component>
"#,
        vendor = manifest.vendor,
        library = manifest.library,
        core = manifest.core,
        version = manifest.version,
        ports = ports,
        top = manifest.rtl.top,
        top_source = top_source,
        board = board,
    );

    IpXactSkeleton {
        file_name,
        content,
        warnings: vec!["IP-XACT output is a reference skeleton; validate and enrich metadata before signing.".
            to_string()],
        limitations: vec![
            "IP-XACT wrapper generation is adapter-level metadata only; it does not add bus bridges.".to_string(),
            "Complete bus interfaces (AXI/AHB/APB), file sets, and timing constraints in external vendor workflows.".to_string(),
        ],
    }
}

fn sanitize_xml_identifier(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "accelfury".to_string()
    } else {
        out
    }
}
