// SPDX-License-Identifier: Apache-2.0
//
// FPGA standards profile CLI contract tests. These pin the machine-readable
// checklist/profile surface before the implementation grows richer validators.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn write_text(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

#[cfg(unix)]
fn write_executable(path: &Path, content: &str) {
    use std::os::unix::fs::PermissionsExt;

    write_text(path, content);
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

fn repo_root() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn af() -> Command {
    Command::cargo_bin("af").expect("cargo bin `af`")
}

fn copy_dir(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir(&from, &to);
        } else {
            fs::copy(&from, &to).unwrap();
        }
    }
}

fn temp_af_mod_add() -> tempfile::TempDir {
    let temp = tempfile::TempDir::new().unwrap();
    copy_dir(
        &repo_root().join("examples").join("af-mod-add"),
        temp.path(),
    );
    temp
}

#[test]
fn standards_export_json_contains_fpga_ip_core_profile() {
    let out = af()
        .args([
            "--json",
            "core",
            "standards",
            "export",
            "--profile",
            "fpga-ip-core-v1",
            "--format",
            "json",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success(), "export failed: {:?}", out);
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");

    assert_eq!(value["status"], "passed");
    assert_eq!(value["profile"]["id"], "fpga-ip-core-v1");
    assert_eq!(value["profile"]["items"].as_array().unwrap().len(), 32);
    assert_eq!(value["profile"]["items"][23]["id"], 24);
    assert_eq!(
        value["profile"]["items"][23]["required_artifact_kinds"][0],
        "ip-xact"
    );

    let security = value["profile"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == 31)
        .expect("security row");
    assert!(security["standards"]
        .as_array()
        .unwrap()
        .iter()
        .any(|standard| standard["name"] == "MITRE CWE-1194"
            && standard["edition"] == "CWE List v4.20 (30 Apr 2026)"));

    let license = value["profile"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == 21)
        .expect("license row");
    assert!(license["required_artifact_kinds"]
        .as_array()
        .unwrap()
        .iter()
        .any(|kind| kind == "spdx-header-audit"));
}

#[test]
fn standards_root_artifacts_match_profile_exports() {
    for (format, path) in [
        ("checklist", "CHECKLIST.md"),
        ("csv", "compliance_matrix.csv"),
        ("json", "compliance_matrix.json"),
    ] {
        let out = af()
            .args([
                "--json",
                "core",
                "standards",
                "export",
                "--profile",
                "fpga-ip-core-v1",
                "--format",
                format,
            ])
            .output()
            .expect("execute");
        assert!(out.status.success(), "export failed: {:?}", out);
        let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
        let exported = value["content"].as_str().expect("exported content");
        let committed = fs::read_to_string(repo_root().join(path)).expect("committed artifact");
        assert_eq!(exported, committed, "{path} is stale");
    }
}

#[test]
fn standards_check_reports_missing_now_evidence_without_certification_claims() {
    let core = repo_root().join("examples").join("af-mod-add");
    let out = af()
        .args([
            "--json",
            "core",
            "standards",
            "check",
            core.to_str().unwrap(),
            "--profile",
            "fpga-ip-core-v1",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success(), "check failed: {:?}", out);
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");

    assert_eq!(value["status"], "blocked");
    assert_eq!(value["profile"], "fpga-ip-core-v1");
    assert_eq!(value["summary"]["total_items"], 32);
    assert_eq!(value["summary"]["foundation_items"], 6);

    let rows = value["rows"].as_array().unwrap();
    let ipxact = rows
        .iter()
        .find(|row| row["checklist_item_id"] == 24)
        .expect("ip-xact row");
    assert_eq!(ipxact["status"], "blocked");
    assert!(ipxact["limitations"][0]
        .as_str()
        .unwrap()
        .contains("ip-xact"));

    let safety = rows
        .iter()
        .find(|row| row["checklist_item_id"] == 30)
        .expect("safety row");
    assert_eq!(safety["status"], "planned");
    assert!(safety["limitations"][0]
        .as_str()
        .unwrap()
        .contains("foundation hook"));

    let license = rows
        .iter()
        .find(|row| row["checklist_item_id"] == 21)
        .expect("license/provenance row");
    assert_eq!(license["status"], "blocked");
    assert_eq!(license["validation_status"], "partial");
    assert!(license["limitations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|limitation| limitation.as_str().unwrap().contains("spdx-hbom")));

    assert_eq!(
        value["gates"]["commercial_baseline_ready"]["status"],
        "blocked"
    );
    assert!(value["gates"]["commercial_baseline_ready"]["limitations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|limitation| limitation.as_str().unwrap().contains("not a certification")));
}

#[test]
fn standards_doctor_reports_tool_availability_without_core() {
    let out = af()
        .args([
            "--json",
            "core",
            "standards",
            "doctor",
            "--profile",
            "fpga-ip-core-v1",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success(), "doctor failed: {:?}", out);
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");

    assert_eq!(value["status"], "passed");
    assert_eq!(value["profile"], "fpga-ip-core-v1");
    let tools = value["tools"].as_array().expect("tools");
    for expected in [
        "xmllint",
        "peakrdl",
        "verible-verilog-lint",
        "verilator",
        "sby",
        "reuse",
        "spdx-sbom-generator",
    ] {
        assert!(
            tools.iter().any(|tool| tool["program"] == expected
                && tool["available"].as_bool().is_some()
                && !tool["required_for"].as_array().unwrap().is_empty()),
            "missing tool doctor row for {expected}: {tools:?}"
        );
    }
    assert!(tools
        .iter()
        .all(|tool| tool["install_hint"].as_str().is_some()));
    assert!(tools.iter().any(|tool| tool["program"] == "peakrdl"
        && tool["install_hint"].as_str().unwrap().contains("peakrdl")));
}

#[test]
fn standards_drift_reports_current_snapshot_without_network() {
    let out = af()
        .args([
            "--json",
            "core",
            "standards",
            "drift",
            "--profile",
            "fpga-ip-core-v1",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success(), "drift failed: {:?}", out);
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");

    assert_eq!(value["status"], "passed");
    assert_eq!(value["profile"], "fpga-ip-core-v1");
    assert_eq!(value["snapshot_date"], "2026-05-25");
    assert!(value["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|finding| finding["standard"] == "MITRE CWE-1194" && finding["severity"] == "ok"));
    assert!(value["limitations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|limitation| limitation.as_str().unwrap().contains("offline freshness")));
}

#[test]
fn standards_check_blocks_malformed_ipxact_artifact() {
    let core = temp_af_mod_add();
    let ipxact_dir = core.path().join("ipxact");
    fs::create_dir_all(&ipxact_dir).unwrap();
    fs::write(ipxact_dir.join("af_mod_add.xml"), "<component/>").unwrap();

    let out = af()
        .args([
            "--json",
            "core",
            "standards",
            "check",
            core.path().to_str().unwrap(),
            "--profile",
            "fpga-ip-core-v1",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success(), "check failed: {:?}", out);
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    let rows = value["rows"].as_array().unwrap();
    let ipxact = rows
        .iter()
        .find(|row| row["checklist_item_id"] == 24)
        .expect("ip-xact row");

    assert_eq!(ipxact["status"], "blocked");
    assert_eq!(ipxact["validation_status"], "invalid");
    assert!(ipxact["limitations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|limitation| limitation
            .as_str()
            .unwrap()
            .contains("semantic validation failed")));
}

#[test]
fn standards_check_accepts_minimal_semantic_ipxact_artifact() {
    let core = temp_af_mod_add();
    let ipxact_dir = core.path().join("ipxact");
    fs::create_dir_all(&ipxact_dir).unwrap();
    fs::write(
        ipxact_dir.join("af_mod_add.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ipxact:component xmlns:ipxact="http://www.accellera.org/XMLSchema/IPXACT/1685-2022">
  <ipxact:vendor>accelfury</ipxact:vendor>
  <ipxact:library>ip</ipxact:library>
  <ipxact:name>af_mod_add</ipxact:name>
  <ipxact:version>0.1.0</ipxact:version>
  <ipxact:busInterfaces>
    <ipxact:busInterface><ipxact:name>stream</ipxact:name></ipxact:busInterface>
  </ipxact:busInterfaces>
  <ipxact:model>
    <ipxact:views>
      <ipxact:view><ipxact:modelName>af_mod_add_top</ipxact:modelName></ipxact:view>
    </ipxact:views>
  </ipxact:model>
  <ipxact:fileSets>
    <ipxact:fileSet>
      <ipxact:name>rtl</ipxact:name>
      <ipxact:file><ipxact:name>rtl/core/af_mod_add_top.sv</ipxact:name></ipxact:file>
    </ipxact:fileSet>
  </ipxact:fileSets>
</ipxact:component>
"#,
    )
    .unwrap();

    let out = af()
        .args([
            "--json",
            "core",
            "standards",
            "check",
            core.path().to_str().unwrap(),
            "--profile",
            "fpga-ip-core-v1",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success(), "check failed: {:?}", out);
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    let rows = value["rows"].as_array().unwrap();
    let ipxact = rows
        .iter()
        .find(|row| row["checklist_item_id"] == 24)
        .expect("ip-xact row");

    assert_eq!(ipxact["status"], "supported");
    assert_eq!(ipxact["validation_status"], "semantic-valid");
}

#[test]
fn standards_scaffold_writes_missing_evidence_without_overwriting() {
    let core = temp_af_mod_add();
    let out = af()
        .args([
            "--json",
            "core",
            "standards",
            "scaffold",
            core.path().to_str().unwrap(),
            "--profile",
            "fpga-ip-core-v1",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success(), "scaffold failed: {:?}", out);
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");

    assert_eq!(value["status"], "passed");
    assert_eq!(value["profile"], "fpga-ip-core-v1");
    assert!(value["written"].as_array().unwrap().len() > 10);

    let spec = core.path().join("docs/spec.md");
    let ipxact = core.path().join("ipxact/af_mod_add.xml");
    let rdl = core.path().join("regs/af_mod_add.rdl");
    let hbom = core.path().join("hbom/af_mod_add.spdx.json");
    assert!(fs::read_to_string(&spec)
        .unwrap()
        .contains("## 24. IP-XACT packaging + machine-readable metadata"));
    assert!(fs::read_to_string(&ipxact).unwrap().contains("1685-2022"));
    assert!(fs::read_to_string(&rdl)
        .unwrap()
        .contains("addrmap af_mod_add"));
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(&hbom).unwrap()).unwrap()["kind"],
        "accelfury.hbom.spdx"
    );

    fs::write(&spec, "local edit\n").unwrap();
    let second = af()
        .args([
            "--json",
            "core",
            "standards",
            "scaffold",
            core.path().to_str().unwrap(),
            "--profile",
            "fpga-ip-core-v1",
        ])
        .output()
        .expect("execute");
    assert!(
        second.status.success(),
        "second scaffold failed: {:?}",
        second
    );
    assert_eq!(fs::read_to_string(&spec).unwrap(), "local edit\n");
    let value: Value = serde_json::from_slice(&second.stdout).expect("JSON");
    assert!(value["existing"]
        .as_array()
        .unwrap()
        .iter()
        .any(|path| path.as_str().unwrap().ends_with("docs/spec.md")));
}

#[test]
fn core_new_can_opt_into_fpga_standards_scaffold() {
    let temp = tempfile::TempDir::new().unwrap();
    let core = temp.path().join("std_demo");

    let out = af()
        .args([
            "--json",
            "core",
            "new",
            core.to_str().unwrap(),
            "--name",
            "std_demo",
            "--class",
            "simple-portable",
            "--language",
            "verilog-2001",
            "--standards-profile",
            "fpga-ip-core-v1",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success(), "core new failed: {:?}", out);
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");

    assert_eq!(value["status"], "passed");
    assert_eq!(value["standards_scaffold"]["profile"], "fpga-ip-core-v1");
    assert!(core.join("docs/spec.md").is_file());
    assert!(core.join("ipxact/std_demo.xml").is_file());
    assert!(core.join("regs/std_demo.rdl").is_file());
    assert!(core.join("hbom/std_demo.spdx.json").is_file());

    let manifest = fs::read_to_string(core.join("af-core.toml")).unwrap();
    assert!(manifest.contains("[standards]"));
    assert!(manifest.contains("profile = \"fpga-ip-core-v1\""));
    assert!(manifest.contains("kind = \"ip-xact\""));
    assert!(manifest.contains("path = \"ipxact/std_demo.xml\""));
    assert!(manifest.contains("kind = \"spdx-hbom\""));
}

#[test]
fn standards_scaffold_declare_adds_manifest_evidence_without_overwriting_files() {
    let core = temp_af_mod_add();
    let spec = core.path().join("docs/spec.md");
    fs::create_dir_all(spec.parent().unwrap()).unwrap();
    fs::write(&spec, "local spec\n").unwrap();

    let out = af()
        .args([
            "--json",
            "core",
            "standards",
            "scaffold",
            core.path().to_str().unwrap(),
            "--profile",
            "fpga-ip-core-v1",
            "--declare",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success(), "scaffold failed: {:?}", out);
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert_eq!(value["declared"], true);
    assert!(value["manifest_artifacts_added"].as_array().unwrap().len() > 10);
    assert_eq!(fs::read_to_string(&spec).unwrap(), "local spec\n");

    let manifest = fs::read_to_string(core.path().join("af-core.toml")).unwrap();
    assert!(manifest.contains("[standards]"));
    assert!(manifest.contains("profile = \"fpga-ip-core-v1\""));
    assert!(manifest.contains("kind = \"ip-xact\""));
    assert!(manifest.contains("path = \"ipxact/af_mod_add.xml\""));
    assert!(manifest.contains("kind = \"security-threat-model\""));

    let second = af()
        .args([
            "--json",
            "core",
            "standards",
            "scaffold",
            core.path().to_str().unwrap(),
            "--profile",
            "fpga-ip-core-v1",
            "--declare",
        ])
        .output()
        .expect("execute");
    assert!(
        second.status.success(),
        "second scaffold failed: {:?}",
        second
    );
    let value: Value = serde_json::from_slice(&second.stdout).expect("JSON");
    assert_eq!(
        value["manifest_artifacts_added"].as_array().unwrap().len(),
        0
    );
}

#[test]
fn core_report_json_includes_standards_summary_when_profile_declared() {
    let temp = tempfile::TempDir::new().unwrap();
    let core = temp.path().join("std_report");
    let new_out = af()
        .args([
            "--json",
            "core",
            "new",
            core.to_str().unwrap(),
            "--name",
            "std_report",
            "--class",
            "simple-portable",
            "--language",
            "verilog-2001",
            "--standards-profile",
            "fpga-ip-core-v1",
        ])
        .output()
        .expect("execute");
    assert!(new_out.status.success(), "core new failed: {:?}", new_out);

    let report = af()
        .args(["--json", "core", "report", core.to_str().unwrap()])
        .output()
        .expect("execute");
    assert!(report.status.success(), "report failed: {:?}", report);
    let value: Value = serde_json::from_slice(&report.stdout).expect("JSON");

    assert_eq!(value["standards"]["profile"], "fpga-ip-core-v1");
    assert_eq!(value["standards"]["summary"]["total_items"], 32);
    assert!(
        value["standards"]["summary"]["supported_items"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert!(value["standards"]["limitations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|limitation| limitation
            .as_str()
            .unwrap()
            .contains("does not claim certification")));
}

#[test]
fn standards_check_strict_keeps_semantic_valid_when_external_validators_are_unavailable() {
    let core = temp_af_mod_add();
    let scaffold = af()
        .args([
            "--json",
            "core",
            "standards",
            "scaffold",
            core.path().to_str().unwrap(),
            "--profile",
            "fpga-ip-core-v1",
        ])
        .output()
        .expect("execute");
    assert!(scaffold.status.success(), "scaffold failed: {:?}", scaffold);

    let out = af()
        .env("PATH", "")
        .args([
            "--json",
            "core",
            "standards",
            "check",
            core.path().to_str().unwrap(),
            "--profile",
            "fpga-ip-core-v1",
            "--strict",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success(), "strict check failed: {:?}", out);
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    let rows = value["rows"].as_array().unwrap();
    let ipxact = rows
        .iter()
        .find(|row| row["checklist_item_id"] == 24)
        .expect("ip-xact row");

    assert_eq!(ipxact["status"], "supported");
    assert_eq!(ipxact["validation_status"], "semantic-valid");
    assert!(ipxact["limitations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|limitation| limitation
            .as_str()
            .unwrap()
            .contains("external validator `xmllint` unavailable")));
    assert!(value["tool_availability"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .any(|tool| tool["program"] == "xmllint" && tool["available"] == false));
}

#[cfg(unix)]
#[test]
fn standards_check_strict_blocks_when_available_external_validator_fails() {
    let core = temp_af_mod_add();
    let scaffold = af()
        .args([
            "--json",
            "core",
            "standards",
            "scaffold",
            core.path().to_str().unwrap(),
            "--profile",
            "fpga-ip-core-v1",
        ])
        .output()
        .expect("execute");
    assert!(scaffold.status.success(), "scaffold failed: {:?}", scaffold);

    let tools = tempfile::TempDir::new().unwrap();
    write_executable(
        &tools.path().join("xmllint"),
        "#!/bin/sh\necho forced-xmllint-failure >&2\nexit 9\n",
    );
    write_executable(&tools.path().join("peakrdl"), "#!/bin/sh\nexit 0\n");

    let out = af()
        .env("PATH", tools.path())
        .args([
            "--json",
            "core",
            "standards",
            "check",
            core.path().to_str().unwrap(),
            "--profile",
            "fpga-ip-core-v1",
            "--strict",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success(), "strict check failed: {:?}", out);
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    let rows = value["rows"].as_array().unwrap();
    let ipxact = rows
        .iter()
        .find(|row| row["checklist_item_id"] == 24)
        .expect("ip-xact row");

    assert_eq!(ipxact["status"], "blocked");
    assert_eq!(ipxact["validation_status"], "invalid");
    assert!(ipxact["limitations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|limitation| limitation
            .as_str()
            .unwrap()
            .contains("forced-xmllint-failure")));
}

#[test]
fn package_spdx_hbom_writes_machine_readable_provenance() {
    let build_root = tempfile::TempDir::new().unwrap();
    let core = repo_root().join("examples").join("af-mod-add");
    let out = af()
        .args([
            "--json",
            "--build-root",
            build_root.path().to_str().unwrap(),
            "core",
            "package",
            core.to_str().unwrap(),
            "--format",
            "spdx-hbom",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success(), "package failed: {:?}", out);
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    let package_path = value["package"].as_str().expect("package path");
    assert!(package_path.ends_with("af_mod_add.hbom.spdx.json"));

    let hbom: Value =
        serde_json::from_slice(&std::fs::read(package_path).expect("read generated HBOM"))
            .expect("HBOM JSON");
    assert_eq!(hbom["kind"], "accelfury.hbom.spdx");
    assert_eq!(hbom["profile"], "fpga-ip-core-v1");
    assert_eq!(hbom["core"], "accelfury:ip:af_mod_add:0.1.0");
    assert_eq!(hbom["release"]["semver"], "0.1.0");
    assert!(hbom["release"]["commit_sha"].as_str().is_some());
    assert!(hbom["release"]["dirty_tree"].as_bool().is_some());
    assert!(hbom["release"]["tag_signature_status"].as_str().is_some());
    assert!(!hbom["files"].as_array().unwrap().is_empty());
    let checksum = &hbom["files"][0]["checksum"];
    assert_eq!(checksum["algorithm"], "SHA256");
    assert_eq!(checksum["value"].as_str().unwrap().len(), 64);
}

#[test]
fn scaffolded_spdx_hbom_includes_standards_evidence_files() {
    let core = temp_af_mod_add();
    let scaffold = af()
        .args([
            "--json",
            "core",
            "standards",
            "scaffold",
            core.path().to_str().unwrap(),
            "--profile",
            "fpga-ip-core-v1",
        ])
        .output()
        .expect("execute");
    assert!(scaffold.status.success(), "scaffold failed: {:?}", scaffold);

    let hbom_path = core.path().join("hbom/af_mod_add.spdx.json");
    let hbom: Value = serde_json::from_slice(&fs::read(&hbom_path).unwrap()).expect("HBOM JSON");
    let files = hbom["files"].as_array().unwrap();

    for expected in [
        "docs/spec.md",
        "ipxact/af_mod_add.xml",
        "regs/af_mod_add.rdl",
        "security/sa-edi.json",
    ] {
        let row = files
            .iter()
            .find(|file| file["path"] == expected)
            .unwrap_or_else(|| panic!("missing HBOM evidence row for {expected}"));
        assert_eq!(row["role"], "standards-evidence");
        assert_eq!(row["checksum"]["algorithm"], "SHA256");
        assert_eq!(row["checksum"]["value"].as_str().unwrap().len(), 64);
    }
}

#[test]
fn spdx_audit_blocks_missing_headers_and_declares_report() {
    let core = temp_af_mod_add();
    let source = core.path().join("rtl/core/af_mod_add_top.sv");
    fs::write(&source, "module af_mod_add_top; endmodule\n").unwrap();

    let out = af()
        .args([
            "--json",
            "core",
            "standards",
            "spdx-audit",
            core.path().to_str().unwrap(),
            "--output",
            "reports/spdx-header-audit.json",
            "--declare",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success(), "spdx audit failed: {:?}", out);
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");

    assert_eq!(value["status"], "blocked");
    assert!(value["summary"]["missing_headers"].as_u64().unwrap() > 0);
    assert_eq!(
        value["output"],
        core.path()
            .join("reports/spdx-header-audit.json")
            .display()
            .to_string()
    );
    assert!(core.path().join("reports/spdx-header-audit.json").is_file());
    let manifest = fs::read_to_string(core.path().join("af-core.toml")).unwrap();
    assert!(manifest.contains("kind = \"spdx-header-audit\""));
    assert!(manifest.contains("path = \"reports/spdx-header-audit.json\""));
}

#[test]
fn standards_collect_declares_build_reports_without_overwriting_manifest_entries() {
    let core = temp_af_mod_add();
    let build_root = tempfile::TempDir::new().unwrap();
    fs::create_dir_all(build_root.path().join("reports")).unwrap();
    fs::create_dir_all(build_root.path().join("package")).unwrap();
    fs::write(
        build_root.path().join("reports/core-lint.json"),
        r#"{"status":"passed","kind":"lint"}"#,
    )
    .unwrap();
    fs::write(
        build_root.path().join("reports/core-sim.json"),
        r#"{"status":"passed","kind":"simulation"}"#,
    )
    .unwrap();
    fs::write(
        build_root.path().join("reports/core-formal.json"),
        r#"{"status":"passed","kind":"formal"}"#,
    )
    .unwrap();
    fs::write(
        build_root.path().join("package/af_mod_add.hbom.spdx.json"),
        r#"{"kind":"accelfury.hbom.spdx","spdx_version":"SPDX-3.0.1-compatible","files":[{"path":"rtl/core/af_mod_add_top.sv","checksum":{"algorithm":"SHA256","value":"0000000000000000000000000000000000000000000000000000000000000000"}}]}"#,
    )
    .unwrap();

    let out = af()
        .args([
            "--json",
            "core",
            "standards",
            "collect",
            core.path().to_str().unwrap(),
            "--build-root",
            build_root.path().to_str().unwrap(),
            "--declare",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success(), "collect failed: {:?}", out);
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");

    assert_eq!(value["status"], "passed");
    assert!(core
        .path()
        .join("reports/standards/core-lint.json")
        .is_file());
    assert!(core
        .path()
        .join("reports/standards/core-sim.json")
        .is_file());
    assert!(core
        .path()
        .join("reports/standards/core-formal.json")
        .is_file());
    assert!(core.path().join("hbom/af_mod_add.spdx.json").is_file());
    let manifest = fs::read_to_string(core.path().join("af-core.toml")).unwrap();
    assert!(manifest.contains("kind = \"native-lint\""));
    assert!(manifest.contains("path = \"reports/standards/core-lint.json\""));
    assert!(manifest.contains("kind = \"simulation-report\""));
    assert!(manifest.contains("path = \"reports/standards/core-sim.json\""));
    assert!(manifest.contains("kind = \"formal-report\""));
    assert!(manifest.contains("path = \"reports/standards/core-formal.json\""));
    assert!(manifest.contains("kind = \"spdx-hbom\""));

    let second = af()
        .args([
            "--json",
            "core",
            "standards",
            "collect",
            core.path().to_str().unwrap(),
            "--build-root",
            build_root.path().to_str().unwrap(),
            "--declare",
        ])
        .output()
        .expect("execute");
    assert!(
        second.status.success(),
        "second collect failed: {:?}",
        second
    );
    let value: Value = serde_json::from_slice(&second.stdout).expect("JSON");
    assert_eq!(
        value["manifest_artifacts_added"].as_array().unwrap().len(),
        0
    );
}

#[test]
fn core_regs_scaffold_and_check_systemrdl_evidence() {
    let core = temp_af_mod_add();
    let out = af()
        .args([
            "--json",
            "core",
            "regs",
            "scaffold",
            core.path().to_str().unwrap(),
            "--declare",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success(), "regs scaffold failed: {:?}", out);
    let value: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert_eq!(value["status"], "passed");
    let rdl_path = core.path().join("regs/af_mod_add.rdl");
    let rdl = fs::read_to_string(&rdl_path).unwrap();
    assert!(rdl.contains("addrmap af_mod_add"));
    assert!(rdl.contains("field"));

    let manifest = fs::read_to_string(core.path().join("af-core.toml")).unwrap();
    assert!(manifest.contains("kind = \"systemrdl\""));
    assert!(manifest.contains("path = \"regs/af_mod_add.rdl\""));

    let check = af()
        .args([
            "--json",
            "core",
            "regs",
            "check",
            core.path().to_str().unwrap(),
        ])
        .output()
        .expect("execute");
    assert!(check.status.success(), "regs check failed: {:?}", check);
    let value: Value = serde_json::from_slice(&check.stdout).expect("JSON");
    assert_eq!(value["status"], "passed");
    assert_eq!(value["validation_status"], "semantic-valid");
}

#[test]
fn safety_domain_scaffold_emits_domain_sections_without_cert_claims() {
    let core = temp_af_mod_add();
    let out = af()
        .args([
            "--json",
            "core",
            "standards",
            "scaffold",
            core.path().to_str().unwrap(),
            "--profile",
            "fpga-ip-core-v1",
            "--safety-domain",
            "automotive",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success(), "scaffold failed: {:?}", out);
    let safety = fs::read_to_string(core.path().join("safety/safety_manual.md")).unwrap();
    assert!(safety.contains("ISO 26262"));
    assert!(safety.contains("SEooC"));
    assert!(safety.contains("SPFM"));
    assert!(safety.contains("not safety-certified"));
}

#[test]
fn security_scaffold_derives_ports_interfaces_assets() {
    let core = temp_af_mod_add();
    let out = af()
        .args([
            "--json",
            "core",
            "standards",
            "scaffold",
            core.path().to_str().unwrap(),
            "--profile",
            "fpga-ip-core-v1",
        ])
        .output()
        .expect("execute");
    assert!(out.status.success(), "scaffold failed: {:?}", out);
    let threat = fs::read_to_string(core.path().join("security/threat_model.md")).unwrap();
    let sa_edi: Value =
        serde_json::from_slice(&fs::read(core.path().join("security/sa-edi.json")).unwrap())
            .expect("SA-EDI JSON");
    assert!(threat.contains("i_valid"));
    assert!(threat.contains("o_ready"));
    assert!(threat.contains("not a security certification"));
    assert!(sa_edi["assets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|asset| asset["name"] == "i_valid"));
}

#[test]
fn standards_ready_example_passes_commercial_baseline_gate() {
    let core = repo_root().join("examples/standards-ready-core");

    let manifest = af()
        .args([
            "--json",
            "manifest",
            "validate",
            core.join("af-core.toml").to_str().unwrap(),
        ])
        .output()
        .expect("execute manifest validate");
    assert!(
        manifest.status.success(),
        "manifest validate failed: {:?}",
        manifest
    );

    let core_check = af()
        .args(["--json", "core", "check", core.to_str().unwrap()])
        .output()
        .expect("execute core check");
    assert!(
        core_check.status.success(),
        "core check failed: {:?}",
        core_check
    );

    let regs = af()
        .args(["--json", "core", "regs", "check", core.to_str().unwrap()])
        .output()
        .expect("execute regs check");
    assert!(regs.status.success(), "regs check failed: {:?}", regs);

    let audit = af()
        .args([
            "--json",
            "core",
            "standards",
            "spdx-audit",
            core.to_str().unwrap(),
        ])
        .output()
        .expect("execute spdx audit");
    assert!(audit.status.success(), "spdx audit failed: {:?}", audit);
    let audit_json: Value = serde_json::from_slice(&audit.stdout).expect("audit JSON");
    assert_eq!(audit_json["status"], "passed");

    let standards = af()
        .args([
            "--json",
            "core",
            "standards",
            "check",
            core.to_str().unwrap(),
            "--profile",
            "fpga-ip-core-v1",
            "--strict",
        ])
        .output()
        .expect("execute standards check");
    assert!(
        standards.status.success(),
        "standards check failed: {:?}",
        standards
    );
    let value: Value = serde_json::from_slice(&standards.stdout).expect("standards JSON");
    assert_eq!(value["status"], "passed");
    assert_eq!(
        value["gates"]["commercial_baseline_ready"]["status"],
        "passed"
    );
    assert!(value["limitations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|limitation| limitation
            .as_str()
            .unwrap()
            .contains("does not claim certification")));
}
