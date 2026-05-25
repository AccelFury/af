// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const FPGA_IP_CORE_PROFILE_ID: &str = "fpga-ip-core-v1";

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct StandardsProfile {
    pub id: String,
    pub version: String,
    pub title: String,
    pub snapshot_date: String,
    pub summary: String,
    pub items: Vec<StandardsChecklistItem>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct StandardsChecklistItem {
    pub id: u8,
    pub item: String,
    pub category: String,
    pub tier_relevance: String,
    pub standards: Vec<StandardMapping>,
    pub required_evidence: String,
    pub required_artifact_kinds: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct StandardMapping {
    pub name: String,
    pub edition: String,
    pub area: String,
}

#[derive(Clone, Debug, Default, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct StandardsDeclaration {
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub artifacts: Vec<StandardsArtifact>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct StandardsArtifact {
    pub kind: String,
    pub path: String,
    #[serde(default)]
    pub standard: Option<String>,
    #[serde(default)]
    pub edition: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub required_for: Vec<u8>,
    #[serde(default)]
    pub conclusion: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
}

impl StandardsProfile {
    pub fn by_id(id: &str) -> Option<Self> {
        match id {
            FPGA_IP_CORE_PROFILE_ID => Some(fpga_ip_core_v1_profile()),
            _ => None,
        }
    }

    pub fn checklist_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("# {}\n\n", self.title));
        out.push_str(&format!(
            "Profile `{}` version `{}`; standards snapshot `{}`.\n\n",
            self.id, self.version, self.snapshot_date
        ));
        out.push_str(
            "| # | Item | Category | Tier relevance | Maps to standard | Required evidence artefact |\n",
        );
        out.push_str("|---|---|---|---|---|---|\n");
        for item in &self.items {
            let standards = item
                .standards
                .iter()
                .map(|s| format!("{} {}", s.name, s.edition))
                .collect::<Vec<_>>()
                .join("; ");
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                item.id,
                markdown_cell(&item.item),
                markdown_cell(&item.category),
                markdown_cell(&item.tier_relevance),
                markdown_cell(&standards),
                markdown_cell(&item.required_evidence),
            ));
        }
        out
    }

    pub fn compliance_csv(&self) -> String {
        let mut out = String::new();
        out.push_str("checklist_item_id,checklist_item,category,maps_to_standard,standard_edition_year,relevant_clause_or_area,evidence_artifact_required,portability_tier_relevance,required_artifact_kinds\n");
        for item in &self.items {
            if item.standards.is_empty() {
                out.push_str(&csv_row(&[
                    item.id.to_string(),
                    item.item.clone(),
                    item.category.clone(),
                    "(none)".to_string(),
                    "n/a".to_string(),
                    "n/a".to_string(),
                    item.required_evidence.clone(),
                    item.tier_relevance.clone(),
                    item.required_artifact_kinds.join(";"),
                ]));
                continue;
            }
            for standard in &item.standards {
                out.push_str(&csv_row(&[
                    item.id.to_string(),
                    item.item.clone(),
                    item.category.clone(),
                    standard.name.clone(),
                    standard.edition.clone(),
                    standard.area.clone(),
                    item.required_evidence.clone(),
                    item.tier_relevance.clone(),
                    item.required_artifact_kinds.join(";"),
                ]));
            }
        }
        out
    }

    pub fn compliance_json(&self) -> Value {
        let mut rows = Vec::new();
        for item in &self.items {
            if item.standards.is_empty() {
                rows.push(json!({
                    "checklist_item_id": item.id,
                    "checklist_item": item.item,
                    "category": item.category,
                    "maps_to_standard": "(none)",
                    "standard_edition_year": "n/a",
                    "relevant_clause_or_area": "n/a",
                    "evidence_artifact_required": item.required_evidence,
                    "portability_tier_relevance": item.tier_relevance,
                    "required_artifact_kinds": item.required_artifact_kinds,
                }));
                continue;
            }
            for standard in &item.standards {
                rows.push(json!({
                    "checklist_item_id": item.id,
                    "checklist_item": item.item,
                    "category": item.category,
                    "maps_to_standard": standard.name,
                    "standard_edition_year": standard.edition,
                    "relevant_clause_or_area": standard.area,
                    "evidence_artifact_required": item.required_evidence,
                    "portability_tier_relevance": item.tier_relevance,
                    "required_artifact_kinds": item.required_artifact_kinds,
                }));
            }
        }
        json!({
            "profile_id": self.id,
            "profile_version": self.version,
            "snapshot_date": self.snapshot_date,
            "rows": rows,
        })
    }
}

fn markdown_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn csv_row(fields: &[String]) -> String {
    let mut out = String::new();
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('"');
        out.push_str(&field.replace('"', "\"\""));
        out.push('"');
    }
    out.push('\n');
    out
}

pub fn fpga_ip_core_v1_profile() -> StandardsProfile {
    StandardsProfile {
        id: FPGA_IP_CORE_PROFILE_ID.to_string(),
        version: "1.0".to_string(),
        title: "af FPGA/IP-Core Standards Profile".to_string(),
        snapshot_date: "2026-05-25".to_string(),
        summary: "Machine-readable evidence profile for portable FPGA/IP cores. The profile is enumerative: safety/security rows are hooks unless explicit evidence is declared.".to_string(),
        items: vec![
            item(1, "Purpose", "now", "U0-U4", &[], "docs/spec.md section 1", &["spec"]),
            item(2, "Target users", "now", "U0-U4", &[], "docs/spec.md section 2", &["spec"]),
            item(3, "Use cases", "now", "U0-U4", &[std("IEEE 1012", "2024", "V&V concept of operations")], "docs/spec.md section 3", &["spec"]),
            item(4, "Non-goals", "now", "U0-U4", &[], "docs/spec.md section 4", &["spec"]),
            item(5, "Portability tier (U0-U4)", "now", "U0-U4", &[std("IEEE 1685", "2022", "vendor extensions / componentParameters")], "docs/spec.md section 5 + IP-XACT tier tag", &["spec", "ip-xact"]),
            item(6, "Interface", "now", "U0-U4", &[std("IEEE 1685", "2022", "busInterfaces / abstractionDefinition"), std("Accellera SystemRDL", "2.0 (Jan 2018)", "register interface description")], "docs/spec.md section 6 + ipxact/component.xml", &["spec", "ip-xact"]),
            item(7, "Parameters", "now", "U0-U4", &[std("IEEE 1685", "2022", "componentParameters"), std("IEEE 1364 / IEEE 1800", "2005 / 2023", "parameter declarations")], "docs/spec.md section 7 + RTL parameters", &["spec"]),
            item(8, "Architecture", "now", "U0-U4", &[std("IEEE 1012", "2024", "design description adjacent process")], "docs/spec.md section 8 + block diagram", &["spec"]),
            item(9, "Datapath / control FSM", "now", "U0-U4", &[std("IEEE 1364", "2001 subset", "procedural blocks and FSM coding"), std("lowRISC Verilog Style Guide", "continuous", "FSM style")], "docs/spec.md section 9 + FSM diagram", &["spec"]),
            item(10, "Reset / clock / CDC behaviour", "now", "U0-U4", &[std("CDC/RDC methodology", "no single IEEE/ISO standard", "Cummings SNUG 2008, vendor CDC guides, Accellera CDC/RDC 1.0")], "docs/spec.md section 10 + CDC notes", &["spec"]),
            item(11, "Protocol semantics", "now", "U0-U4", &[std("IEEE 1685", "2022", "abstractionDefinition")], "docs/spec.md section 11 + timing diagrams", &["spec"]),
            item(12, "Error / status / counter behaviour", "now", "U0-U4", &[std("Accellera SystemRDL", "2.0 (Jan 2018)", "field/register semantics"), std("IEEE 1685", "2022", "register descriptions")], "docs/spec.md section 12 + regs.rdl", &["spec", "systemrdl"]),
            item(13, "Timing and latency", "now", "U0-U4", &[std("Vendor datasheet conventions", "continuous", "AMD PGxxx / Intel UG-xxxxx style latency tables")], "docs/spec.md section 13 + latency table", &["spec"]),
            item(14, "Corner cases", "now", "U0-U4", &[std("IEEE 1012", "2024", "V&V activities")], "docs/spec.md section 14", &["spec"]),
            item(15, "Simulation plan", "now", "U0-U4", &[std("IEEE 1800.2", "2020", "UVM"), std("IEEE 1800", "2023", "SVA"), std("IEEE 1850", "2010", "PSL"), std("ISO/IEC/IEEE 29119", "2021-2024", "test process/docs/techniques")], "sim/README.md + test plan", &["simulation-plan"]),
            item(16, "Formal verification plan", "now/foundation", "U0-U2", &[std("IEEE 1800", "2023", "SVA"), std("IEEE 1850", "2010", "PSL"), std("IEEE 1012", "2024", "V&V process")], "formal/README.md + property files", &["formal-plan"]),
            item(17, "Synthesis targets", "now", "U0-U4", &[std("IEEE 1364", "2001 subset", "portable synthesis baseline"), std("IEEE 1800", "2023", "restricted synthesis subset")], "synth/results.md", &["synthesis-report"]),
            item(18, "Board demo plan", "now", "U0-U2", &[], "boards/<board>/README.md", &["board-demo"]),
            item(19, "Repository structure", "now", "U0-U4", &[std("IEEE 1685", "2022", "fileSets"), std("FuseSoC core file", "continuous", ".core package convention")], "repo layout + <core>.core", &["fusesoc-core"]),
            item(20, "Documentation requirements", "now", "U0-U4", &[std("ISO/IEC/IEEE 26515", "2018", "user documentation process, informative")], "docs/datasheet.md", &["datasheet"]),
            item(21, "License / provenance notes", "now", "U0-U4", &[std("SPDX License List", "pin at release time", "license identifiers"), std("CERN-OHL-2.0 / Apache-2.0 / MIT / BSD-3-Clause", "various", "license policy"), std("IEEE 1735", "2023", "explicitly disallowed unless commercially fenced")], "LICENSE, NOTICE, SPDX headers", &["license", "spdx-header-audit", "spdx-hbom"]),
            item(22, "Acceptance criteria", "now", "U0-U4", &[std("IEEE 1012", "2024", "acceptance test activities"), std("ISO/IEC/IEEE 29119-3", "2021", "test completion report")], "docs/acceptance.md + CI evidence", &["acceptance"]),
            item(23, "Risks and mitigations", "now/foundation", "U0-U4", &[std("ISO 26262-9", "2018", "ASIL-oriented analyses, forward-looking")], "docs/risks.md", &["risks"]),
            item(24, "IP-XACT packaging + machine-readable metadata", "now", "U0-U4", &[std("IEEE 1685", "2022", "components, busInterfaces, abstractionDefinition, design, generators")], "ipxact/<core>.xml", &["ip-xact"]),
            item(25, "Register description (single source of truth)", "now", "U0-U4", &[std("Accellera SystemRDL", "2.0 (Jan 2018)", "field/register/regfile/address map"), std("IEEE 1800.2", "2020", "UVM RAL")], "regs/<core>.rdl", &["systemrdl"]),
            item(26, "Power intent (or N/A justification)", "foundation", "U2-U4", &[std("IEEE 1801", "2024 (published Mar 2025)", "UPF 4.0 power intent")], "power/<core>.upf or N/A note", &["upf", "power-na"]),
            item(27, "DFT / test access", "foundation", "U2-U4", &[std("IEEE 1149.1", "2013 inactive-reserved / legacy hook", "JTAG TAP and boundary scan"), std("IEEE 1500", "2022", "embedded core test / CTL")], "dft/<core>.ctl, BSDL fragment, or N/A note", &["dft", "jtag-na"]),
            item(28, "Coding-style and lint", "now", "U0-U4", &[std("lowRISC Verilog Style Guide", "continuous", "RTL style"), std("Verible", "continuous", "lint/format"), std("IEEE 1800", "2023", "restricted subset policy")], ".verible.rules + lint-clean CI", &["verible-lint", "native-lint"]),
            item(29, "CI, reproducibility, semantic versioning", "now", "U0-U4", &[std("SemVer", "2.0.0", "MAJOR.MINOR.PATCH"), std("ISO/IEC/IEEE 29119-2", "2021", "test process")], ".github/workflows/*.yml, VERSION, CHANGELOG.md", &["ci"]),
            item(30, "Safety hooks (forward-looking)", "foundation", "U0-U4", &[std("ISO 26262", "2018", "HW/supporting/semiconductor guidance"), std("IEC 61508-2", "2010", "SIL hardware requirements"), std("DO-254 / FAA AC 20-152A", "2000 / 2020", "airborne electronic hardware")], "safety/safety_manual.md placeholder or populated manual", &["safety-manual"]),
            item(31, "Security hooks (forward-looking, actionable now)", "foundation", "U0-U4", &[std("MITRE CWE-1194", "CWE List v4.20 (30 Apr 2026)", "Hardware Design view"), std("NIST IR 8517", "13 Nov 2024", "hardware security failure scenarios"), std("Accellera SA-EDI", "1.0 (14 Jul 2021)", "IP security annotations"), std("IEEE P3164", "draft / PAR active", "future security annotation standard"), std("FIPS 140-3", "2019", "crypto modules only"), std("ISO/IEC 15408", "2022", "Common Criteria only if evaluated")], "security/threat_model.md, security/sa-edi.json, CWE coverage", &["security-threat-model", "sa-edi", "cwe-coverage"]),
            item(32, "HBOM / provenance / SBOM-equivalent", "now", "U0-U4", &[std("SPDX Specification", "3.0.1", "SBOM/HBOM document format"), std("SPDX License List", "pin at release time", "license expression grammar"), std("CycloneDX", "continuous", "alternative HBOM profile")], "hbom/<core>.spdx.json or CycloneDX", &["spdx-hbom", "cyclonedx-hbom"]),
        ],
    }
}

fn item(
    id: u8,
    item: &str,
    category: &str,
    tier_relevance: &str,
    standards: &[StandardMapping],
    required_evidence: &str,
    required_artifact_kinds: &[&str],
) -> StandardsChecklistItem {
    StandardsChecklistItem {
        id,
        item: item.to_string(),
        category: category.to_string(),
        tier_relevance: tier_relevance.to_string(),
        standards: standards.to_vec(),
        required_evidence: required_evidence.to_string(),
        required_artifact_kinds: required_artifact_kinds
            .iter()
            .map(|kind| (*kind).to_string())
            .collect(),
    }
}

fn std(name: &str, edition: &str, area: &str) -> StandardMapping {
    StandardMapping {
        name: name.to_string(),
        edition: edition.to_string(),
        area: area.to_string(),
    }
}
