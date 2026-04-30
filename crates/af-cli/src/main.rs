use af_backend::{
    AfBackend, BackendStatus, CommandRecord, CommandRunner, CommandSpec, ProcessCommandRunner,
    ToolVersion,
};
use af_backend_verilator::VerilatorBackend;
use af_board_db::BoardDbError;
use af_core::{check_core, CoreError};
use af_manifest::{CoreManifest, ManifestError, ManifestValidationReport};
use af_report::{write_reports, AfReport, ReportError};
use af_vectors::{generate_mod_add_vectors, GenerateConfig};
use af_wrapper_gen::{generate_wrapper, WrapperGenError, WrapperTarget};
use clap::{ArgAction, Parser, Subcommand};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "af", version, about = "AccelFury IP Toolchain")]
struct Cli {
    #[arg(long, global = true)]
    json: bool,
    #[arg(long, global = true, action = ArgAction::Count)]
    verbose: u8,
    #[arg(long, global = true)]
    quiet: bool,
    #[arg(long, global = true, default_value = ".af-build")]
    build_root: PathBuf,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Doctor,
    Manifest {
        #[command(subcommand)]
        command: ManifestCommand,
    },
    Core {
        #[command(subcommand)]
        command: CoreCommand,
    },
    Registry {
        #[command(subcommand)]
        command: RegistryCommand,
    },
    Board {
        #[command(subcommand)]
        command: BoardCommand,
    },
    Vectors {
        #[command(subcommand)]
        command: VectorsCommand,
    },
    Wrapper {
        #[command(subcommand)]
        command: WrapperCommand,
    },
    Ci {
        #[command(subcommand)]
        command: CiCommand,
    },
}

#[derive(Subcommand, Debug)]
enum ManifestCommand {
    Validate { path: PathBuf },
}

#[derive(Subcommand, Debug)]
enum CoreCommand {
    Check {
        core_dir: PathBuf,
    },
    New {
        core_dir: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "examples")]
        library: String,
        #[arg(long, default_value = "systemverilog")]
        language: String,
    },
    Lint {
        core_dir: PathBuf,
        #[arg(long, default_value = "verilator")]
        backend: String,
    },
    Sim {
        core_dir: PathBuf,
        #[arg(long, default_value = "verilator")]
        backend: String,
    },
    Report {
        input: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum RegistryCommand {
    Check {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum BoardCommand {
    Matrix {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    New {
        #[arg(long)]
        board_id: String,
        #[arg(long)]
        vendor: String,
        #[arg(long)]
        family: String,
        #[arg(long, value_name = "format")]
        constraint_format: String,
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum VectorsCommand {
    Generate {
        #[arg(long, default_value = "cores/af-mod-add/vectors/af_mod_add_basic.json")]
        basic_out: PathBuf,
        #[arg(
            long,
            default_value = "cores/af-mod-add/vectors/af_mod_add_random.json"
        )]
        random_out: PathBuf,
        #[arg(long, default_value = "cores/af-mod-add/vectors/af_mod_add_random.svh")]
        svh_out: PathBuf,
        #[arg(long, default_value_t = 64)]
        count: usize,
        #[arg(long, default_value = "0x1234567890ABCDEF")]
        seed: String,
    },
}

#[derive(Subcommand, Debug)]
enum WrapperCommand {
    Generate {
        core_dir: PathBuf,
        #[arg(long)]
        target: String,
    },
}

#[derive(Subcommand, Debug)]
enum CiCommand {
    Generate {
        #[arg(long)]
        target: String,
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

#[derive(Debug)]
struct CliOutput {
    human: String,
    json: Value,
}

#[derive(Debug, Serialize)]
struct ErrorPayload {
    code: String,
    message: String,
    hint: String,
    exit_code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
}

#[derive(Debug)]
struct CliError {
    payload: ErrorPayload,
}

impl CliError {
    fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        hint: impl Into<String>,
        exit_code: i32,
    ) -> Self {
        Self {
            payload: ErrorPayload {
                code: code.into(),
                message: message.into(),
                hint: hint.into(),
                exit_code,
                details: None,
            },
        }
    }

    fn with_details<T: Serialize>(mut self, details: &T) -> Self {
        self.payload.details = serde_json::to_value(details).ok();
        self
    }
}

impl From<ManifestError> for CliError {
    fn from(err: ManifestError) -> Self {
        let mut cli = CliError::new(err.code(), err.to_string(), err.hint(), err.exit_code());
        if let ManifestError::Validation { issues } = &err {
            cli = cli.with_details(&json!({ "issues": issues }));
        }
        cli
    }
}

impl From<CoreError> for CliError {
    fn from(err: CoreError) -> Self {
        let mut cli = CliError::new(err.code(), err.to_string(), err.hint(), err.exit_code());
        if let CoreError::CheckFailed { report } = &err {
            cli = cli.with_details(report);
        }
        cli
    }
}

impl From<WrapperGenError> for CliError {
    fn from(err: WrapperGenError) -> Self {
        CliError::new(err.code(), err.to_string(), err.hint(), err.exit_code())
    }
}

impl From<ReportError> for CliError {
    fn from(err: ReportError) -> Self {
        CliError::new(err.code(), err.to_string(), err.hint(), err.exit_code())
    }
}

impl From<af_ci::CiError> for CliError {
    fn from(err: af_ci::CiError) -> Self {
        CliError::new(err.code(), err.to_string(), err.hint(), err.exit_code())
    }
}

impl From<BoardDbError> for CliError {
    fn from(err: BoardDbError) -> Self {
        let mut cli = CliError::new(err.code(), err.to_string(), err.hint(), err.exit_code());
        if let BoardDbError::Validation { issues } = &err {
            cli = cli.with_details(&json!({ "issues": issues }));
        }
        cli
    }
}

fn main() {
    let cli = Cli::parse();
    init_tracing(cli.verbose, cli.quiet);
    match execute(&cli) {
        Ok(output) => {
            if cli.json {
                println!("{}", to_pretty_json(&output.json));
            } else if !cli.quiet {
                println!("{}", output.human);
            }
        }
        Err(err) => {
            if cli.json {
                println!("{}", to_pretty_json(&err.payload));
            } else if !cli.quiet {
                eprintln!(
                    "{}: {}\nhint: {}",
                    err.payload.code, err.payload.message, err.payload.hint
                );
            }
            std::process::exit(err.payload.exit_code);
        }
    }
}

fn init_tracing(verbose: u8, quiet: bool) {
    if quiet {
        return;
    }
    let level = match verbose {
        0 => "warn",
        1 => "info",
        _ => "debug",
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}

fn execute(cli: &Cli) -> Result<CliOutput, CliError> {
    match &cli.command {
        Commands::Doctor => doctor(),
        Commands::Manifest { command } => match command {
            ManifestCommand::Validate { path } => manifest_validate(path),
        },
        Commands::Core { command } => match command {
            CoreCommand::Check { core_dir } => core_check(core_dir, &cli.build_root),
            CoreCommand::New {
                core_dir,
                name,
                library,
                language,
            } => core_new(core_dir, name, library, language),
            CoreCommand::Lint { core_dir, backend } => {
                core_lint(core_dir, &cli.build_root, backend)
            }
            CoreCommand::Sim { core_dir, backend } => core_sim(core_dir, &cli.build_root, backend),
            CoreCommand::Report { input } => core_report(input, &cli.build_root),
        },
        Commands::Registry { command } => match command {
            RegistryCommand::Check { root } => registry_check(root),
        },
        Commands::Board { command } => match command {
            BoardCommand::Matrix { root, output } => board_matrix(root, output.as_ref()),
            BoardCommand::New {
                board_id,
                vendor,
                family,
                constraint_format,
                root,
            } => board_new(root, board_id, vendor, family, constraint_format),
        },
        Commands::Vectors { command } => match command {
            VectorsCommand::Generate {
                basic_out,
                random_out,
                svh_out,
                count,
                seed,
            } => vectors_generate(basic_out, random_out, svh_out, *count, seed),
        },
        Commands::Wrapper { command } => match command {
            WrapperCommand::Generate { core_dir, target } => {
                wrapper_generate(core_dir, &cli.build_root, target)
            }
        },
        Commands::Ci { command } => match command {
            CiCommand::Generate { target, output } => ci_generate(target, output.as_ref()),
        },
    }
}

fn doctor() -> Result<CliOutput, CliError> {
    let runner = ProcessCommandRunner;
    let verilator = VerilatorBackend::process()
        .doctor()
        .expect("doctor is infallible");
    let (fusesoc_version, fusesoc_commands) = probe_tool(&runner, "fusesoc", &["--version"]);

    let mut report = AfReport::new("passed");
    report.merge_backend(&verilator);
    report.tool_versions.push(fusesoc_version.clone());
    report.commands.extend(fusesoc_commands);
    report.limitations.push(
        "MVP doctor checks tool visibility only; it does not validate vendor bitstream flows."
            .to_string(),
    );

    if report.tool_versions.iter().any(|tool| !tool.available) {
        report.status = "warning".to_string();
        report
            .warnings
            .push("One or more optional backend tools are unavailable.".to_string());
    }

    Ok(CliOutput {
        human: format!("doctor {}", report.status),
        json: json!(report),
    })
}

fn manifest_validate(path: &Path) -> Result<CliOutput, CliError> {
    let manifest = CoreManifest::from_path(path)?;
    let report = ManifestValidationReport {
        valid: true,
        issues: Vec::new(),
    };
    Ok(CliOutput {
        human: format!("manifest valid: {}", manifest.vlnv()),
        json: json!({
            "status": "passed",
            "manifest": manifest,
            "validation": report,
        }),
    })
}

fn core_new(
    core_dir: &Path,
    name: &str,
    library: &str,
    language: &str,
) -> Result<CliOutput, CliError> {
    if language != "systemverilog" {
        return Err(CliError::new(
            "AF_CORE_NEW_LANGUAGE_UNSUPPORTED",
            format!("core new language `{language}` is unsupported"),
            "Use --language systemverilog for the built-in scaffold.",
            2,
        ));
    }
    let module = to_module_ident(name)?;
    let rtl_dir = core_dir.join("rtl");
    fs::create_dir_all(&rtl_dir).map_err(|err| {
        CliError::new(
            "AF_CORE_NEW_CREATE_DIR_FAILED",
            format!("failed to create `{}`: {err}", rtl_dir.display()),
            "Check filesystem permissions and choose a writable core directory.",
            5,
        )
    })?;

    let manifest = format!(
        r#"af_version = "0.2"
name = "{name}"
vendor = "accelfury"
library = "{library}"
core = "{module}"
version = "0.1.0"
known_limitations = ["Generated scaffold; no timing, board, or hardware validation claims."]

[metadata]
license = "Apache-2.0"
authors = ["AccelFury contributors"]
description = "Generated AccelFury core scaffold."

[rtl]
top = "{module}"
language = "systemverilog"
default_clock = "clk"
default_reset = "rst_n"

[sources]
files = ["rtl/{module}.sv"]
include_dirs = []

[[clocks]]
name = "clk"

[[resets]]
name = "rst_n"
active = "low"

[[ports]]
name = "clk"
direction = "input"
width = 1
clock = "clk"

[[ports]]
name = "rst_n"
direction = "input"
width = 1
reset = "rst_n"

[[ports]]
name = "enable"
direction = "input"
width = 1
clock = "clk"
reset = "rst_n"

[[ports]]
name = "done"
direction = "output"
width = 1
clock = "clk"
reset = "rst_n"

[backend_compatibility]
verilator = true
fusesoc = true
"#
    );
    let rtl = format!(
        r#"// SPDX-License-Identifier: Apache-2.0
module {module} (
  input  logic clk,
  input  logic rst_n,
  input  logic enable,
  output logic done
);
  always_ff @(posedge clk or negedge rst_n) begin
    if (!rst_n) begin
      done <= 1'b0;
    end else begin
      done <= enable;
    end
  end
endmodule
"#
    );
    write_new_file(&core_dir.join("af-core.toml"), manifest.as_bytes())?;
    write_new_file(&rtl_dir.join(format!("{module}.sv")), rtl.as_bytes())?;
    let manifest = CoreManifest::from_path(core_dir.join("af-core.toml"))?;
    Ok(CliOutput {
        human: format!("core scaffold written: {}", core_dir.display()),
        json: json!({
            "status": "passed",
            "core_dir": core_dir,
            "manifest": manifest,
        }),
    })
}

fn registry_check(root: &Path) -> Result<CliOutput, CliError> {
    let report = af_board_db::check_registry(root)?;
    if report.valid {
        Ok(CliOutput {
            human: format!(
                "registry check passed: {} boards, {} aliases",
                report.board_count, report.alias_count
            ),
            json: json!({
                "status": "passed",
                "registry": report,
            }),
        })
    } else {
        Err(CliError::new(
            "AF_REGISTRY_INVALID",
            "registry check failed",
            "Fix the listed board registry issues.",
            2,
        )
        .with_details(&report))
    }
}

fn board_matrix(root: &Path, output: Option<&PathBuf>) -> Result<CliOutput, CliError> {
    let matrix = af_board_db::render_board_matrix(root)?;
    if let Some(output) = output {
        if let Some(parent) = output.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|err| {
                    CliError::new(
                        "AF_BOARD_MATRIX_CREATE_DIR_FAILED",
                        format!("failed to create `{}`: {err}", parent.display()),
                        "Check filesystem permissions and the selected output path.",
                        5,
                    )
                })?;
            }
        }
        fs::write(output, &matrix).map_err(|err| {
            CliError::new(
                "AF_BOARD_MATRIX_WRITE_FAILED",
                format!("failed to write `{}`: {err}", output.display()),
                "Check filesystem permissions and the selected output path.",
                5,
            )
        })?;
    }
    Ok(CliOutput {
        human: output
            .map(|path| format!("board matrix written: {}", path.display()))
            .unwrap_or_else(|| matrix.clone()),
        json: json!({
            "status": "passed",
            "matrix": matrix,
            "output": output,
        }),
    })
}

fn board_new(
    root: &Path,
    board_id: &str,
    vendor: &str,
    family: &str,
    constraint_format: &str,
) -> Result<CliOutput, CliError> {
    let constraint_file = constraint_file_name(constraint_format).ok_or_else(|| {
        CliError::new(
            "AF_BOARD_CONSTRAINT_FORMAT_UNSUPPORTED",
            format!("unsupported constraint format `{constraint_format}`"),
            "Use one of: cst, pcf, lpf, qsf, sdc, xdc, pdc.",
            2,
        )
    })?;
    let board_dir = root.join("boards").join(vendor).join(board_id);
    let constraint_dir = board_dir.join("constraints");
    let top_dir = board_dir.join("top");
    let registry_path = root.join("registries/boards.registry.json");
    let mut registry: af_board_db::RegistryBoardsFile = read_json_file(&registry_path)?;
    if registry
        .boards
        .iter()
        .any(|board| board.board_id == board_id)
    {
        return Err(CliError::new(
            "AF_BOARD_ALREADY_REGISTERED",
            format!("board `{board_id}` already exists in registry"),
            "Use a new board id or edit the existing board entry.",
            2,
        ));
    }

    fs::create_dir_all(&constraint_dir).map_err(|err| {
        CliError::new(
            "AF_BOARD_CREATE_DIR_FAILED",
            format!("failed to create `{}`: {err}", constraint_dir.display()),
            "Check filesystem permissions and the selected root.",
            5,
        )
    })?;
    fs::create_dir_all(&top_dir).map_err(|err| {
        CliError::new(
            "AF_BOARD_CREATE_DIR_FAILED",
            format!("failed to create `{}`: {err}", top_dir.display()),
            "Check filesystem permissions and the selected root.",
            5,
        )
    })?;

    write_new_file(
        &board_dir.join("README.md"),
        format!("# {board_id}\n\nTemplate board target for `{family}`.\n").as_bytes(),
    )?;
    write_new_file(
        &board_dir.join("bringup.md"),
        b"# Bringup\n\nDraft only. Verify schematic, power rails, clocks, resets, and every pin before programming hardware.\n",
    )?;
    write_new_file(
        &board_dir.join("board.status.json"),
        format!(
            r#"{{
  "board": "{board_id}",
  "status": {{
    "template": true,
    "sim": "not_applicable",
    "synthesis": "not_measured",
    "pnr": "not_measured",
    "hardware_bringup": "not_tested"
  }},
  "warnings": [
    "Draft template generated by af board new; no pinout or hardware claims are verified."
  ]
}}
"#
        )
        .as_bytes(),
    )?;
    write_new_file(
        &constraint_dir.join("README.md"),
        b"# Constraints\n\nPlaceholder only. Replace with verified board constraints before hardware use.\n",
    )?;
    write_new_file(
        &constraint_dir.join(constraint_file),
        b"# Placeholder constraints. Do not use for hardware until verified.\n",
    )?;
    write_new_file(
        &top_dir.join("af_board_top.sv"),
        r#"// SPDX-License-Identifier: CERN-OHL-S-2.0
module af_board_top (
  input  logic clk,
  input  logic rst_n
);
  logic unused;
  assign unused = clk ^ rst_n;
endmodule
"#
        .as_bytes(),
    )?;

    registry.boards.push(af_board_db::BoardEntry {
        board_id: board_id.to_string(),
        display_name: board_id.replace('_', " "),
        vendor: vendor.to_string(),
        fpga_family: family.to_string(),
        fpga_part_if_known_or_template: family.to_string(),
        logic_size_class: "unknown".to_string(),
        dsp_class: "unknown".to_string(),
        memory_class: "unknown".to_string(),
        high_speed_io_class: "unknown".to_string(),
        default_toolchain: "unknown".to_string(),
        alternative_toolchains: Vec::new(),
        constraint_format: constraint_format.to_string(),
        board_dir: format!("boards/{vendor}/{board_id}"),
        exact_pinout_status: "draft_placeholder".to_string(),
        safe_for_beginner: false,
        suggested_ip_classes: Vec::new(),
        excluded_ip_classes: Vec::new(),
        notes: "Generated draft target. Pin mapping must be verified before use.".to_string(),
    });
    write_json_file(&registry_path, &registry)?;

    Ok(CliOutput {
        human: format!(
            "board scaffold written and registered: {}",
            board_dir.display()
        ),
        json: json!({
            "status": "passed",
            "board_id": board_id,
            "board_dir": board_dir,
            "registry": registry_path,
        }),
    })
}

fn vectors_generate(
    basic_out: &Path,
    random_out: &Path,
    svh_out: &Path,
    count: usize,
    seed: &str,
) -> Result<CliOutput, CliError> {
    let report = generate_mod_add_vectors(&GenerateConfig {
        basic_out: basic_out.to_path_buf(),
        random_out: random_out.to_path_buf(),
        svh_out: svh_out.to_path_buf(),
        count,
        seed: seed.to_string(),
    })
    .map_err(|err| {
        CliError::new(
            "AF_VECTORS_GENERATE_FAILED",
            err.to_string(),
            "Check vector output paths and seed format.",
            5,
        )
    })?;
    Ok(CliOutput {
        human: format!(
            "vectors generated: {} basic, {} random",
            report.basic_count, report.random_count
        ),
        json: json!({
            "status": "passed",
            "vectors": report,
        }),
    })
}

fn core_check(core_dir: &Path, build_root: &Path) -> Result<CliOutput, CliError> {
    let report = check_core(core_dir)?;
    let mut af_report = AfReport::for_core("passed", &report.manifest);
    af_report.artifacts.extend(
        report
            .inspection
            .scanned_files
            .iter()
            .map(|path| path.display().to_string()),
    );
    af_report.warnings.extend(report.warnings.clone());
    let written = write_reports(build_root.join("reports"), "core-check", &af_report)?;

    Ok(CliOutput {
        human: format!(
            "core check passed: {} (reports: {}, {})",
            report.manifest.vlnv(),
            written.json.display(),
            written.markdown.display()
        ),
        json: json!({
            "status": "passed",
            "check": report,
            "reports": written,
        }),
    })
}

fn core_lint(core_dir: &Path, build_root: &Path, backend: &str) -> Result<CliOutput, CliError> {
    let checked = check_core(core_dir)?;
    let backend_report = match backend {
        "verilator" => VerilatorBackend::process().lint(&checked.manifest, core_dir, build_root),
        other => Err(af_backend::BackendError::Unsupported {
            backend: other.to_string(),
        }),
    }
    .map_err(|err| CliError::new(err.code(), err.to_string(), err.hint(), err.exit_code()))?;

    let mut af_report = AfReport::for_core(status_text(&backend_report.status), &checked.manifest);
    af_report.merge_backend(&backend_report);
    let written = write_reports(build_root.join("reports"), "core-lint", &af_report)?;

    let status = backend_report.status.clone();
    match status {
        BackendStatus::Passed => Ok(CliOutput {
            human: format!(
                "core lint passed with {backend} (reports: {}, {})",
                written.json.display(),
                written.markdown.display()
            ),
            json: json!({
                "status": "passed",
                "backend_report": backend_report,
                "reports": written,
            }),
        }),
        _ => {
            let detail = json!({
                "backend_report": backend_report,
                "reports": written,
            });
            let (code, message, exit_code) = if status == BackendStatus::Unavailable {
                (
                    "AF_BACKEND_UNAVAILABLE",
                    "core lint backend is unavailable",
                    4,
                )
            } else {
                ("AF_BACKEND_FAILED", "core lint backend command failed", 3)
            };
            Err(CliError::new(
                code,
                message,
                "Inspect backend command details in the report.",
                exit_code,
            )
            .with_details(&detail))
        }
    }
}

fn core_sim(core_dir: &Path, build_root: &Path, backend: &str) -> Result<CliOutput, CliError> {
    let checked = check_core(core_dir)?;
    let backend_report = match backend {
        "verilator" => VerilatorBackend::process().sim(&checked.manifest, core_dir, build_root),
        other => Err(af_backend::BackendError::Unsupported {
            backend: other.to_string(),
        }),
    }
    .map_err(|err| CliError::new(err.code(), err.to_string(), err.hint(), err.exit_code()))?;

    let mut af_report = AfReport::for_core(status_text(&backend_report.status), &checked.manifest);
    af_report.merge_backend(&backend_report);
    let written = write_reports(build_root.join("reports"), "core-sim", &af_report)?;

    match backend_report.status {
        BackendStatus::Passed => Ok(CliOutput {
            human: format!(
                "core sim passed with {backend} (reports: {}, {})",
                written.json.display(),
                written.markdown.display()
            ),
            json: json!({
                "status": "passed",
                "backend_report": backend_report,
                "reports": written,
            }),
        }),
        _ => {
            let detail = json!({
                "backend_report": backend_report,
                "reports": written,
            });
            let status = detail["backend_report"]["status"]
                .as_str()
                .unwrap_or("Failed")
                .to_string();
            let (code, message, exit_code) = if status == "Unavailable" {
                (
                    "AF_BACKEND_UNAVAILABLE",
                    "core sim backend is unavailable",
                    4,
                )
            } else {
                ("AF_BACKEND_FAILED", "core sim backend command failed", 3)
            };
            Err(CliError::new(
                code,
                message,
                "Inspect backend command details in the report.",
                exit_code,
            )
            .with_details(&detail))
        }
    }
}

fn core_report(input: &Path, build_root: &Path) -> Result<CliOutput, CliError> {
    let mut report = if input.join("af-core.toml").is_file() {
        let checked = check_core(input)?;
        let mut report = AfReport::for_core("passed", &checked.manifest);
        report.warnings.extend(checked.warnings);
        report.artifacts.extend(
            checked
                .inspection
                .scanned_files
                .iter()
                .map(|path| path.display().to_string()),
        );
        report
    } else {
        let mut report = AfReport::new("passed");
        report.warnings.push(
            "Input did not contain af-core.toml; generated an artifact report for the build directory."
                .to_string(),
        );
        report.artifacts.extend(collect_artifacts(input));
        report.limitations.push(
            "Build-directory reports cannot reconstruct manifest metadata unless af-core.toml is present."
                .to_string(),
        );
        report
    };
    if report.artifacts.is_empty() {
        report
            .warnings
            .push("No artifacts were discovered for the report input.".to_string());
    }
    let written = write_reports(build_root.join("reports"), "core-report", &report)?;
    Ok(CliOutput {
        human: format!(
            "core report written: {}, {}",
            written.json.display(),
            written.markdown.display()
        ),
        json: json!({
            "status": "passed",
            "report": report,
            "reports": written,
        }),
    })
}

fn wrapper_generate(
    core_dir: &Path,
    build_root: &Path,
    target: &str,
) -> Result<CliOutput, CliError> {
    let target = WrapperTarget::parse(target)?;
    let report = generate_wrapper(core_dir, build_root, target)?;
    Ok(CliOutput {
        human: format!(
            "wrapper generated: {}",
            report
                .artifacts
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        json: json!({
            "status": "passed",
            "wrapper": report,
        }),
    })
}

fn ci_generate(target: &str, output: Option<&PathBuf>) -> Result<CliOutput, CliError> {
    let output = output
        .cloned()
        .unwrap_or_else(|| PathBuf::from(".github/workflows/accelfury.yml"));
    let artifact = af_ci::write(target, &output)?;
    Ok(CliOutput {
        human: format!("CI workflow written: {}", artifact.path.display()),
        json: json!({
            "status": "passed",
            "ci": artifact,
        }),
    })
}

fn status_text(status: &BackendStatus) -> &'static str {
    match status {
        BackendStatus::Passed => "passed",
        BackendStatus::Failed => "failed",
        BackendStatus::Unavailable => "unavailable",
    }
}

fn probe_tool(
    runner: &impl CommandRunner,
    program: &str,
    args: &[&str],
) -> (ToolVersion, Vec<CommandRecord>) {
    let spec = CommandSpec::new(program).args(args.iter().copied());
    match runner.run(&spec) {
        Ok(output) => {
            let text = output
                .stdout
                .lines()
                .chain(output.stderr.lines())
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or("version output was empty")
                .to_string();
            (
                ToolVersion::available(program, text),
                vec![CommandRecord::from(output)],
            )
        }
        Err(err) => (
            ToolVersion::unavailable(program, err.to_string()),
            Vec::new(),
        ),
    }
}

fn collect_artifacts(input: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(input) else {
        return Vec::new();
    };
    let mut artifacts = entries
        .flatten()
        .map(|entry| entry.path().display().to_string())
        .collect::<Vec<_>>();
    artifacts.sort();
    artifacts
}

fn write_new_file(path: &Path, contents: &[u8]) -> Result<(), CliError> {
    if path.exists() {
        return Err(CliError::new(
            "AF_FILE_EXISTS",
            format!("refusing to overwrite existing file `{}`", path.display()),
            "Choose a new output path or remove the existing file intentionally.",
            2,
        ));
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|err| {
                CliError::new(
                    "AF_CREATE_DIR_FAILED",
                    format!("failed to create `{}`: {err}", parent.display()),
                    "Check filesystem permissions and the selected output path.",
                    5,
                )
            })?;
        }
    }
    fs::write(path, contents).map_err(|err| {
        CliError::new(
            "AF_WRITE_FAILED",
            format!("failed to write `{}`: {err}", path.display()),
            "Check filesystem permissions and the selected output path.",
            5,
        )
    })
}

fn read_json_file<T: DeserializeOwned>(path: &Path) -> Result<T, CliError> {
    let raw = fs::read_to_string(path).map_err(|err| {
        CliError::new(
            "AF_JSON_READ_FAILED",
            format!("failed to read `{}`: {err}", path.display()),
            "Check that the JSON file exists and is readable.",
            2,
        )
    })?;
    serde_json::from_str(&raw).map_err(|err| {
        CliError::new(
            "AF_JSON_PARSE_FAILED",
            format!("failed to parse `{}`: {err}", path.display()),
            "Fix the JSON syntax before retrying.",
            2,
        )
    })
}

fn write_json_file<T: Serialize>(path: &Path, value: &T) -> Result<(), CliError> {
    let mut payload = serde_json::to_string_pretty(value).map_err(|err| {
        CliError::new(
            "AF_JSON_SERIALIZE_FAILED",
            err.to_string(),
            "Report this bug with the data that could not be serialized.",
            1,
        )
    })?;
    payload.push('\n');
    fs::write(path, payload).map_err(|err| {
        CliError::new(
            "AF_JSON_WRITE_FAILED",
            format!("failed to write `{}`: {err}", path.display()),
            "Check filesystem permissions and the selected output path.",
            5,
        )
    })
}

fn to_module_ident(name: &str) -> Result<String, CliError> {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else if matches!(ch, '-' | '.') {
            out.push('_');
        } else {
            return Err(CliError::new(
                "AF_IDENTIFIER_INVALID",
                format!("unsupported character `{ch}` in core name `{name}`"),
                "Use letters, digits, underscore, dash, or dot.",
                2,
            ));
        }
    }
    let Some(first) = out.chars().next() else {
        return Err(CliError::new(
            "AF_IDENTIFIER_INVALID",
            "core name must not be empty",
            "Provide a non-empty core name.",
            2,
        ));
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(CliError::new(
            "AF_IDENTIFIER_INVALID",
            format!("core name `{name}` must start with a letter or underscore"),
            "Use a Verilog-compatible module name.",
            2,
        ));
    }
    Ok(out)
}

fn constraint_file_name(format: &str) -> Option<&'static str> {
    match format {
        "cst" => Some("pins.cst"),
        "pcf" => Some("pins.pcf"),
        "lpf" => Some("constraints.lpf"),
        "qsf" => Some("project.qsf"),
        "sdc" => Some("timing.sdc"),
        "xdc" => Some("constraints.xdc"),
        "pdc" => Some("constraints.pdc"),
        _ => None,
    }
}

fn to_pretty_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|err| {
        json!({
            "code": "AF_JSON_SERIALIZE_FAILED",
            "message": err.to_string(),
            "hint": "Report this bug with the command that produced non-serializable output.",
            "exit_code": 1
        })
        .to_string()
    })
}
