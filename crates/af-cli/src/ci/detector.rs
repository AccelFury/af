// SPDX-License-Identifier: Apache-2.0

use crate::ci::scanner::RepoScan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectProfile {
    Verilog2001CoreOnly,
    VerilogWithIverilogMake,
    VerilatorCpp,
    VhdlGhdl,
    FormalSby,
    Ice40Board,
    Ecp5Board,
    GowinBoard,
    XilinxSynthOnly,
    IntelSynthOnly,
    UnknownSynthOnly,
}

impl ProjectProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Verilog2001CoreOnly => "verilog_2001_core_only",
            Self::VerilogWithIverilogMake => "verilog_with_iverilog_make",
            Self::VerilatorCpp => "verilator_cpp",
            Self::VhdlGhdl => "vhdl_ghdl",
            Self::FormalSby => "formal_sby",
            Self::Ice40Board => "ice40_board",
            Self::Ecp5Board => "ecp5_board",
            Self::GowinBoard => "gowin_board",
            Self::XilinxSynthOnly => "xilinx_synth_only",
            Self::IntelSynthOnly => "intel_synth_only",
            Self::UnknownSynthOnly => "unknown_synth_only",
        }
    }
}

pub fn detect_profile(scan: &RepoScan, hdl: &str) -> ProjectProfile {
    if hdl.to_lowercase().contains("vhdl") {
        return ProjectProfile::VhdlGhdl;
    }
    if scan.has_sby {
        return ProjectProfile::FormalSby;
    }
    if !scan.constraints.is_empty() {
        let has_cst = scan
            .constraints
            .iter()
            .any(|path| path.extension().and_then(|ext| ext.to_str()) == Some("cst"));
        if has_cst {
            return ProjectProfile::GowinBoard;
        }
        let has_lpf = scan
            .constraints
            .iter()
            .any(|path| path.extension().and_then(|ext| ext.to_str()) == Some("lpf"));
        if has_lpf {
            return ProjectProfile::Ecp5Board;
        }
        let has_pcf = scan
            .constraints
            .iter()
            .any(|path| path.extension().and_then(|ext| ext.to_str()) == Some("pcf"));
        if has_pcf {
            return ProjectProfile::Ice40Board;
        }
    }
    if scan.has_make_test_target {
        return ProjectProfile::VerilogWithIverilogMake;
    }

    if !scan.sim_files.is_empty() {
        ProjectProfile::VerilatorCpp
    } else {
        ProjectProfile::Verilog2001CoreOnly
    }
}

pub fn has_vendor_board_profile(scan: &RepoScan) -> bool {
    matches!(
        detect_profile(scan, "verilog"),
        ProjectProfile::Ice40Board | ProjectProfile::Ecp5Board | ProjectProfile::GowinBoard
    )
}

pub fn profile_from_str(value: &str) -> Option<ProjectProfile> {
    match value.to_lowercase().as_str() {
        "verilog_2001_core_only" | "core_only" | "core-only" | "coreonly" => {
            Some(ProjectProfile::Verilog2001CoreOnly)
        }
        "verilog_with_iverilog_make" | "iverilog_make" | "iverilog" => {
            Some(ProjectProfile::VerilogWithIverilogMake)
        }
        "verilator_cpp" | "verilator" => Some(ProjectProfile::VerilatorCpp),
        "vhdl_ghdl" | "vhdl" => Some(ProjectProfile::VhdlGhdl),
        "formal_sby" | "sby" => Some(ProjectProfile::FormalSby),
        "ice40_board" | "ice40" => Some(ProjectProfile::Ice40Board),
        "ecp5_board" | "ecp5" => Some(ProjectProfile::Ecp5Board),
        "gowin_board" | "gowin" => Some(ProjectProfile::GowinBoard),
        "xilinx_synth_only" | "xilinx" => Some(ProjectProfile::XilinxSynthOnly),
        "intel_synth_only" | "intel" => Some(ProjectProfile::IntelSynthOnly),
        "unknown_synth_only" | "unknown" => Some(ProjectProfile::UnknownSynthOnly),
        _ => None,
    }
}
