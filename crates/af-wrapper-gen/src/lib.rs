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
    StreamFifo,
}

impl WrapperTarget {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FuseSoc => "fusesoc",
            Self::LiteX => "litex",
            Self::IpXact => "ipxact",
            Self::StreamFifo => "stream-fifo",
        }
    }

    pub fn parse(input: &str) -> Result<Self, WrapperGenError> {
        match input {
            "fusesoc" => Ok(Self::FuseSoc),
            "litex" => Ok(Self::LiteX),
            "ipxact" => Ok(Self::IpXact),
            "stream-fifo" => Ok(Self::StreamFifo),
            other => Err(WrapperGenError::UnsupportedTarget {
                target: other.to_string(),
            }),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct WrapperTargetCapability {
    pub target: String,
    pub adapter_kind: Option<String>,
    pub generates_rtl: bool,
    pub status: String,
    pub limitations: Vec<String>,
}

pub fn wrapper_target_capabilities() -> Vec<WrapperTargetCapability> {
    vec![
        WrapperTargetCapability {
            target: WrapperTarget::FuseSoc.as_str().to_string(),
            adapter_kind: None,
            generates_rtl: false,
            status: "supported".to_string(),
            limitations: vec![
                "package metadata only; does not add protocol glue".to_string(),
            ],
        },
        WrapperTargetCapability {
            target: WrapperTarget::LiteX.as_str().to_string(),
            adapter_kind: None,
            generates_rtl: false,
            status: "supported".to_string(),
            limitations: vec![
                "reference skeleton only; does not add bus bridges or CDC".to_string(),
            ],
        },
        WrapperTargetCapability {
            target: WrapperTarget::IpXact.as_str().to_string(),
            adapter_kind: None,
            generates_rtl: false,
            status: "supported".to_string(),
            limitations: vec![
                "metadata skeleton only; bus interfaces remain external".to_string(),
            ],
        },
        WrapperTargetCapability {
            target: WrapperTarget::StreamFifo.as_str().to_string(),
            adapter_kind: Some("stream_fifo_adapter".to_string()),
            generates_rtl: true,
            status: "supported".to_string(),
            limitations: vec![
                "known FIFO control to ready/valid adapter only; no CDC, width conversion, AXI, or vendor primitives".to_string(),
            ],
        },
    ]
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
                "Use --target fusesoc, --target litex, --target ipxact, or --target stream-fifo."
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
        WrapperTarget::StreamFifo => {
            let output_dir = build_root.join("stream-fifo");
            fs::create_dir_all(&output_dir).map_err(|err| WrapperGenError::Write {
                path: output_dir.clone(),
                message: err.to_string(),
            })?;
            let wrapper = generate_stream_fifo_wrapper(&core_report.manifest);
            let output = safe_join(&output_dir, &wrapper.file_name)?;
            fs::write(&output, &wrapper.content).map_err(|err| WrapperGenError::Write {
                path: output.clone(),
                message: err.to_string(),
            })?;
            let mut warnings = core_report.warnings;
            warnings.extend(wrapper.warnings);
            let mut limitations = core_report.limitations;
            limitations.extend(wrapper.limitations);
            Ok(WrapperReport {
                target,
                artifacts: vec![output],
                warnings,
                limitations,
            })
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct StreamFifoWrapper {
    pub file_name: String,
    pub content: String,
    pub warnings: Vec<String>,
    pub limitations: Vec<String>,
}

pub fn generate_stream_fifo_wrapper(manifest: &CoreManifest) -> StreamFifoWrapper {
    let wrapper_module = format!(
        "{}_stream_fifo",
        sanitize_verilog_identifier(&manifest.core)
    );
    let fifo_module = sanitize_verilog_identifier(&manifest.rtl.top);
    let file_name = format!("{wrapper_module}.v");
    let full_write_policy = manifest
        .contracts
        .fifo
        .as_ref()
        .and_then(|fifo| fifo.full_write_policy.as_deref())
        .unwrap_or("reject_when_full");
    let ready_expr = if matches!(
        full_write_policy,
        "accept_when_full_with_read" | "allow_when_same_cycle_read"
    ) {
        "!fifo_full_w || fifo_rd_en_w"
    } else {
        "!fifo_full_w"
    };
    let parameters = render_stream_fifo_parameters(manifest);
    let parameter_overrides = render_stream_fifo_parameter_overrides(manifest);
    let mut warnings = Vec::new();
    if manifest.contracts.fifo.is_none() {
        warnings.push(
            "AF_WRAPPER_STREAM_FIFO_CONTRACT_MISSING: generated wrapper uses conventional af_sync_fifo-style ports because [contracts.fifo] is absent."
                .to_string(),
        );
    }
    let content = format!(
        r#"`default_nettype none
// Generated by AccelFury IP Toolchain
// Ready/valid adapter around raw FIFO control ports. Handwritten RTL remains
// the source of the FIFO; this wrapper contains only protocol mapping.

module {wrapper_module} #(
{parameters}
) (
    input  wire                 clk,
    input  wire                 rst,
    input  wire                 clear,

    input  wire                 s_valid,
    output wire                 s_ready,
    input  wire [DATA_BITS-1:0] s_data,

    output wire                 m_valid,
    input  wire                 m_ready,
    output wire [DATA_BITS-1:0] m_data
);

    wire fifo_full_w;
    wire fifo_empty_w;
    wire fifo_almost_full_unused_w;
    wire [FIFO_ADDR_BITS:0] fifo_level_unused_w;
    wire fifo_rd_en_w;
    wire fifo_wr_en_w;

    assign m_valid      = !fifo_empty_w;
    assign fifo_rd_en_w = m_ready && !fifo_empty_w;
    assign s_ready      = {ready_expr};
    assign fifo_wr_en_w = s_valid && s_ready;

    {fifo_module} #(
{parameter_overrides}
    ) u_fifo (
        .clk(clk),
        .rst(rst),
        .clear(clear),
        .wr_en(fifo_wr_en_w),
        .wr_data(s_data),
        .full(fifo_full_w),
        .almost_full(fifo_almost_full_unused_w),
        .rd_en(fifo_rd_en_w),
        .rd_data(m_data),
        .empty(fifo_empty_w),
        .level(fifo_level_unused_w)
    );

endmodule

`default_nettype wire
"#
    );
    StreamFifoWrapper {
        file_name,
        content,
        warnings,
        limitations: vec![
            "Stream FIFO wrapper is a generated protocol adapter; it does not add CDC, AXI, bus bridging, timing constraints, or vendor primitives."
                .to_string(),
        ],
    }
}

fn render_stream_fifo_parameters(manifest: &CoreManifest) -> String {
    let mut lines = Vec::new();
    for parameter in &manifest.parameters {
        lines.push(format!(
            "    parameter {} = {}",
            sanitize_verilog_identifier(&parameter.name),
            parameter.value
        ));
    }
    if lines.is_empty() {
        lines.push("    parameter DATA_BITS = 32".to_string());
        lines.push("    parameter FIFO_ADDR_BITS = 4".to_string());
        lines.push("    parameter ALMOST_FULL_TH = (1 << FIFO_ADDR_BITS) - 2".to_string());
    }
    lines.join(",\n")
}

fn render_stream_fifo_parameter_overrides(manifest: &CoreManifest) -> String {
    let names: Vec<String> = if manifest.parameters.is_empty() {
        vec![
            "DATA_BITS".to_string(),
            "FIFO_ADDR_BITS".to_string(),
            "ALMOST_FULL_TH".to_string(),
        ]
    } else {
        manifest
            .parameters
            .iter()
            .map(|parameter| sanitize_verilog_identifier(&parameter.name))
            .collect()
    };
    names
        .iter()
        .enumerate()
        .map(|(idx, name)| {
            let comma = if idx + 1 == names.len() { "" } else { "," };
            format!("        .{name}({name}){comma}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn generate_ipxact_skeleton(manifest: &CoreManifest, board: Option<&str>) -> IpXactSkeleton {
    let board = board.unwrap_or("unbound-board");
    let file_name = format!(
        "{}_{}_{}.xml",
        sanitize_xml_identifier(&manifest.vendor),
        sanitize_xml_identifier(&manifest.library),
        sanitize_xml_identifier(&manifest.core),
    );

    let mut bus_interfaces = String::new();
    for interface in &manifest.interfaces {
        bus_interfaces.push_str(&format!(
            "    <ipxact:busInterface>\n      <ipxact:name>{}</ipxact:name>\n      <ipxact:description>Manifest interface kind: {}</ipxact:description>\n    </ipxact:busInterface>\n",
            xml_escape(&interface.name),
            xml_escape(&interface.kind)
        ));
    }
    if bus_interfaces.is_empty() {
        bus_interfaces.push_str(
            "    <ipxact:busInterface>\n      <ipxact:name>ready_valid</ipxact:name>\n      <ipxact:description>Conventional ready/valid interface placeholder.</ipxact:description>\n    </ipxact:busInterface>\n",
        );
    }

    let mut ports = String::new();
    for port in &manifest.ports {
        ports.push_str(&format!(
            "      <ipxact:port>\n        <ipxact:name>{}</ipxact:name>\n        <ipxact:wire>\n          <ipxact:direction>{}</ipxact:direction>\n        </ipxact:wire>\n      </ipxact:port>\n",
            xml_escape(&port.name),
            xml_escape(&port.direction.to_lowercase())
        ));
    }
    if ports.is_empty() {
        ports.push_str("      <ipxact:port/>\n");
    }

    let mut files = String::new();
    for file in &manifest.sources.files {
        files.push_str(&format!(
            "      <ipxact:file>\n        <ipxact:name>{}</ipxact:name>\n        <ipxact:fileType>verilogSource</ipxact:fileType>\n      </ipxact:file>\n",
            xml_escape(file)
        ));
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
<ipxact:component xmlns:ipxact="http://www.accellera.org/XMLSchema/IPXACT/1685-2022" xmlns:af="https://accelfury.dev/ipxact/vendor-extensions/1.0">
  <ipxact:vendor>{vendor}</ipxact:vendor>
  <ipxact:library>{library}</ipxact:library>
  <ipxact:name>{core}</ipxact:name>
  <ipxact:version>{version}</ipxact:version>
  <ipxact:description>IEEE 1685-2022 IP-XACT component skeleton generated by AccelFury wrapper generator.</ipxact:description>
  <ipxact:busInterfaces>
{bus_interfaces}  </ipxact:busInterfaces>
  <ipxact:model>
    <ipxact:views>
      <ipxact:view>
        <ipxact:name>rtl</ipxact:name>
        <ipxact:envIdentifier>accelfury:rtl</ipxact:envIdentifier>
        <ipxact:modelName>{top}</ipxact:modelName>
        <ipxact:fileSetRef>
          <ipxact:localName>{core}_files</ipxact:localName>
        </ipxact:fileSetRef>
      </ipxact:view>
    </ipxact:views>
    <ipxact:ports>
{ports}    </ipxact:ports>
  </ipxact:model>
  <ipxact:fileSets>
    <ipxact:fileSet>
      <ipxact:name>{core}_files</ipxact:name>
{files}    </ipxact:fileSet>
  </ipxact:fileSets>
  <ipxact:vendorExtensions>
    <af:boardTarget>{board}</af:boardTarget>
    <af:topSource>{top_source}</af:topSource>
    <af:portabilityTier>{tier}</af:portabilityTier>
  </ipxact:vendorExtensions>
</ipxact:component>
"#,
        vendor = xml_escape(&manifest.vendor),
        library = xml_escape(&manifest.library),
        core = xml_escape(&manifest.core),
        version = xml_escape(&manifest.version),
        bus_interfaces = bus_interfaces,
        ports = ports,
        files = files,
        top = xml_escape(&manifest.rtl.top),
        top_source = xml_escape(top_source),
        board = xml_escape(board),
        tier = xml_escape(
            &manifest
                .portability_level
                .as_ref()
                .map(|level| format!("{level:?}"))
                .unwrap_or_else(|| "Unspecified".to_string())
        ),
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

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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

fn sanitize_verilog_identifier(input: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in input.chars().enumerate() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            if idx == 0 && ch.is_ascii_digit() {
                out.push('_');
            }
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "af_wrapper".to_string()
    } else {
        out
    }
}
