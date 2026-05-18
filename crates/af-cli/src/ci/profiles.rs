// SPDX-License-Identifier: Apache-2.0

use std::borrow::Cow;

#[derive(Debug, Clone)]
pub struct ProfileTemplate {
    pub profile: Cow<'static, str>,
    pub sim_command: Cow<'static, str>,
    pub synth_command: Cow<'static, str>,
    pub formal_command: Cow<'static, str>,
    pub pnr_command: Cow<'static, str>,
    pub warnings: Vec<String>,
}

pub const GENERIC_YOSYS: ProfileTemplate = ProfileTemplate {
    profile: Cow::Borrowed("generic_yosys"),
    sim_command: Cow::Borrowed("cd sim && make test"),
    synth_command: Cow::Borrowed(
        "yosys -p 'read_verilog -sv -Wall ${RTL_FILES}; hierarchy -check -top ${TOP}; proc; opt; write_json ${ARTIFACT_JSON}'",
    ),
    formal_command: Cow::Borrowed("sby -f ${FORMAL_FILE}"),
    pnr_command: Cow::Borrowed("true"),
    warnings: Vec::new(),
};

pub const GOWIN_HIMBAECHL: ProfileTemplate = ProfileTemplate {
    profile: Cow::Borrowed("gowin_himbaechel"),
    sim_command: Cow::Borrowed("cd sim && make test"),
    synth_command: Cow::Borrowed(
        "yosys -p 'read_verilog ${RTL_FILES}; hierarchy -top ${TOP}; write_json ${ARTIFACT_JSON}'",
    ),
    formal_command: Cow::Borrowed("sby -f ${FORMAL_FILE}"),
    pnr_command: Cow::Borrowed(
        "nextpnr-gowin --json ${SYNTH_JSON} --pcf ${CONSTRAINTS} --write ${PNR_JSON}",
    ),
    warnings: Vec::new(),
};

pub const VERILATOR_CPP: ProfileTemplate = ProfileTemplate {
    profile: Cow::Borrowed("verilator_cpp"),
    sim_command: Cow::Borrowed("verilator --lint-only ${RTL_FILES}"),
    synth_command: Cow::Borrowed("yosys -p 'read_verilog ${RTL_FILES}; hierarchy -check -top ${TOP}; write_json ${ARTIFACT_JSON}'"),
    formal_command: Cow::Borrowed("sby -f ${FORMAL_FILE}"),
    pnr_command: Cow::Borrowed("true"),
    warnings: Vec::new(),
};

pub const IVERILOG_MAKE: ProfileTemplate = ProfileTemplate {
    profile: Cow::Borrowed("iverilog_make"),
    sim_command: Cow::Borrowed("cd sim && make test"),
    synth_command: Cow::Borrowed("yosys -p 'read_verilog ${RTL_FILES}; hierarchy -check -top ${TOP}; write_json ${ARTIFACT_JSON}'"),
    formal_command: Cow::Borrowed("sby -f ${FORMAL_FILE}"),
    pnr_command: Cow::Borrowed("true"),
    warnings: Vec::new(),
};
