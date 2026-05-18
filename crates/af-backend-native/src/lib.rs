// SPDX-License-Identifier: Apache-2.0
use af_backend::{
    AfBackend, BackendCapability, BackendDiagnostic, BackendReport, BackendStatus,
    DiagnosticSeverity, ToolVersion,
};
use af_core::{check_core, CoreError};
use af_manifest::CoreManifest;
use af_rtl_inspector::RtlIssueSeverity;
use std::path::Path;

pub struct NativeBackend;

pub fn capabilities() -> Vec<BackendCapability> {
    NativeBackend.capabilities()
}

impl AfBackend for NativeBackend {
    fn name(&self) -> &'static str {
        "native"
    }

    fn capabilities(&self) -> Vec<BackendCapability> {
        vec![
            BackendCapability {
                name: "native-portable-core-check".to_string(),
                supported: true,
                detail: Some(
                    "AccelFury built-in manifest and portable Verilog-2001 base-core checks; no external tools are executed."
                        .to_string(),
                ),
            },
            BackendCapability {
                name: "native-portability-lint".to_string(),
                supported: true,
                detail: Some(
                    "Rejects SystemVerilog constructs, vendor primitives, AXI-only markers, hidden PLL markers, and implicit port style in portable base RTL."
                        .to_string(),
                ),
            },
            BackendCapability {
                name: "native-simulation".to_string(),
                supported: false,
                detail: Some(
                    "Native simulation is planned; use Verilator for executable simulation until the built-in engine exists."
                        .to_string(),
                ),
            },
        ]
    }

    fn doctor(&self) -> Result<BackendReport, af_backend::BackendError> {
        let mut report = BackendReport::new(self.name(), BackendStatus::Passed);
        report.tool_versions.push(ToolVersion::available(
            "af-native",
            env!("CARGO_PKG_VERSION"),
        ));
        report.limitations.push(
            "Native backend performs structural and portability checks only; it does not synthesize, simulate, place, route, or prove RTL."
                .to_string(),
        );
        Ok(report)
    }

    fn lint(
        &self,
        _manifest: &CoreManifest,
        core_dir: &Path,
        _build_root: &Path,
    ) -> Result<BackendReport, af_backend::BackendError> {
        Ok(report_from_core_check(self.name(), core_dir))
    }

    fn sim(
        &self,
        _manifest: &CoreManifest,
        _core_dir: &Path,
        _build_root: &Path,
    ) -> Result<BackendReport, af_backend::BackendError> {
        let mut report = BackendReport::new(self.name(), BackendStatus::Unavailable);
        report.tool_versions.push(ToolVersion::available(
            "af-native",
            env!("CARGO_PKG_VERSION"),
        ));
        report.warnings.push(
            "Native simulation is not implemented; use Verilator for executable simulation."
                .to_string(),
        );
        report.limitations.push(
            "Native backend currently replaces external services only for core structure and portable RTL linting."
                .to_string(),
        );
        Ok(report)
    }
}

fn report_from_core_check(backend: &str, core_dir: &Path) -> BackendReport {
    match check_core(core_dir) {
        Ok(core_report) => {
            let mut report = BackendReport::new(backend, BackendStatus::Passed);
            report.tool_versions.push(ToolVersion::available(
                "af-native",
                env!("CARGO_PKG_VERSION"),
            ));
            report
                .artifacts
                .extend(core_report.inspection.scanned_files);
            report.warnings.extend(core_report.warnings);
            report.limitations.extend(core_report.limitations);
            report.metrics.insert(
                "scanned_files".to_string(),
                report.artifacts.len().to_string(),
            );
            for (check, status) in core_report.inspection.checks {
                report.metrics.insert(check, status);
            }
            report
        }
        Err(CoreError::CheckFailed { report }) => {
            let mut backend_report = BackendReport::new(backend, BackendStatus::Failed);
            backend_report.tool_versions.push(ToolVersion::available(
                "af-native",
                env!("CARGO_PKG_VERSION"),
            ));
            backend_report
                .artifacts
                .extend(report.inspection.scanned_files.clone());
            backend_report.warnings.extend(report.warnings.clone());
            backend_report
                .limitations
                .extend(report.limitations.clone());
            backend_report.metrics.insert(
                "scanned_files".to_string(),
                backend_report.artifacts.len().to_string(),
            );
            for (check, status) in &report.inspection.checks {
                backend_report
                    .metrics
                    .insert(check.to_string(), status.to_string());
            }
            backend_report
                .diagnostics
                .extend(
                    report
                        .inspection
                        .issues
                        .iter()
                        .map(|issue| BackendDiagnostic {
                            code: issue.code.clone(),
                            severity: match &issue.severity {
                                RtlIssueSeverity::Error => DiagnosticSeverity::Error,
                                RtlIssueSeverity::Warning => DiagnosticSeverity::Warning,
                            },
                            message: issue.message.clone(),
                            hint: Some(issue.hint.clone()),
                        }),
                );
            backend_report
        }
        Err(err) => {
            let mut report = BackendReport::new(backend, BackendStatus::Failed);
            report.tool_versions.push(ToolVersion::available(
                "af-native",
                env!("CARGO_PKG_VERSION"),
            ));
            report.diagnostics.push(BackendDiagnostic {
                code: err.code().to_string(),
                severity: DiagnosticSeverity::Error,
                message: err.to_string(),
                hint: Some(err.hint().to_string()),
            });
            report
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn native_lint_passes_portable_core_without_external_commands() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(
            dir.path().join("af-core.toml"),
            r#"
af_version = "0.2"
name = "demo"
vendor = "accelfury"
library = "ip"
core = "demo"
version = "0.1.0"

[metadata]
license = "AccelFury Source Available License v1.0"

[rtl]
top = "demo"
language = "verilog-2001"

[sources]
files = ["rtl/demo.v"]

[[clocks]]
name = "clk"
port = "clk"

[[resets]]
name = "rst_n"
active = "low"

[[ports]]
name = "clk"
direction = "input"
width = 1

[[ports]]
name = "rst_n"
direction = "input"
width = 1

[[ports]]
name = "done"
direction = "output"
width = 1
"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("LICENSE"),
            "AccelFury Source Available License v1.0\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("COMMERCIAL-LICENSE.md"),
            "Closed-source or proprietary use requires a separate paid commercial license. Support and warranty terms are separate.\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("rtl/demo.v"),
            r#"`default_nettype none
module demo (
  input wire clk,
  input wire rst_n,
  output reg done
);
  always @(posedge clk) begin
    if (!rst_n) begin
      done <= 1'b0;
    end else begin
      done <= 1'b1;
    end
  end
endmodule
`default_nettype wire
"#,
        )
        .unwrap();

        let manifest = CoreManifest::from_path(dir.path().join("af-core.toml")).unwrap();
        let report = NativeBackend
            .lint(&manifest, dir.path(), dir.path())
            .unwrap();
        assert_eq!(report.status, BackendStatus::Passed);
        assert!(report.commands.is_empty());
        assert_eq!(
            report.metrics.get("portable_verilog_policy"),
            Some(&"pass".to_string())
        );
    }

    #[test]
    fn native_lint_reports_portability_failures_as_diagnostics() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rtl")).unwrap();
        fs::write(
            dir.path().join("af-core.toml"),
            r#"
af_version = "0.2"
name = "demo"
vendor = "accelfury"
library = "ip"
core = "demo"
version = "0.1.0"

[rtl]
top = "demo"
language = "verilog-2001"

[sources]
files = ["rtl/demo.v"]

[[clocks]]
name = "clk"
port = "clk"

[[resets]]
name = "rst_n"
active = "low"

[[ports]]
name = "clk"
direction = "input"
width = 1

[[ports]]
name = "rst_n"
direction = "input"
width = 1
"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("rtl/demo.v"),
            r#"`default_nettype none
module demo (
  input logic clk,
  input logic rst_n
);
endmodule
`default_nettype wire
"#,
        )
        .unwrap();

        let manifest = CoreManifest::from_path(dir.path().join("af-core.toml")).unwrap();
        let report = NativeBackend
            .lint(&manifest, dir.path(), dir.path())
            .unwrap();
        assert_eq!(report.status, BackendStatus::Failed);
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "AF_PORTABLE_SYSTEMVERILOG_CONSTRUCT"));
    }
}
