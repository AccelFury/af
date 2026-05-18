// SPDX-License-Identifier: Apache-2.0
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct VendorTarget {
    pub vendor: String,
    pub family: Option<String>,
    pub board: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct VendorCapability {
    pub vendor: String,
    pub family: String,
    pub bram: bool,
    pub dsp: bool,
    pub pll: bool,
    pub hard_ip: Vec<String>,
    pub notes: Vec<String>,
}

pub fn resolve_target(
    vendor: Option<&str>,
    family: Option<&str>,
    board: Option<&str>,
) -> VendorTarget {
    if let Some(board) = board {
        if let Some((vendor, family)) = board_alias(board) {
            return VendorTarget {
                vendor: vendor.to_string(),
                family: Some(family.to_string()),
                board: Some(board.to_string()),
            };
        }
    }
    VendorTarget {
        vendor: vendor.unwrap_or("generic").to_string(),
        family: family.map(str::to_string),
        board: board.map(str::to_string),
    }
}

pub fn capability(target: &VendorTarget) -> VendorCapability {
    let family = target.family.as_deref().unwrap_or("generic");
    match target.vendor.as_str() {
        "xilinx" => VendorCapability {
            vendor: "xilinx".to_string(),
            family: family.to_string(),
            bram: true,
            dsp: true,
            pll: true,
            hard_ip: vec![
                "pcie".to_string(),
                "ddr".to_string(),
                "hbm_family_dependent".to_string(),
            ],
            notes: vec![
                "Offline capability registry; exact part limits require vendor tools.".to_string(),
            ],
        },
        "intel" => VendorCapability {
            vendor: "intel".to_string(),
            family: family.to_string(),
            bram: true,
            dsp: true,
            pll: true,
            hard_ip: vec![
                "pcie_family_dependent".to_string(),
                "ddr_family_dependent".to_string(),
            ],
            notes: vec![
                "Offline capability registry; exact part limits require Quartus reports."
                    .to_string(),
            ],
        },
        "gowin" => VendorCapability {
            vendor: "gowin".to_string(),
            family: family.to_string(),
            bram: true,
            dsp: true,
            pll: true,
            hard_ip: Vec::new(),
            notes: vec![
                "Offline capability registry; board-level limits are approximate.".to_string(),
            ],
        },
        "lattice" => VendorCapability {
            vendor: "lattice".to_string(),
            family: family.to_string(),
            bram: true,
            dsp: family.contains("ecp5"),
            pll: true,
            hard_ip: Vec::new(),
            notes: vec!["Offline capability registry; DSP support depends on family.".to_string()],
        },
        _ => VendorCapability {
            vendor: target.vendor.clone(),
            family: family.to_string(),
            bram: false,
            dsp: false,
            pll: false,
            hard_ip: Vec::new(),
            notes: vec!["Unknown target; resource plan remains generic.".to_string()],
        },
    }
}

fn board_alias(board: &str) -> Option<(&'static str, &'static str)> {
    match board {
        "tang-nano-20k" | "sipeed_tang_nano_20k" => Some(("gowin", "gw2a")),
        "tang-nano-9k" | "sipeed_tang_nano_9k" => Some(("gowin", "gw1n")),
        "arty-a7" | "digilent_arty_a7" => Some(("xilinx", "artix-7")),
        "basys3" | "digilent_basys3_artix7" => Some(("xilinx", "artix-7")),
        "xilinx-u55c" | "alveo-u55c" => Some(("xilinx", "ultrascale-plus")),
        "orangecrab" | "orangecrab_ecp5" => Some(("lattice", "ecp5")),
        _ => None,
    }
}
