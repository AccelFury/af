// SPDX-License-Identifier: Apache-2.0
//
// Handlers for `af backend list` and `af backend scaffold`.
//
// `af backend run` stays in main.rs because it dispatches to the
// `core_lint` / `core_sim` / `core_formal` handlers that still live there.

use crate::{CliError, CliOutput};
use af_backend::{AfBackend, BackendCapability};
use af_backend_verilator::VerilatorBackend;
use af_core::load_validated_manifest;
use serde_json::json;
use std::path::Path;

pub fn backend_list() -> Result<CliOutput, CliError> {
    let mut capabilities: Vec<BackendCapability> = Vec::new();
    capabilities.extend(af_backend_native::capabilities());
    capabilities.extend(af_backend_icarus::capabilities());
    capabilities.extend(VerilatorBackend::process().capabilities());
    capabilities.push(BackendCapability {
        name: "fusesoc-package-export".to_string(),
        supported: true,
        detail: Some(
            "FuseSoC .core generation is deterministic and does not require executing FuseSoC."
                .to_string(),
        ),
    });
    capabilities.push(BackendCapability {
        name: "litex-wrapper-skeleton".to_string(),
        supported: true,
        detail: Some("LiteX skeleton/reference dry-run generation is available.".to_string()),
    });
    capabilities.push(BackendCapability {
        name: "ipxact-wrapper-skeleton".to_string(),
        supported: true,
        detail: Some(
            "IP-XACT skeleton component metadata generation is available for wrapper export."
                .to_string(),
        ),
    });
    capabilities.extend(af_backend_yosys::capabilities());
    capabilities.extend(af_backend_sby::capabilities());
    capabilities.extend(af_backend_nextpnr::capabilities());
    capabilities.extend(af_backend_flash::capabilities());
    capabilities.extend(af_backend_vendor::capabilities());
    Ok(CliOutput {
        human: capabilities
            .iter()
            .map(|capability| {
                format!(
                    "{}: {}",
                    capability.name,
                    if capability.supported {
                        "supported"
                    } else {
                        "planned"
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        json: json!({
            "status": "passed",
            "capabilities": capabilities,
        }),
    })
}

pub fn backend_scaffold(
    core_dir: &Path,
    vendor: &str,
    family: &str,
) -> Result<CliOutput, CliError> {
    // Refuse to scaffold vendor backends into a core whose manifest+RTL is
    // structurally invalid; otherwise we silently pollute a broken tree with
    // generated vendor files and report `passed`.
    if core_dir.join("af-core.toml").is_file() {
        load_validated_manifest(core_dir)?;
    }
    let report = af_template::scaffold_backend(core_dir, vendor, family)?;
    Ok(CliOutput {
        human: format!(
            "backend scaffold written: {}/vendor/{}",
            core_dir.display(),
            vendor
        ),
        json: json!(report),
    })
}
