// SPDX-License-Identifier: Apache-2.0
mod agent;
mod catalog_readiness;
mod ci;
mod commands;
mod cores_registry;
mod tooling;

use af_backend::{
    AfBackend, BackendStatus, CommandRecord, CommandRunner, CommandSpec, ProcessCommandRunner,
    ToolVersion,
};
use af_backend_icarus::IcarusBackend;
use af_backend_native::NativeBackend;
use af_backend_nextpnr::NextpnrBackend;
use af_backend_sby::SbyBackend;
use af_backend_verilator::VerilatorBackend;
use af_backend_yosys::YosysBackend;
use af_board_db::BoardDbError;
use af_complexity::{classify_path, classify_spec_file, ProjectClass};
use af_core::{
    check_core, load_manifest_from_core_dir, load_validated_manifest,
    resolve_workspace_dependencies, CoreDependencyResolution, CoreError,
};
use af_manifest::{
    standards::{
        StandardsArtifact, StandardsChecklistItem, StandardsProfile, FPGA_IP_CORE_PROFILE_ID,
    },
    CoreManifest, ManifestError, ManifestValidationReport,
};
use af_report::{
    reusable_core_maturity, write_reports, AfReport, BuildPayload, CheckPayload, CiEvidenceRecord,
    CommandPayload, DoctorPayload, FlashPayload, FormalPayload, LintPayload, MaturityInputs,
    PackagePayload, ReportError, ReportPayload, SimulationPayload, ToolingPayload, WrittenReports,
};
use af_vectors::{generate_mod_add_vectors, GenerateConfig};
use af_wrapper_gen::{generate_ipxact_skeleton, generate_wrapper, WrapperGenError, WrapperTarget};
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tooling::ToolingCommand;
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
    #[arg(long, global = true, value_enum, default_value = "always")]
    color: ColorChoice,
    #[arg(long, global = true, default_value = ".af-build")]
    build_root: PathBuf,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum ColorChoice {
    Always,
    Auto,
    Never,
}

impl ColorChoice {
    fn enabled_for_stderr(self) -> bool {
        match self {
            Self::Always => true,
            Self::Auto => std::env::var_os("NO_COLOR").is_none() && std::io::stderr().is_terminal(),
            Self::Never => false,
        }
    }
}

#[derive(Subcommand, Debug)]
enum Commands {
    Doctor,
    Tooling {
        #[command(subcommand)]
        command: ToolingCommand,
    },
    #[command(name = "self")]
    SelfCheck {
        #[command(subcommand)]
        command: SelfCommand,
    },
    Manifest {
        #[command(subcommand)]
        command: ManifestCommand,
    },
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    Core {
        #[command(subcommand)]
        command: CoreCommand,
    },
    Architecture {
        #[command(subcommand)]
        command: ArchitectureCommand,
    },
    Resource {
        #[command(subcommand)]
        command: ResourceCommand,
    },
    Compatibility {
        #[command(subcommand)]
        command: CompatibilityCommand,
    },
    Constructor {
        #[command(subcommand)]
        command: ConstructorCommand,
    },
    Signoff {
        #[command(subcommand)]
        command: SignoffCommand,
    },
    Dependency {
        #[command(subcommand)]
        command: DependencyCommand,
    },
    Registry {
        #[command(subcommand)]
        command: RegistryCommand,
    },
    Board {
        #[command(subcommand)]
        command: BoardCommand,
    },
    Build {
        core_dir: PathBuf,
        #[arg(long)]
        board: String,
        #[arg(long, default_value = "litex")]
        backend: String,
    },
    Flash {
        build_dir: PathBuf,
        #[arg(long, default_value = "openfpgaloader")]
        backend: String,
    },
    Clean {
        #[arg(long)]
        yes: bool,
    },
    Backend {
        #[command(subcommand)]
        command: BackendCommand,
    },
    Report {
        input: PathBuf,
    },
    Evidence {
        #[command(subcommand)]
        command: EvidenceCommand,
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
    Release {
        #[command(subcommand)]
        command: ReleaseCommand,
    },
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },
}

#[derive(Subcommand, Debug)]
enum ReleaseCommand {
    Check(commands::release_check::ReleaseCheckArgs),
}

#[derive(Subcommand, Debug)]
enum AgentCommand {
    /// List supported issue kinds (alias → template file mapping).
    Kinds,
    /// Print the deterministic context bundle (af version, repro, commit SHA).
    Context {
        #[arg(long)]
        from_error: Option<PathBuf>,
    },
    /// Render a Markdown issue body for the chosen kind.
    Issue {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        summary: Option<String>,
        #[arg(long)]
        from_error: Option<PathBuf>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Build the pre-filled GitHub `new issue` URL for an existing body.
    GhUrl {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        body_file: PathBuf,
        #[arg(long)]
        labels: Option<String>,
    },
    /// Emit a `gh issue create` command line for an existing body file.
    GhCli {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        body_file: PathBuf,
        #[arg(long)]
        labels: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum ManifestCommand {
    Validate {
        path: PathBuf,
    },
    Migrate {
        path: PathBuf,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        write: bool,
    },
}

#[derive(Subcommand, Debug)]
enum SelfCommand {
    Check {
        #[arg(long, default_value = "af-selfcheck.toml")]
        config: PathBuf,
        #[arg(long)]
        include_optional: bool,
        #[arg(long)]
        target: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
enum ProjectCommand {
    Classify {
        path: Option<PathBuf>,
        #[arg(long)]
        interactive: bool,
        #[arg(long)]
        from_spec: Option<PathBuf>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    New {
        project_dir: PathBuf,
        #[arg(long)]
        class: String,
        #[arg(long)]
        name: Option<String>,
    },
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
        #[arg(long)]
        class: Option<String>,
        #[arg(long, default_value = "examples")]
        library: String,
        #[arg(long, default_value = "verilog-2001")]
        language: String,
        #[arg(long, default_value = "stream-ip")]
        profile: String,
        #[arg(long)]
        standards_profile: Option<String>,
        #[arg(long)]
        portability_level: Option<String>,
        #[arg(long)]
        priority: Option<String>,
        #[arg(long)]
        maturity: Option<String>,
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
    Formal {
        core_dir: PathBuf,
        #[arg(long, default_value = "sby")]
        backend: String,
    },
    Tooling {
        core_dir: PathBuf,
        #[arg(long)]
        require_all: bool,
    },
    Package {
        core_dir: PathBuf,
        #[arg(long, default_value = "manifest")]
        format: String,
    },
    Regs {
        #[command(subcommand)]
        command: CoreRegsCommand,
    },
    Standards {
        #[command(subcommand)]
        command: CoreStandardsCommand,
    },
    Report {
        input: PathBuf,
    },
    Verify {
        core_dir: PathBuf,
        #[arg(long)]
        tier: String,
    },
    Registry {
        #[command(subcommand)]
        command: CoreRegistryCommand,
    },
}

#[derive(Subcommand, Debug)]
enum CoreStandardsCommand {
    Check {
        core_dir: PathBuf,
        #[arg(long, default_value = FPGA_IP_CORE_PROFILE_ID)]
        profile: String,
        #[arg(long)]
        strict: bool,
    },
    Doctor {
        #[arg(long, default_value = FPGA_IP_CORE_PROFILE_ID)]
        profile: String,
    },
    Drift {
        #[arg(long, default_value = FPGA_IP_CORE_PROFILE_ID)]
        profile: String,
    },
    Export {
        #[arg(long, default_value = FPGA_IP_CORE_PROFILE_ID)]
        profile: String,
        #[arg(long, default_value = "json")]
        format: String,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    Scaffold {
        core_dir: PathBuf,
        #[arg(long, default_value = FPGA_IP_CORE_PROFILE_ID)]
        profile: String,
        #[arg(long)]
        declare: bool,
        #[arg(long, default_value = "none")]
        safety_domain: String,
    },
    SpdxAudit {
        core_dir: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        declare: bool,
    },
    Collect {
        core_dir: PathBuf,
        #[arg(long)]
        build_root: PathBuf,
        #[arg(long, default_value = FPGA_IP_CORE_PROFILE_ID)]
        profile: String,
        #[arg(long)]
        declare: bool,
    },
}

#[derive(Subcommand, Debug)]
enum CoreRegsCommand {
    Scaffold {
        core_dir: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        declare: bool,
    },
    Check {
        core_dir: PathBuf,
        #[arg(long)]
        path: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum CoreRegistryCommand {
    List {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        priority: Option<String>,
        #[arg(long)]
        portability: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum ArchitectureCommand {
    Check { project_dir: PathBuf },
}

#[derive(Subcommand, Debug)]
enum ResourceCommand {
    Plan {
        core_dir: PathBuf,
        #[arg(long)]
        vendor: Option<String>,
        #[arg(long)]
        family: Option<String>,
        #[arg(long)]
        board: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum CompatibilityCommand {
    Check {
        inputs: Vec<PathBuf>,
        #[arg(long)]
        constructor: bool,
    },
}

#[derive(Subcommand, Debug)]
enum ConstructorCommand {
    Export {
        input: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        catalog: bool,
    },
    Assemble {
        cores: Vec<PathBuf>,
        #[arg(long)]
        board: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value = ".")]
        registry_root: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum SignoffCommand {
    Plan {
        input: PathBuf,
        #[arg(long)]
        class: Option<String>,
        #[arg(long)]
        board: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum DependencyCommand {
    Graph {
        core_dir: PathBuf,
        #[arg(long, default_value = "json")]
        format: String,
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
    List {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    Check {
        path: PathBuf,
    },
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
enum BackendCommand {
    List,
    Scaffold {
        core_dir: PathBuf,
        #[arg(long)]
        vendor: String,
        #[arg(long)]
        family: String,
    },
    Run {
        backend: String,
        #[arg(long, default_value = "lint")]
        target: String,
        #[arg(long)]
        core_dir: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum EvidenceCommand {
    Ingest {
        #[arg(long)]
        kind: EvidenceKind,
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        core: Option<String>,
        #[arg(long)]
        tool: Option<String>,
        #[arg(long)]
        status: Option<EvidenceStatus>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum EvidenceKind {
    SimulationLog,
    LintTranscript,
    FormalVerdict,
    SynthesisReport,
    PnrReport,
    ProgrammingLog,
    HardwareMeasurement,
    CiRun,
}

impl EvidenceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::SimulationLog => "simulation-log",
            Self::LintTranscript => "lint-transcript",
            Self::FormalVerdict => "formal-verdict",
            Self::SynthesisReport => "synthesis-report",
            Self::PnrReport => "pnr-report",
            Self::ProgrammingLog => "programming-log",
            Self::HardwareMeasurement => "hardware-measurement",
            Self::CiRun => "ci-run",
        }
    }

    fn report_stem(self) -> &'static str {
        match self {
            Self::SimulationLog => "simulation_report",
            Self::LintTranscript => "lint_report",
            Self::FormalVerdict => "formal_report",
            Self::SynthesisReport => "synthesis_report",
            Self::PnrReport => "pnr_report",
            Self::ProgrammingLog => "programming_report",
            Self::HardwareMeasurement => "hardware_measurement_report",
            Self::CiRun => "ci_run_report",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum EvidenceStatus {
    Passed,
    Warning,
    Failed,
    Unknown,
}

impl EvidenceStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Warning => "warning",
            Self::Failed => "failed",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Subcommand, Debug)]
enum VectorsCommand {
    Generate {
        #[arg(
            long,
            default_value = "examples/af-mod-add/vectors/af_mod_add_basic.json"
        )]
        basic_out: PathBuf,
        #[arg(
            long,
            default_value = "examples/af-mod-add/vectors/af_mod_add_random.json"
        )]
        random_out: PathBuf,
        #[arg(
            long,
            default_value = "examples/af-mod-add/vectors/af_mod_add_random.svh"
        )]
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
        #[arg(long)]
        board: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum CiCommand {
    Init(commands::ci_init::CiInitArgs),
    Render(commands::ci_render::CiRenderArgs),
    Doctor(commands::ci_doctor::CiDoctorArgs),
    Improve(commands::ci_improve::CiImproveArgs),
    AddBoard(commands::ci_add_board::CiAddBoardArgs),
    Validate(commands::ci_validate::CiValidateArgs),
    RunLocal(commands::ci_run_local::CiRunLocalArgs),
    // Legacy compatibility with existing behavior.
    Generate {
        #[arg(long, default_value = "github-actions")]
        target: String,
        #[arg(long, value_delimiter = ',')]
        backends: Vec<String>,
        #[arg(long)]
        optional_fail_closed: bool,
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

impl From<af_complexity::ComplexityError> for CliError {
    fn from(err: af_complexity::ComplexityError) -> Self {
        CliError::new(err.code(), err.to_string(), err.hint(), err.exit_code())
    }
}

impl From<af_architecture::ArchitectureError> for CliError {
    fn from(err: af_architecture::ArchitectureError) -> Self {
        CliError::new(err.code(), err.to_string(), err.hint(), err.exit_code())
    }
}

impl From<af_resource_model::ResourceModelError> for CliError {
    fn from(err: af_resource_model::ResourceModelError) -> Self {
        CliError::new(err.code(), err.to_string(), err.hint(), err.exit_code())
    }
}

impl From<af_compatibility::CompatibilityError> for CliError {
    fn from(err: af_compatibility::CompatibilityError) -> Self {
        CliError::new(err.code(), err.to_string(), err.hint(), err.exit_code())
    }
}

impl From<af_constructor_export::ConstructorExportError> for CliError {
    fn from(err: af_constructor_export::ConstructorExportError) -> Self {
        CliError::new(err.code(), err.to_string(), err.hint(), err.exit_code())
    }
}

impl From<af_signoff::SignoffError> for CliError {
    fn from(err: af_signoff::SignoffError) -> Self {
        CliError::new(err.code(), err.to_string(), err.hint(), err.exit_code())
    }
}

impl From<af_template::TemplateError> for CliError {
    fn from(err: af_template::TemplateError) -> Self {
        CliError::new(err.code(), err.to_string(), err.hint(), err.exit_code())
    }
}

fn main() {
    let cli = Cli::parse();
    let color = cli.color.enabled_for_stderr();
    init_tracing(cli.verbose, cli.quiet, color);
    let command = command_path(&cli.command);
    let started = Instant::now();
    tracing::info!(
        command = command.as_str(),
        json = cli.json,
        build_root = %cli.build_root.display(),
        "af command started"
    );
    match execute(&cli) {
        Ok(output) => {
            tracing::info!(
                command = command.as_str(),
                status = output_status(&output),
                duration_ms = started.elapsed().as_millis() as u64,
                "af command completed"
            );
            if cli.json {
                println!("{}", to_pretty_json(&output.json));
            } else if !cli.quiet {
                println!("{}", output.human);
            }
        }
        Err(err) => {
            tracing::error!(
                command = command.as_str(),
                code = err.payload.code.as_str(),
                exit_code = err.payload.exit_code,
                duration_ms = started.elapsed().as_millis() as u64,
                "af command failed"
            );
            if cli.json {
                println!("{}", to_pretty_json(&err.payload));
            } else if !cli.quiet {
                eprintln!("{}", format_error(&err.payload, color));
            }
            std::process::exit(err.payload.exit_code);
        }
    }
}

fn init_tracing(verbose: u8, quiet: bool, color: bool) {
    if quiet {
        return;
    }
    let level = match verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(color)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init();
}

fn command_path(command: &Commands) -> String {
    match command {
        Commands::Doctor => "doctor".to_string(),
        Commands::Tooling { command } => format!("tooling {}", tooling_command_name(command)),
        Commands::SelfCheck { command } => format!("self {}", self_command_name(command)),
        Commands::Manifest { command } => format!("manifest {}", manifest_command_name(command)),
        Commands::Project { command } => format!("project {}", project_command_name(command)),
        Commands::Core { command } => format!("core {}", core_command_name(command)),
        Commands::Architecture { command } => {
            format!("architecture {}", architecture_command_name(command))
        }
        Commands::Resource { command } => format!("resource {}", resource_command_name(command)),
        Commands::Compatibility { command } => {
            format!("compatibility {}", compatibility_command_name(command))
        }
        Commands::Constructor { command } => {
            format!("constructor {}", constructor_command_name(command))
        }
        Commands::Signoff { command } => format!("signoff {}", signoff_command_name(command)),
        Commands::Dependency { command } => {
            format!("dependency {}", dependency_command_name(command))
        }
        Commands::Registry { command } => format!("registry {}", registry_command_name(command)),
        Commands::Board { command } => format!("board {}", board_command_name(command)),
        Commands::Build { .. } => "build".to_string(),
        Commands::Flash { .. } => "flash".to_string(),
        Commands::Clean { .. } => "clean".to_string(),
        Commands::Backend { command } => format!("backend {}", backend_command_name(command)),
        Commands::Report { .. } => "report".to_string(),
        Commands::Evidence { command } => format!("evidence {}", evidence_command_name(command)),
        Commands::Vectors { command } => format!("vectors {}", vectors_command_name(command)),
        Commands::Wrapper { command } => format!("wrapper {}", wrapper_command_name(command)),
        Commands::Ci { command } => format!("ci {}", ci_command_name(command)),
        Commands::Release { command } => format!("release {}", release_command_name(command)),
        Commands::Agent { command } => format!("agent {}", agent_command_name(command)),
    }
}

fn release_command_name(command: &ReleaseCommand) -> &'static str {
    match command {
        ReleaseCommand::Check(_) => "check",
    }
}

fn agent_command_name(command: &AgentCommand) -> &'static str {
    match command {
        AgentCommand::Kinds => "kinds",
        AgentCommand::Context { .. } => "context",
        AgentCommand::Issue { .. } => "issue",
        AgentCommand::GhUrl { .. } => "gh-url",
        AgentCommand::GhCli { .. } => "gh-cli",
    }
}

fn tooling_command_name(command: &ToolingCommand) -> &'static str {
    match command {
        ToolingCommand::Check(_) => "check",
        ToolingCommand::Plan(_) => "plan",
        ToolingCommand::Ensure(_) => "ensure",
    }
}

fn self_command_name(command: &SelfCommand) -> &'static str {
    match command {
        SelfCommand::Check { .. } => "check",
    }
}

fn manifest_command_name(command: &ManifestCommand) -> &'static str {
    match command {
        ManifestCommand::Validate { .. } => "validate",
        ManifestCommand::Migrate { .. } => "migrate",
    }
}

fn project_command_name(command: &ProjectCommand) -> &'static str {
    match command {
        ProjectCommand::Classify { .. } => "classify",
        ProjectCommand::New { .. } => "new",
    }
}

fn core_command_name(command: &CoreCommand) -> &'static str {
    match command {
        CoreCommand::Check { .. } => "check",
        CoreCommand::New { .. } => "new",
        CoreCommand::Lint { .. } => "lint",
        CoreCommand::Sim { .. } => "sim",
        CoreCommand::Formal { .. } => "formal",
        CoreCommand::Registry { command } => core_registry_command_name(command),
        CoreCommand::Tooling { .. } => "tooling",
        CoreCommand::Package { .. } => "package",
        CoreCommand::Regs { command } => core_regs_command_name(command),
        CoreCommand::Standards { command } => core_standards_command_name(command),
        CoreCommand::Report { .. } => "report",
        CoreCommand::Verify { .. } => "verify",
    }
}

fn core_standards_command_name(command: &CoreStandardsCommand) -> &'static str {
    match command {
        CoreStandardsCommand::Check { .. } => "standards check",
        CoreStandardsCommand::Doctor { .. } => "standards doctor",
        CoreStandardsCommand::Drift { .. } => "standards drift",
        CoreStandardsCommand::Export { .. } => "standards export",
        CoreStandardsCommand::Scaffold { .. } => "standards scaffold",
        CoreStandardsCommand::SpdxAudit { .. } => "standards spdx-audit",
        CoreStandardsCommand::Collect { .. } => "standards collect",
    }
}

fn core_regs_command_name(command: &CoreRegsCommand) -> &'static str {
    match command {
        CoreRegsCommand::Scaffold { .. } => "regs scaffold",
        CoreRegsCommand::Check { .. } => "regs check",
    }
}

fn core_registry_command_name(command: &CoreRegistryCommand) -> &'static str {
    match command {
        CoreRegistryCommand::List { .. } => "registry list",
    }
}

fn architecture_command_name(command: &ArchitectureCommand) -> &'static str {
    match command {
        ArchitectureCommand::Check { .. } => "check",
    }
}

fn resource_command_name(command: &ResourceCommand) -> &'static str {
    match command {
        ResourceCommand::Plan { .. } => "plan",
    }
}

fn compatibility_command_name(command: &CompatibilityCommand) -> &'static str {
    match command {
        CompatibilityCommand::Check { .. } => "check",
    }
}

fn constructor_command_name(command: &ConstructorCommand) -> &'static str {
    match command {
        ConstructorCommand::Export { .. } => "export",
        ConstructorCommand::Assemble { .. } => "assemble",
    }
}

fn signoff_command_name(command: &SignoffCommand) -> &'static str {
    match command {
        SignoffCommand::Plan { .. } => "plan",
    }
}

fn dependency_command_name(command: &DependencyCommand) -> &'static str {
    match command {
        DependencyCommand::Graph { .. } => "graph",
    }
}

fn registry_command_name(command: &RegistryCommand) -> &'static str {
    match command {
        RegistryCommand::Check { .. } => "check",
    }
}

fn board_command_name(command: &BoardCommand) -> &'static str {
    match command {
        BoardCommand::List { .. } => "list",
        BoardCommand::Check { .. } => "check",
        BoardCommand::Matrix { .. } => "matrix",
        BoardCommand::New { .. } => "new",
    }
}

fn backend_command_name(command: &BackendCommand) -> &'static str {
    match command {
        BackendCommand::List => "list",
        BackendCommand::Scaffold { .. } => "scaffold",
        BackendCommand::Run { .. } => "run",
    }
}

fn evidence_command_name(command: &EvidenceCommand) -> &'static str {
    match command {
        EvidenceCommand::Ingest { .. } => "ingest",
    }
}

fn vectors_command_name(command: &VectorsCommand) -> &'static str {
    match command {
        VectorsCommand::Generate { .. } => "generate",
    }
}

fn wrapper_command_name(command: &WrapperCommand) -> &'static str {
    match command {
        WrapperCommand::Generate { .. } => "generate",
    }
}

fn ci_command_name(command: &CiCommand) -> &'static str {
    match command {
        CiCommand::Init(_) => "init",
        CiCommand::Render(_) => "render",
        CiCommand::Doctor(_) => "doctor",
        CiCommand::Improve(_) => "improve",
        CiCommand::AddBoard(_) => "add-board",
        CiCommand::Validate(_) => "validate",
        CiCommand::RunLocal(_) => "run-local",
        CiCommand::Generate { .. } => "generate",
    }
}

fn format_error(payload: &ErrorPayload, color: bool) -> String {
    if color {
        format!(
            "{}: {}\n{}: {}",
            ansi("1;31", &payload.code),
            payload.message,
            ansi("1;33", "hint"),
            payload.hint
        )
    } else {
        format!(
            "{}: {}\nhint: {}",
            payload.code, payload.message, payload.hint
        )
    }
}

fn ansi(style: &str, text: &str) -> String {
    format!("\u{1b}[{style}m{text}\u{1b}[0m")
}

fn output_status(output: &CliOutput) -> &str {
    output
        .json
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
}

fn execute(cli: &Cli) -> Result<CliOutput, CliError> {
    match &cli.command {
        Commands::Doctor => doctor(&cli.build_root),
        Commands::Tooling { command } => tooling::execute(command),
        Commands::SelfCheck { command } => match command {
            SelfCommand::Check {
                config,
                include_optional,
                target,
            } => {
                commands::self_check::self_check(config, *include_optional, target, &cli.build_root)
            }
        },
        Commands::Manifest { command } => match command {
            ManifestCommand::Validate { path } => manifest_validate(path),
            ManifestCommand::Migrate {
                path,
                from,
                to,
                write,
            } => manifest_migrate(path, from, to, *write),
        },
        Commands::Project { command } => match command {
            ProjectCommand::Classify {
                path,
                interactive,
                from_spec,
                output,
            } => project_classify(
                path.as_ref(),
                *interactive,
                from_spec.as_ref(),
                output.as_ref(),
            ),
            ProjectCommand::New {
                project_dir,
                class,
                name,
            } => project_new(project_dir, class, name.as_deref()),
        },
        Commands::Core { command } => match command {
            CoreCommand::Check { core_dir } => core_check(core_dir, &cli.build_root),
            CoreCommand::New {
                core_dir,
                name,
                class,
                library,
                language,
                profile,
                standards_profile,
                portability_level,
                priority,
                maturity,
            } => {
                if let Some(profile) = standards_profile.as_deref() {
                    load_standards_profile(profile)?;
                }
                let axes = commands::core_new::AxesOverride::from_cli(
                    portability_level.as_deref(),
                    priority.as_deref(),
                    maturity.as_deref(),
                )?;
                let mut output = commands::core_new::core_new(
                    core_dir,
                    name,
                    class.as_deref(),
                    library,
                    language,
                    profile,
                    axes,
                )?;
                if let Some(profile) = standards_profile.as_deref() {
                    let scaffold = core_standards_scaffold(core_dir, profile, true, "none")?;
                    output.human = format!("{}\n{}", output.human, scaffold.human);
                    if let Some(object) = output.json.as_object_mut() {
                        object.insert("standards_scaffold".to_string(), scaffold.json);
                    }
                }
                Ok(output)
            }
            CoreCommand::Lint { core_dir, backend } => {
                core_lint(core_dir, &cli.build_root, backend)
            }
            CoreCommand::Sim { core_dir, backend } => core_sim(core_dir, &cli.build_root, backend),
            CoreCommand::Formal { core_dir, backend } => {
                core_formal(core_dir, &cli.build_root, backend)
            }
            CoreCommand::Tooling {
                core_dir,
                require_all,
            } => core_tooling(core_dir, &cli.build_root, *require_all),
            CoreCommand::Package { core_dir, format } => {
                core_package(core_dir, &cli.build_root, format)
            }
            CoreCommand::Regs { command } => match command {
                CoreRegsCommand::Scaffold {
                    core_dir,
                    output,
                    declare,
                } => core_regs_scaffold(core_dir, output.as_deref(), *declare),
                CoreRegsCommand::Check { core_dir, path } => {
                    core_regs_check(core_dir, path.as_deref())
                }
            },
            CoreCommand::Standards { command } => match command {
                CoreStandardsCommand::Check {
                    core_dir,
                    profile,
                    strict,
                } => core_standards_check(core_dir, profile, *strict),
                CoreStandardsCommand::Doctor { profile } => core_standards_doctor(profile),
                CoreStandardsCommand::Drift { profile } => core_standards_drift(profile),
                CoreStandardsCommand::Export {
                    profile,
                    format,
                    output,
                } => core_standards_export(profile, format, output.as_deref()),
                CoreStandardsCommand::Scaffold {
                    core_dir,
                    profile,
                    declare,
                    safety_domain,
                } => core_standards_scaffold(core_dir, profile, *declare, safety_domain),
                CoreStandardsCommand::SpdxAudit {
                    core_dir,
                    output,
                    declare,
                } => core_standards_spdx_audit(core_dir, output.as_deref(), *declare),
                CoreStandardsCommand::Collect {
                    core_dir,
                    build_root,
                    profile,
                    declare,
                } => core_standards_collect(core_dir, build_root, profile, *declare),
            },
            CoreCommand::Report { input } => core_report(input, &cli.build_root),
            CoreCommand::Verify { core_dir, tier } => core_verify(core_dir, tier, &cli.build_root),
            CoreCommand::Registry { command } => match command {
                CoreRegistryCommand::List {
                    root,
                    priority,
                    portability,
                } => core_registry_list(root, priority.as_deref(), portability.as_deref()),
            },
        },
        Commands::Architecture { command } => match command {
            ArchitectureCommand::Check { project_dir } => architecture_check(project_dir),
        },
        Commands::Resource { command } => match command {
            ResourceCommand::Plan {
                core_dir,
                vendor,
                family,
                board,
            } => resource_plan(
                core_dir,
                vendor.as_deref(),
                family.as_deref(),
                board.as_deref(),
            ),
        },
        Commands::Compatibility { command } => match command {
            CompatibilityCommand::Check {
                inputs,
                constructor,
            } => compatibility_check(inputs, *constructor),
        },
        Commands::Constructor { command } => match command {
            ConstructorCommand::Export {
                input,
                output,
                catalog,
            } => constructor_export(input, output.as_ref(), *catalog, &cli.build_root),
            ConstructorCommand::Assemble {
                cores,
                board,
                name,
                output,
                registry_root,
            } => constructor_assemble(cores, board, name, output, registry_root),
        },
        Commands::Signoff { command } => match command {
            SignoffCommand::Plan {
                input,
                class,
                board,
            } => signoff_plan(input, class.as_deref(), board.as_deref()),
        },
        Commands::Dependency { command } => match command {
            DependencyCommand::Graph { core_dir, format } => dependency_graph(core_dir, format),
        },
        Commands::Registry { command } => match command {
            RegistryCommand::Check { root } => registry_check(root),
        },
        Commands::Board { command } => match command {
            BoardCommand::List { root } => board_list(root),
            BoardCommand::Check { path } => board_check(path),
            BoardCommand::Matrix { root, output } => board_matrix(root, output.as_ref()),
            BoardCommand::New {
                board_id,
                vendor,
                family,
                constraint_format,
                root,
            } => board_new(root, board_id, vendor, family, constraint_format),
        },
        Commands::Build {
            core_dir,
            board,
            backend,
        } => build(core_dir, &cli.build_root, board, backend),
        Commands::Flash { build_dir, backend } => flash(build_dir, backend),
        Commands::Clean { yes } => clean(&cli.build_root, *yes),
        Commands::Backend { command } => match command {
            BackendCommand::List => commands::backend::backend_list(),
            BackendCommand::Scaffold {
                core_dir,
                vendor,
                family,
            } => commands::backend::backend_scaffold(core_dir, vendor, family),
            BackendCommand::Run {
                backend,
                target,
                core_dir,
            } => backend_run(backend, target, core_dir.as_ref(), &cli.build_root),
        },
        Commands::Report { input } => core_report(input, &cli.build_root),
        Commands::Evidence { command } => match command {
            EvidenceCommand::Ingest {
                kind,
                input,
                core,
                tool,
                status,
                output,
            } => commands::evidence::evidence_ingest(
                *kind,
                input,
                core.as_deref(),
                tool.as_deref(),
                *status,
                output.as_ref(),
                &cli.build_root,
            ),
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
            WrapperCommand::Generate {
                core_dir,
                target,
                board,
            } => commands::wrapper::wrapper_generate(
                core_dir,
                &cli.build_root,
                target,
                board.as_deref(),
            ),
        },
        Commands::Ci { command } => match command {
            CiCommand::Init(args) => commands::ci_init::run(args),
            CiCommand::Render(args) => commands::ci_render::run(args),
            CiCommand::Doctor(args) => commands::ci_doctor::run(args),
            CiCommand::Improve(args) => commands::ci_improve::run(args),
            CiCommand::AddBoard(args) => commands::ci_add_board::run(args),
            CiCommand::Validate(args) => commands::ci_validate::run(args),
            CiCommand::RunLocal(args) => commands::ci_run_local::run(args),
            CiCommand::Generate {
                target,
                backends,
                optional_fail_closed,
                output,
            } => ci_generate(
                target,
                output.as_ref(),
                backends.as_slice(),
                *optional_fail_closed,
            ),
        },
        Commands::Release { command } => match command {
            ReleaseCommand::Check(args) => commands::release_check::run(args, &cli.build_root),
        },
        Commands::Agent { command } => agent_dispatch(command),
    }
}

fn agent_dispatch(command: &AgentCommand) -> Result<CliOutput, CliError> {
    match command {
        AgentCommand::Kinds => agent_kinds(),
        AgentCommand::Context { from_error } => agent_context_cmd(from_error.as_deref()),
        AgentCommand::Issue {
            kind,
            title,
            summary,
            from_error,
            output,
        } => agent_issue_cmd(
            kind,
            title,
            summary.as_deref(),
            from_error.as_deref(),
            output.as_deref(),
        ),
        AgentCommand::GhUrl {
            kind,
            title,
            body_file,
            labels,
        } => agent_gh_url_cmd(kind, title, body_file, labels.as_deref()),
        AgentCommand::GhCli {
            kind,
            title,
            body_file,
            labels,
        } => agent_gh_cli_cmd(kind, title, body_file, labels.as_deref()),
    }
}

fn agent_parse_kind(raw: &str) -> Result<agent::IssueKind, CliError> {
    raw.parse::<agent::IssueKind>().map_err(|err| {
        CliError::new(
            "AF_AGENT_KIND_UNSUPPORTED",
            err,
            "Use `af agent kinds` to list supported issue kinds.",
            2,
        )
    })
}

fn agent_kinds() -> Result<CliOutput, CliError> {
    let entries: Vec<Value> = agent::IssueKind::ALL
        .iter()
        .map(|k| {
            json!({
                "kind": k.as_str(),
                "template_file": k.template_file(),
                "default_labels": k.default_labels(),
                "title_prefix": k.title_prefix(),
            })
        })
        .collect();
    let human = agent::IssueKind::ALL
        .iter()
        .map(|k| {
            format!(
                "{:14}  → .github/ISSUE_TEMPLATE/{}",
                k.as_str(),
                k.template_file()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(CliOutput {
        human,
        json: json!({
            "status": "passed",
            "kinds": entries,
        }),
    })
}

fn agent_context_cmd(from_error: Option<&Path>) -> Result<CliOutput, CliError> {
    let repo_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let context = agent::AgentContext::gather(&repo_root);
    let mut payload = serde_json::to_value(&context).map_err(|err| {
        CliError::new(
            "AF_AGENT_CONTEXT_SERIALIZE_FAILED",
            err.to_string(),
            "Report this as a bug; AgentContext should always serialise.",
            5,
        )
    })?;
    if let Some(path) = from_error {
        let raw = fs::read_to_string(path).map_err(|err| {
            CliError::new(
                "AF_AGENT_ERROR_FILE_UNREADABLE",
                format!("failed to read `{}`: {err}", path.display()),
                "Provide a readable JSON file produced by an earlier `af ... --json` failure.",
                2,
            )
        })?;
        let parsed: Value = serde_json::from_str(&raw).map_err(|err| {
            CliError::new(
                "AF_AGENT_ERROR_FILE_INVALID",
                format!("failed to parse `{}` as JSON: {err}", path.display()),
                "Pass a `--json` payload from `af` (CliError or AfReport block).",
                2,
            )
        })?;
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("from_error".to_string(), parsed);
        }
    }
    Ok(CliOutput {
        human: format!(
            "af_version={} commit_sha={} repo={}/{}",
            context.af_version,
            context.current_commit_sha.as_deref().unwrap_or("unknown"),
            context.repo_owner,
            context.repo_name,
        ),
        json: payload,
    })
}

fn agent_issue_cmd(
    kind: &str,
    title: &str,
    summary: Option<&str>,
    from_error: Option<&Path>,
    output: Option<&Path>,
) -> Result<CliOutput, CliError> {
    let kind = agent_parse_kind(kind)?;
    let repo_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let context = agent::AgentContext::gather(&repo_root);
    let error_json = if let Some(path) = from_error {
        Some(fs::read_to_string(path).map_err(|err| {
            CliError::new(
                "AF_AGENT_ERROR_FILE_UNREADABLE",
                format!("failed to read `{}`: {err}", path.display()),
                "Provide a readable JSON file produced by an earlier `af ... --json` failure.",
                2,
            )
        })?)
    } else {
        None
    };
    let body = agent::render_issue_markdown(kind, title, summary, &context, error_json.as_deref());
    let body_path = if let Some(out) = output {
        fs::write(out, &body).map_err(|err| {
            CliError::new(
                "AF_AGENT_OUTPUT_WRITE_FAILED",
                format!("failed to write `{}`: {err}", out.display()),
                "Pick a writable --output path.",
                5,
            )
        })?;
        Some(out.to_path_buf())
    } else {
        None
    };
    let labels: Vec<&str> = kind.default_labels().to_vec();
    let (gh_url, url_warnings) = agent::render_gh_url(
        &context.repo_owner,
        &context.repo_name,
        kind,
        title,
        &body,
        &labels,
    );
    let body_file_for_cli = body_path
        .clone()
        .unwrap_or_else(|| PathBuf::from("body.md"));
    let gh_cli = agent::render_gh_cli(
        &context.repo_owner,
        &context.repo_name,
        title,
        &body_file_for_cli,
        &labels,
    );
    let preview: String = body.lines().take(8).collect::<Vec<_>>().join("\n");
    let human = body_path
        .as_ref()
        .map(|p| format!("issue body written: {}", p.display()))
        .unwrap_or_else(|| body.clone());
    Ok(CliOutput {
        human,
        json: json!({
            "status": "passed",
            "kind": kind.as_str(),
            "title": title,
            "body_preview": preview,
            "body_path": body_path,
            "labels": labels,
            "gh_url": gh_url,
            "gh_cli": gh_cli,
            "warnings": url_warnings,
            "context": context,
        }),
    })
}

fn agent_gh_url_cmd(
    kind: &str,
    title: &str,
    body_file: &Path,
    labels: Option<&str>,
) -> Result<CliOutput, CliError> {
    let kind = agent_parse_kind(kind)?;
    let body = fs::read_to_string(body_file).map_err(|err| {
        CliError::new(
            "AF_AGENT_BODY_FILE_UNREADABLE",
            format!("failed to read `{}`: {err}", body_file.display()),
            "Pass a readable --body-file path (e.g. produced by `af agent issue --output`).",
            2,
        )
    })?;
    let repo_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let (owner, repo) = agent::discover_github_repo(&repo_root)
        .unwrap_or_else(|| ("AccelFury".into(), "af".into()));
    let labels_vec: Vec<&str> = match labels {
        Some(s) => s
            .split(',')
            .map(|x| x.trim())
            .filter(|x| !x.is_empty())
            .collect(),
        None => kind.default_labels().to_vec(),
    };
    let (url, warnings) = agent::render_gh_url(&owner, &repo, kind, title, &body, &labels_vec);
    Ok(CliOutput {
        human: url.clone(),
        json: json!({
            "status": "passed",
            "kind": kind.as_str(),
            "title": title,
            "repo": format!("{owner}/{repo}"),
            "labels": labels_vec,
            "gh_url": url,
            "warnings": warnings,
        }),
    })
}

fn agent_gh_cli_cmd(
    kind: &str,
    title: &str,
    body_file: &Path,
    labels: Option<&str>,
) -> Result<CliOutput, CliError> {
    let kind = agent_parse_kind(kind)?;
    if !body_file.is_file() {
        return Err(CliError::new(
            "AF_AGENT_BODY_FILE_UNREADABLE",
            format!("body file `{}` does not exist", body_file.display()),
            "Pass a readable --body-file path (e.g. produced by `af agent issue --output`).",
            2,
        ));
    }
    let repo_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let (owner, repo) = agent::discover_github_repo(&repo_root)
        .unwrap_or_else(|| ("AccelFury".into(), "af".into()));
    let labels_vec: Vec<&str> = match labels {
        Some(s) => s
            .split(',')
            .map(|x| x.trim())
            .filter(|x| !x.is_empty())
            .collect(),
        None => kind.default_labels().to_vec(),
    };
    let cli = agent::render_gh_cli(&owner, &repo, title, body_file, &labels_vec);
    Ok(CliOutput {
        human: cli.clone(),
        json: json!({
            "status": "passed",
            "kind": kind.as_str(),
            "title": title,
            "repo": format!("{owner}/{repo}"),
            "labels": labels_vec,
            "gh_cli": cli,
        }),
    })
}

fn doctor(build_root: &Path) -> Result<CliOutput, CliError> {
    let runner = ProcessCommandRunner;
    let verilator = VerilatorBackend::process()
        .doctor()
        .expect("doctor is infallible");
    let yosys = YosysBackend::process()
        .doctor()
        .expect("doctor is infallible");
    let icarus = IcarusBackend::process()
        .doctor()
        .expect("doctor is infallible");
    let sby = SbyBackend::process()
        .doctor()
        .expect("doctor is infallible");
    let nextpnr = NextpnrBackend::process()
        .doctor()
        .expect("doctor is infallible");
    let native = NativeBackend.doctor().expect("doctor is infallible");

    let tool_probes = [
        ("deno", vec!["--version"]),
        ("fusesoc", vec!["--version"]),
        ("xmllint", vec!["--version"]),
        ("python3", vec!["--version"]),
        ("boolector", vec!["--version"]),
        ("z3", vec!["--version"]),
        ("yices-smt2", vec!["--version"]),
        ("bitwuzla", vec!["--version"]),
        ("cvc5", vec!["--version"]),
        ("openFPGALoader", vec!["--help"]),
        ("gw_sh", vec!["--version"]),
        ("programmer_cli", vec!["--version"]),
    ];

    let mut report = AfReport::new("passed");
    report.merge_backend(&native);
    report.merge_backend(&verilator);
    report.merge_backend(&yosys);
    report.merge_backend(&icarus);
    report.merge_backend(&sby);
    report.merge_backend(&nextpnr);
    for (program, args) in tool_probes {
        let (tool_version, commands) = probe_tool(&runner, program, &args);
        report.tool_versions.push(tool_version);
        report.commands.extend(commands);
    }
    let (litex_version, litex_commands) = probe_python_module(&runner, "litex");
    report.tool_versions.push(litex_version);
    report.commands.extend(litex_commands);
    let (deno_audit_version, deno_audit_commands) = probe_deno_audit_readiness(&runner);
    report.tool_versions.push(deno_audit_version);
    report.commands.extend(deno_audit_commands);
    report.limitations.push(
        "MVP doctor checks tool visibility only; it does not validate vendor bitstream flows or EULA status."
            .to_string(),
    );
    report.limitations.push(
        "Deno audit readiness checks Deno visibility and the audit:repo task declaration; it does not run the write-capable audit task."
            .to_string(),
    );

    if report.tool_versions.iter().any(|tool| !tool.available) {
        report.status = "warning".to_string();
        report
            .warnings
            .push("One or more optional backend tools are unavailable.".to_string());
    }
    let deno_available = report
        .tool_versions
        .iter()
        .find(|tool| tool.tool == "deno")
        .is_some_and(|tool| tool.available);
    let deno_audit_available = report
        .tool_versions
        .iter()
        .find(|tool| tool.tool == "deno-audit-repo")
        .is_some_and(|tool| tool.available);
    if !deno_available {
        report
            .warnings
            .push("Deno is unavailable; `deno task audit:repo` readiness is blocked.".to_string());
    } else if !deno_audit_available {
        report.warnings.push(
            "`deno task audit:repo` is not ready from the current repository context.".to_string(),
        );
    }

    let total_tools = report.tool_versions.len();
    let available_tools = report
        .tool_versions
        .iter()
        .filter(|tv| tv.available)
        .count();
    let missing_tools: Vec<String> = report
        .tool_versions
        .iter()
        .filter(|tv| !tv.available)
        .map(|tv| tv.tool.clone())
        .collect();
    report.command_payload = Some(CommandPayload::Doctor(DoctorPayload {
        overall_status: report.status.clone(),
        total_tools,
        available_tools,
        missing_tools,
    }));
    // Persist stdout/stderr of every tool probe run by `doctor` so the
    // report's commands array carries log file references, not just inline
    // text.
    persist_backend_logs(&mut report, build_root, "doctor");

    Ok(CliOutput {
        human: format!("doctor {}", report.status),
        json: json!(report),
    })
}

fn manifest_validate(path: &Path) -> Result<CliOutput, CliError> {
    let manifest = CoreManifest::from_path(path)?;
    let core_dir = manifest_dependency_core_dir(path);
    let (dependency_resolutions, dependency_issues) =
        resolve_workspace_dependencies(core_dir, &manifest);
    if !dependency_issues.is_empty() {
        return Err(CliError::new(
            "AF_MANIFEST_DEPENDENCY_INVALID",
            format!(
                "manifest dependency resolution failed with {} issue(s)",
                dependency_issues.len()
            ),
            "Fix [[dependencies.cores]].path so dependencies resolve to sibling workspace cores with af-core.toml.",
            2,
        )
        .with_details(&json!({
            "dependency_issues": dependency_issues,
            "dependency_resolutions": dependency_resolutions,
        })));
    }
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
            "dependency_resolutions": dependency_resolutions,
        }),
    })
}

fn manifest_dependency_core_dir(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn manifest_migrate(path: &Path, from: &str, to: &str, write: bool) -> Result<CliOutput, CliError> {
    if from != "0.1" || !matches!(to, "0.1" | "0.2") {
        return Err(CliError::new(
            "AF_MANIFEST_MIGRATION_UNSUPPORTED",
            format!("manifest migration {from} -> {to} is unsupported"),
            "Use --from 0.1 --to 0.2 for the built-in compatibility migration.",
            2,
        ));
    }
    let mut manifest = CoreManifest::from_path(path)?;
    manifest.af_version = to.to_string();
    if manifest.kind.is_none() {
        manifest.kind = Some("accelfury.core".to_string());
    }
    let migrated_report = manifest.validate();
    if !migrated_report.valid {
        return Err(ManifestError::Validation {
            issues: migrated_report.issues,
        }
        .into());
    }
    let payload = toml::to_string_pretty(&manifest).map_err(|err| {
        CliError::new(
            "AF_MANIFEST_MIGRATION_SERIALIZE_FAILED",
            err.to_string(),
            "Report this bug with the manifest that could not be serialized.",
            1,
        )
    })?;
    let output = if write {
        path.to_path_buf()
    } else {
        path.with_file_name(format!(
            "{}.migrated-{to}.toml",
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("af-core")
        ))
    };
    if !write && output.exists() {
        return Err(CliError::new(
            "AF_FILE_EXISTS",
            format!("refusing to overwrite existing migration output `{}`", output.display()),
            "Pass --write to overwrite the source manifest, or remove the generated migration file intentionally.",
            2,
        ));
    }
    fs::write(&output, payload).map_err(|err| {
        CliError::new(
            "AF_MANIFEST_MIGRATION_WRITE_FAILED",
            format!("failed to write `{}`: {err}", output.display()),
            "Check filesystem permissions for the manifest directory.",
            5,
        )
    })?;
    Ok(CliOutput {
        human: format!("manifest migrated: {}", output.display()),
        json: json!({
            "status": "passed",
            "source": path,
            "output": output,
            "from": from,
            "to": to,
            "write": write,
        }),
    })
}

fn project_classify(
    path: Option<&PathBuf>,
    interactive: bool,
    from_spec: Option<&PathBuf>,
    output: Option<&PathBuf>,
) -> Result<CliOutput, CliError> {
    let mut report = if let Some(spec) = from_spec {
        classify_spec_file(spec)?
    } else {
        classify_path(path.map(PathBuf::as_path).unwrap_or_else(|| Path::new(".")))?
    };
    if interactive {
        report.warnings.push(
            "Interactive questionnaire is deterministic in this first release; filesystem/spec evidence was used instead of prompting.".to_string(),
        );
    }
    if let Some(output) = output {
        if let Some(parent) = output.parent() {
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
        write_json_file(output, &report)?;
    }
    Ok(CliOutput {
        human: format!(
            "project class: {} (score {})",
            report.project_class, report.score
        ),
        json: json!(report),
    })
}

fn project_new(project_dir: &Path, class: &str, name: Option<&str>) -> Result<CliOutput, CliError> {
    let project_class = parse_project_class(class)?;
    ensure_project_dir_is_safe(project_dir)?;
    let report = af_template::scaffold_project(project_dir, project_class, name)?;
    Ok(CliOutput {
        human: format!(
            "project scaffold written: {} ({})",
            project_dir.display(),
            report.project_class
        ),
        json: json!(report),
    })
}

/// Refuse to scaffold a project on top of a directory that already declares an
/// AccelFury core/project/product manifest. This prevents `af project new /tmp`
/// (or any populated path) from silently colonising a user's existing tree.
fn ensure_project_dir_is_safe(project_dir: &Path) -> Result<(), CliError> {
    if !project_dir.exists() {
        return Ok(());
    }
    if !project_dir.is_dir() {
        return Err(CliError::new(
            "AF_PROJECT_NEW_TARGET_NOT_DIR",
            format!(
                "project new target `{}` exists but is not a directory",
                project_dir.display()
            ),
            "Choose a new project directory or remove the existing path intentionally.",
            2,
        ));
    }
    for manifest in ["af-core.toml", "af-project.toml", "af-product.toml"] {
        let candidate = project_dir.join(manifest);
        if candidate.exists() {
            return Err(CliError::new(
                "AF_PROJECT_NEW_DIR_NOT_EMPTY",
                format!(
                    "project new target `{}` already contains `{manifest}`",
                    project_dir.display()
                ),
                "Choose a fresh project directory; `af project new` refuses to overwrite an existing AccelFury manifest.",
                2,
            ));
        }
    }
    Ok(())
}

pub(crate) fn parse_project_class(value: &str) -> Result<ProjectClass, CliError> {
    value.parse::<ProjectClass>().map_err(|_| {
        CliError::new(
            "AF_COMPLEXITY_CLASS_REQUIRED",
            format!("unsupported project class `{value}`"),
            "Use one of: simple-portable, composite-portable, complex-vendor-aware, system-platform, product-stack.",
            2,
        )
    })
}

fn architecture_check(project_dir: &Path) -> Result<CliOutput, CliError> {
    let report = af_architecture::check_architecture(project_dir)?;
    if report.status == "failed" {
        return Err(CliError::new(
            "AF_ARCH_LAYER_VIOLATION",
            "architecture check failed",
            "Fix the listed architecture issues before backend implementation or constructor export.",
            2,
        )
        .with_details(&report));
    }
    Ok(CliOutput {
        human: format!("architecture check {}", report.status),
        json: json!(report),
    })
}

fn resource_plan(
    core_dir: &Path,
    vendor: Option<&str>,
    family: Option<&str>,
    board: Option<&str>,
) -> Result<CliOutput, CliError> {
    let report = af_resource_model::plan_resources(core_dir, vendor, family, board)?;
    Ok(CliOutput {
        human: format!("resource plan {}: {}", report.status, core_dir.display()),
        json: json!(report),
    })
}

fn compatibility_check(inputs: &[PathBuf], constructor: bool) -> Result<CliOutput, CliError> {
    for input in inputs {
        if input.join("af-core.toml").is_file() {
            load_validated_manifest(input)?;
        }
    }
    let report = af_compatibility::check_compatibility(inputs, constructor)?;
    if report.status == "failed" {
        return Err(CliError::new(
            "AF_COMPAT_PROTOCOL_MISMATCH",
            "compatibility check failed",
            "Fix the listed protocol, width, clock/reset, resource, vendor, or security conflicts; suggested adapters are included when possible.",
            2,
        )
        .with_details(&report));
    }
    Ok(CliOutput {
        human: format!("compatibility check {}", report.status),
        json: json!(report),
    })
}

fn constructor_export(
    input: &Path,
    output: Option<&PathBuf>,
    catalog: bool,
    build_root: &Path,
) -> Result<CliOutput, CliError> {
    if input.is_dir() && input.join("af-core.toml").is_file() {
        load_validated_manifest(input)?;
    }
    let output = output
        .cloned()
        .unwrap_or_else(|| build_root.join("constructor"));
    let report = af_constructor_export::export_constructor_package(input, &output, catalog)?;
    Ok(CliOutput {
        human: format!("constructor export {}: {}", report.status, output.display()),
        json: json!(report),
    })
}

fn constructor_assemble(
    cores: &[PathBuf],
    board: &str,
    name: &str,
    output: &Path,
    registry_root: &Path,
) -> Result<CliOutput, CliError> {
    let report = af_constructor_export::assemble_project(cores, board, name, output, registry_root)
        .map_err(|err| {
            let code = err.code();
            let hint = err.hint();
            let exit_code = err.exit_code();
            CliError::new(code, err.to_string(), hint, exit_code)
        })?;
    Ok(CliOutput {
        human: format!(
            "constructor assemble {}: {} ({} cores, board `{}`)",
            report.status,
            output.display(),
            report.cores.len(),
            report.board
        ),
        json: json!(report),
    })
}

fn signoff_plan(
    input: &Path,
    class: Option<&str>,
    board: Option<&str>,
) -> Result<CliOutput, CliError> {
    let project_class = class.map(parse_project_class).transpose()?;
    if input.is_dir() && input.join("af-core.toml").is_file() {
        load_validated_manifest(input)?;
    }
    let report = af_signoff::create_signoff_plan(input, project_class, board.map(str::to_string))?;
    Ok(CliOutput {
        human: format!(
            "signoff plan: {} ({})",
            input.display(),
            report.project_class
        ),
        json: json!(report),
    })
}

fn dependency_graph(core_dir: &Path, format: &str) -> Result<CliOutput, CliError> {
    let manifest = load_validated_manifest(core_dir)?;
    let nodes = std::iter::once(manifest.core.clone())
        .chain(
            manifest
                .dependencies
                .cores
                .iter()
                .map(|dependency| dependency.name.clone()),
        )
        .collect::<Vec<_>>();
    let edges = manifest
        .dependencies
        .cores
        .iter()
        .map(|dependency| {
            json!({
                "from": &manifest.core,
                "to": dependency.name,
                "role": dependency.role,
                "version": dependency.version,
            })
        })
        .collect::<Vec<_>>();
    match format {
        "json" => Ok(CliOutput {
            human: format!("dependency graph: {} nodes", nodes.len()),
            json: json!({
                "generated_by": "AccelFury IP Toolchain",
                "status": "passed",
                "format": "json",
                "core_dir": core_dir,
                "nodes": nodes,
                "edges": edges,
            }),
        }),
        "dot" => {
            let mut dot = String::from("digraph af_dependencies {\n");
            dot.push_str(&format!("  \"{}\";\n", manifest.core));
            for dependency in &manifest.dependencies.cores {
                dot.push_str(&format!(
                    "  \"{}\" -> \"{}\" [label=\"{} {}\"];\n",
                    manifest.core, dependency.name, dependency.role, dependency.version
                ));
            }
            dot.push_str("}\n");
            Ok(CliOutput {
                human: dot.clone(),
                json: json!({
                    "generated_by": "AccelFury IP Toolchain",
                    "status": "passed",
                    "format": "dot",
                    "dot": dot,
                }),
            })
        }
        _ => Err(CliError::new(
            "AF_DEPENDENCY_GRAPH_FORMAT_UNSUPPORTED",
            format!("dependency graph format `{format}` is unsupported"),
            "Use --format json or --format dot.",
            2,
        )),
    }
}

fn registry_check(root: &Path) -> Result<CliOutput, CliError> {
    let board_report = af_board_db::check_registry(root)?;
    let cores_report = cores_registry::check(root);
    let catalog_readiness = catalog_readiness::check(root);

    let mut human_lines = Vec::new();
    human_lines.push(format!(
        "board registry: {} boards, {} aliases",
        board_report.board_count, board_report.alias_count
    ));
    human_lines.push(format!(
        "cores registry: {} cores{}",
        cores_report.core_count,
        if cores_report.warnings.is_empty() {
            String::new()
        } else {
            format!(", {} warnings", cores_report.warnings.len())
        }
    ));

    let all_valid = board_report.valid && cores_report.valid;
    let payload = json!({
        "status": if all_valid { "passed" } else { "failed" },
        "registry": board_report,
        "cores_registry": cores_report,
        "catalog_readiness": catalog_readiness,
    });

    if all_valid {
        Ok(CliOutput {
            human: human_lines.join("\n"),
            json: payload,
        })
    } else {
        Err(CliError::new(
            "AF_REGISTRY_INVALID",
            "registry check failed",
            "Fix the listed board or cores registry issues.",
            2,
        )
        .with_details(&payload))
    }
}

fn core_registry_list(
    root: &Path,
    priority: Option<&str>,
    portability: Option<&str>,
) -> Result<CliOutput, CliError> {
    let registry = cores_registry::load(root).map_err(|err| {
        CliError::new(err.code(), err.message(), err.hint(), 5).with_details(&json!({
            "path": cores_registry::registry_path(root),
        }))
    })?;

    if let Some(priority) = priority {
        if !matches!(priority, "P0" | "P1" | "P2") {
            return Err(CliError::new(
                "AF_CORES_REGISTRY_FILTER_INVALID",
                format!("unsupported --priority `{priority}`"),
                "Use --priority P0, P1, or P2.",
                2,
            ));
        }
    }
    if let Some(portability) = portability {
        if !matches!(portability, "U0" | "U1" | "U2" | "U3" | "U4") {
            return Err(CliError::new(
                "AF_CORES_REGISTRY_FILTER_INVALID",
                format!("unsupported --portability `{portability}`"),
                "Use --portability U0..U4.",
                2,
            ));
        }
    }

    let filtered: Vec<&cores_registry::RegisteredCore> = registry
        .cores
        .iter()
        .filter(|core| priority.is_none_or(|wanted| core.priority == wanted))
        .filter(|core| portability.is_none_or(|wanted| core.portability_level == wanted))
        .collect();

    let human = if filtered.is_empty() {
        "no cores match the given filters".to_string()
    } else {
        filtered
            .iter()
            .map(|core| {
                format!(
                    "{:18}  {}/{}/{}  [{}]  {}",
                    core.core_id,
                    core.priority,
                    core.portability_level,
                    core.maturity,
                    core.category,
                    core.summary
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    Ok(CliOutput {
        human,
        json: json!({
            "status": "passed",
            "schema_version": registry.schema_version,
            "filter": {
                "priority": priority,
                "portability": portability,
            },
            "core_count": filtered.len(),
            "cores": filtered,
        }),
    })
}

fn board_list(root: &Path) -> Result<CliOutput, CliError> {
    let boards = af_board_db::list_boards(root)?;
    Ok(CliOutput {
        human: boards
            .iter()
            .map(|board| {
                let marker = if board_is_verified(&board.exact_pinout_status) {
                    "[VERIFIED]"
                } else {
                    "[DRAFT]"
                };
                format!("{marker} {} ({})", board.board_id, board.display_name)
            })
            .collect::<Vec<_>>()
            .join("\n"),
        json: json!({
            "status": "passed",
            "boards": boards,
        }),
    })
}

pub(crate) fn board_is_verified(exact_pinout_status: &str) -> bool {
    exact_pinout_status
        .trim()
        .eq_ignore_ascii_case("verified_on_hardware")
}

fn board_check(path: &Path) -> Result<CliOutput, CliError> {
    let profile = af_board_db::check_board_profile(path)?;
    Ok(CliOutput {
        human: format!("board profile valid: {}", profile.id),
        json: json!({
            "status": "passed",
            "board": profile,
        }),
    })
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
        revision: None,
        revision_source_locator: None,
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
    af_report
        .artifacts
        .extend(dependency_artifacts(&report.dependency_resolutions));
    af_report.warnings.extend(report.warnings.clone());
    af_report.command_payload = Some(CommandPayload::Check(CheckPayload {
        manifest_status: report.status.clone(),
        source_count: report.manifest.sources.files.len(),
        inspection_issue_count: report.inspection.issues.len(),
        legal_issue_count: report.legal_issues.len(),
    }));
    let written = write_reports_with_aliases(
        build_root.join("reports"),
        "core-check",
        &["core_check_report"],
        &mut af_report,
    )?;

    Ok(CliOutput {
        human: format!(
            "core check passed: {} (reports: {}, {})",
            report.manifest.vlnv(),
            written.json.display(),
            written.markdown.display()
        ),
        json: json!({
            "status": "passed",
            "command_payload": af_report.command_payload,
            "check": report,
            "reports": written,
        }),
    })
}

#[derive(Debug, Serialize)]
struct CoreToolingReport {
    generated_by: &'static str,
    schema_version: &'static str,
    kind: &'static str,
    status: String,
    core_dir: PathBuf,
    manifest_path: PathBuf,
    core: String,
    groups: Vec<CoreToolingGroupReport>,
    artifacts: Vec<PathBuf>,
    warnings: Vec<String>,
    limitations: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CoreToolingGroupReport {
    id: &'static str,
    title: &'static str,
    status: String,
    tools: Vec<CoreToolingToolStatus>,
    commands: Vec<CommandRecord>,
    artifacts: Vec<PathBuf>,
    warnings: Vec<String>,
    limitations: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CoreToolingToolStatus {
    tool: String,
    available: bool,
    version: Option<String>,
    message: Option<String>,
    purpose: String,
}

struct CoreToolProbe {
    tool: &'static str,
    probe: CoreToolProbeKind,
    purpose: &'static str,
}

#[derive(Clone, Copy)]
enum CoreToolProbeKind {
    Command(&'static [&'static str]),
    PythonModule,
}

struct CoreToolGroupSpec {
    id: &'static str,
    title: &'static str,
    project_report: &'static str,
    project_versions: &'static str,
    build_report: &'static str,
    missing_warning: &'static str,
    limitations: &'static [&'static str],
    probes: &'static [CoreToolProbe],
}

const SMT_SOLVER_PROBES: &[CoreToolProbe] = &[
    CoreToolProbe {
        tool: "boolector",
        probe: CoreToolProbeKind::Command(&["--version"]),
        purpose: "Bit-vector and array SMT solver for yosys-smtbmc/SymbiYosys cross-checks.",
    },
    CoreToolProbe {
        tool: "z3",
        probe: CoreToolProbeKind::Command(&["--version"]),
        purpose: "General SMT-LIB solver for formal cross-checks.",
    },
    CoreToolProbe {
        tool: "yices-smt2",
        probe: CoreToolProbeKind::Command(&["--version"]),
        purpose: "Yices SMT-LIB solver used by yosys-smtbmc formal flows.",
    },
    CoreToolProbe {
        tool: "bitwuzla",
        probe: CoreToolProbeKind::Command(&["--version"]),
        purpose: "Bit-vector and array SMT solver for formal cross-checks.",
    },
    CoreToolProbe {
        tool: "cvc5",
        probe: CoreToolProbeKind::Command(&["--version"]),
        purpose: "SMT-LIB solver for formal compatibility and cross-checks.",
    },
];

const CORE_INTEGRATION_TOOL_PROBES: &[CoreToolProbe] = &[
    CoreToolProbe {
        tool: "xmllint",
        probe: CoreToolProbeKind::Command(&["--version"]),
        purpose: "XML package/schema validation helper for IP-XACT and constructor metadata.",
    },
    CoreToolProbe {
        tool: "fusesoc",
        probe: CoreToolProbeKind::Command(&["--version"]),
        purpose: "FuseSoC package and dependency integration checks.",
    },
    CoreToolProbe {
        tool: "edalize",
        probe: CoreToolProbeKind::PythonModule,
        purpose: "Edalize Python backend API used by FuseSoC/export integration flows.",
    },
];

const SMT_LIMITATIONS: &[&str] = &[
    "SMT solver visibility does not prove formal coverage or property completeness.",
    "Passing this check does not imply timing closure, CDC/RDC signoff, vendor implementation, or hardware readiness.",
];

const CORE_INTEGRATION_LIMITATIONS: &[&str] = &[
    "Package tool visibility does not prove that generated FuseSoC, Edalize, IP-XACT, or constructor bundles are semantically complete.",
    "Passing this check does not imply vendor implementation, board integration, or hardware readiness.",
];

const CORE_TOOL_GROUPS: &[CoreToolGroupSpec] = &[
    CoreToolGroupSpec {
        id: "smt_solvers",
        title: "SMT solver visibility",
        project_report: "artifacts/openfpga-ci/reports/core-smt-solvers.json",
        project_versions: "artifacts/openfpga-ci/logs/smt-solvers-tool-versions.txt",
        build_report: "reports/core-smt-solvers.json",
        missing_warning: "Missing SMT solvers: {missing}. Formal flows using these solvers remain blocked until they are installed or Docker runtime is used.",
        limitations: SMT_LIMITATIONS,
        probes: SMT_SOLVER_PROBES,
    },
    CoreToolGroupSpec {
        id: "package_integration",
        title: "Core package integration tool visibility",
        project_report: "artifacts/openfpga-ci/reports/core-integration-tools.json",
        project_versions: "artifacts/openfpga-ci/logs/integration-tool-versions.txt",
        build_report: "reports/core-integration-tools.json",
        missing_warning: "Missing package integration tools: {missing}. Wrapper/package validation using xmllint, FuseSoC, or Edalize remains blocked until they are installed or Docker runtime is used.",
        limitations: CORE_INTEGRATION_LIMITATIONS,
        probes: CORE_INTEGRATION_TOOL_PROBES,
    },
];

fn core_tooling(
    core_dir: &Path,
    build_root: &Path,
    require_all: bool,
) -> Result<CliOutput, CliError> {
    let checked = check_core(core_dir)?;
    let runner = ProcessCommandRunner;
    let mut warnings = Vec::new();
    let mut artifacts = Vec::new();
    let mut groups = Vec::new();
    for spec in CORE_TOOL_GROUPS {
        let group = collect_core_tool_group(spec, core_dir, build_root, &runner, require_all)?;
        warnings.extend(group.warnings.clone());
        artifacts.extend(group.artifacts.clone());
        groups.push(group);
    }

    let project_report = core_dir.join("artifacts/openfpga-ci/reports/core-tooling.json");
    let build_report = build_root.join("reports/core-tooling.json");
    artifacts.push(project_report.clone());
    artifacts.push(build_report.clone());

    let any_missing = groups.iter().any(|group| group.status != "passed");
    let status = if !any_missing {
        "passed"
    } else if groups.iter().any(|group| group.status == "failed") {
        "failed"
    } else {
        "warning"
    };
    let report = CoreToolingReport {
        generated_by: af_report::GENERATED_BY,
        schema_version: "0.2",
        kind: "accelfury.core_development_tooling_check",
        status: status.to_string(),
        core_dir: core_dir.to_path_buf(),
        manifest_path: checked.manifest_path,
        core: checked.manifest.vlnv(),
        groups,
        artifacts,
        warnings,
        limitations: vec![
            "Tool visibility artifacts prove only that selected commands or Python modules are visible to the current environment.".to_string(),
            "Passing this check does not imply formal coverage, package semantic completeness, timing closure, CDC/RDC signoff, vendor implementation, or hardware readiness.".to_string(),
        ],
    };

    write_json_file_creating_parent(&project_report, &report)?;
    write_json_file_creating_parent(&build_report, &report)?;

    let total_tools: usize = report.groups.iter().map(|g| g.tools.len()).sum();
    let available_tools: usize = report
        .groups
        .iter()
        .flat_map(|g| g.tools.iter())
        .filter(|tv| tv.available)
        .count();
    let missing_tools: Vec<String> = report
        .groups
        .iter()
        .flat_map(|g| g.tools.iter())
        .filter(|tv| !tv.available)
        .map(|tv| tv.tool.clone())
        .collect();
    let command_payload = CommandPayload::Tooling(ToolingPayload {
        total_tools,
        available_tools,
        missing_tools,
    });

    if report.status == "failed" {
        return Err(CliError::new(
            "AF_CORE_TOOLING_MISSING",
            "one or more required core development tools are unavailable",
            "Install missing tools with scripts/install-smt-solvers.sh and scripts/install-core-integration-tools.sh, or use the Docker runtime, then rerun `af core tooling --require-all`.",
            4,
        )
        .with_details(&json!({
            "command_payload": command_payload,
            "report": report,
        })));
    }

    Ok(CliOutput {
        human: format!(
            "core development tooling {}: {} (project report: {})",
            report.status,
            core_dir.display(),
            project_report.display()
        ),
        json: json!({
            "command_payload": command_payload,
            "report": report,
        }),
    })
}

fn collect_core_tool_group(
    spec: &CoreToolGroupSpec,
    core_dir: &Path,
    build_root: &Path,
    runner: &impl CommandRunner,
    require_all: bool,
) -> Result<CoreToolingGroupReport, CliError> {
    let mut tools = Vec::new();
    let mut commands = Vec::new();
    for probe in spec.probes {
        let (version, probe_commands) = match probe.probe {
            CoreToolProbeKind::Command(args) => probe_tool(runner, probe.tool, args),
            CoreToolProbeKind::PythonModule => probe_python_module(runner, probe.tool),
        };
        commands.extend(probe_commands);
        tools.push(CoreToolingToolStatus {
            tool: version.tool,
            available: version.available,
            version: version.version,
            message: version.message,
            purpose: probe.purpose.to_string(),
        });
    }

    let missing = tools
        .iter()
        .filter(|tool| !tool.available)
        .map(|tool| tool.tool.as_str())
        .collect::<Vec<_>>();
    let mut warnings = Vec::new();
    if !missing.is_empty() {
        warnings.push(
            spec.missing_warning
                .replace("{missing}", &missing.join(", ")),
        );
    }

    let project_report = core_dir.join(spec.project_report);
    let project_versions = core_dir.join(spec.project_versions);
    let build_report = build_root.join(spec.build_report);
    let artifacts = vec![
        project_report.clone(),
        project_versions.clone(),
        build_report.clone(),
    ];
    let status = if missing.is_empty() {
        "passed"
    } else if require_all {
        "failed"
    } else {
        "warning"
    };

    let report = CoreToolingGroupReport {
        id: spec.id,
        title: spec.title,
        status: status.to_string(),
        tools,
        commands,
        artifacts,
        warnings,
        limitations: spec
            .limitations
            .iter()
            .map(|limitation| (*limitation).to_string())
            .collect(),
    };

    write_json_file_creating_parent(&project_report, &report)?;
    write_json_file_creating_parent(&build_report, &report)?;
    write_text_file_creating_parent(&project_versions, &core_tool_versions_text(&report))?;

    Ok(report)
}

fn core_tool_versions_text(report: &CoreToolingGroupReport) -> String {
    let mut out = String::from("# Generated by AccelFury IP Toolchain\n");
    out.push_str(&format!("# {}\n", report.title));
    for tool in &report.tools {
        if tool.available {
            out.push_str(&format!(
                "{}: {}\n",
                tool.tool,
                tool.version.as_deref().unwrap_or("available")
            ));
        } else {
            out.push_str(&format!(
                "{}: missing ({})\n",
                tool.tool,
                tool.message.as_deref().unwrap_or("no details")
            ));
        }
    }
    out
}

fn core_lint(core_dir: &Path, build_root: &Path, backend: &str) -> Result<CliOutput, CliError> {
    if matches!(backend, "native" | "af-native") {
        return core_lint_native(core_dir, build_root);
    }

    let checked = check_core(core_dir)?;
    let backend_report = match backend {
        "verilator" => VerilatorBackend::process().lint(&checked.manifest, core_dir, build_root),
        "yosys" => YosysBackend::process().lint(&checked.manifest, core_dir, build_root),
        "icarus" | "iverilog" => {
            IcarusBackend::process().lint(&checked.manifest, core_dir, build_root)
        }
        other => Err(af_backend::BackendError::Unsupported {
            backend: other.to_string(),
        }),
    }
    .map_err(|err| CliError::new(err.code(), err.to_string(), err.hint(), err.exit_code()))?;

    let mut af_report = AfReport::for_core(status_text(&backend_report.status), &checked.manifest);
    af_report.merge_backend(&backend_report);
    af_report.command_payload = Some(CommandPayload::Lint(LintPayload {
        backend: backend.to_string(),
        backend_status: status_text(&backend_report.status).to_string(),
        source_count: checked.manifest.sources.files.len(),
        include_dir_count: checked.manifest.sources.include_dirs.len(),
    }));
    persist_backend_logs(&mut af_report, build_root, &format!("core-lint-{backend}"));
    let written = write_reports_with_aliases(
        build_root.join("reports"),
        "core-lint",
        &["lint_report"],
        &mut af_report,
    )?;

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
                ("AF_LINT_FAILED", "core lint backend command failed", 7)
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

fn core_lint_native(core_dir: &Path, build_root: &Path) -> Result<CliOutput, CliError> {
    let manifest = load_manifest_from_core_dir(core_dir)?;
    let backend_report = NativeBackend
        .lint(&manifest, core_dir, build_root)
        .map_err(|err| CliError::new(err.code(), err.to_string(), err.hint(), err.exit_code()))?;

    let mut af_report = AfReport::for_core(status_text(&backend_report.status), &manifest);
    af_report.merge_backend(&backend_report);
    af_report.command_payload = Some(CommandPayload::Lint(LintPayload {
        backend: "native".to_string(),
        backend_status: status_text(&backend_report.status).to_string(),
        source_count: manifest.sources.files.len(),
        include_dir_count: manifest.sources.include_dirs.len(),
    }));
    persist_backend_logs(&mut af_report, build_root, "core-lint-native");
    let written = write_reports_with_aliases(
        build_root.join("reports"),
        "core-lint",
        &["lint_report"],
        &mut af_report,
    )?;

    match backend_report.status {
        BackendStatus::Passed => Ok(CliOutput {
            human: format!(
                "core lint passed with native (reports: {}, {})",
                written.json.display(),
                written.markdown.display()
            ),
            json: json!({
                "status": "passed",
                "command_payload": af_report.command_payload,
                "backend_report": backend_report,
                "reports": written,
            }),
        }),
        BackendStatus::Unavailable => Err(CliError::new(
            "AF_BACKEND_UNAVAILABLE",
            "core lint backend `native` is unavailable",
            "Inspect backend diagnostics in the report.",
            4,
        )
        .with_details(&json!({
            "command_payload": af_report.command_payload,
            "backend_report": backend_report,
            "reports": written,
        }))),
        BackendStatus::Failed => Err(CliError::new(
            "AF_LINT_FAILED",
            "native portable-core lint failed",
            "Fix the listed portable Verilog-2001 diagnostics.",
            7,
        )
        .with_details(&json!({
            "command_payload": af_report.command_payload,
            "backend_report": backend_report,
            "reports": written,
        }))),
    }
}

fn core_sim(core_dir: &Path, build_root: &Path, backend: &str) -> Result<CliOutput, CliError> {
    let checked = check_core(core_dir)?;
    let backend_report = match backend {
        "verilator" => VerilatorBackend::process().sim(&checked.manifest, core_dir, build_root),
        "icarus" | "iverilog" => {
            IcarusBackend::process().sim(&checked.manifest, core_dir, build_root)
        }
        other => Err(af_backend::BackendError::Unsupported {
            backend: other.to_string(),
        }),
    }
    .map_err(|err| CliError::new(err.code(), err.to_string(), err.hint(), err.exit_code()))?;

    let mut af_report = AfReport::for_core(status_text(&backend_report.status), &checked.manifest);
    af_report.merge_backend(&backend_report);
    af_report.command_payload = Some(CommandPayload::Simulation(SimulationPayload {
        backend: backend.to_string(),
        backend_status: status_text(&backend_report.status).to_string(),
        testbench_count: checked.manifest.testbenches.len(),
    }));
    persist_backend_logs(&mut af_report, build_root, &format!("core-sim-{backend}"));
    let written = write_reports_with_aliases(
        build_root.join("reports"),
        "core-sim",
        &["simulation_report"],
        &mut af_report,
    )?;

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
                ("AF_SIMULATION_FAILED", "core sim backend command failed", 6)
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

fn core_formal(core_dir: &Path, build_root: &Path, backend: &str) -> Result<CliOutput, CliError> {
    let checked = check_core(core_dir)?;
    if backend != "sby" {
        return Err(CliError::new(
            "AF_FORMAL_BACKEND_UNSUPPORTED",
            format!("core formal backend `{backend}` is unsupported"),
            "Use --backend sby for the first-release formal backend.",
            2,
        ));
    }
    let backend_report = SbyBackend::process()
        .run_formal(&checked.manifest, core_dir, build_root)
        .map_err(|err| CliError::new(err.code(), err.to_string(), err.hint(), err.exit_code()))?;
    let mut report = AfReport::for_core(status_text(&backend_report.status), &checked.manifest);
    report.merge_backend(&backend_report);
    report.command_payload = Some(CommandPayload::Formal(FormalPayload {
        backend: backend.to_string(),
        backend_status: status_text(&backend_report.status).to_string(),
        property_count: checked
            .manifest
            .formal
            .as_ref()
            .map(|formal| formal.properties.len())
            .unwrap_or(0),
    }));
    persist_backend_logs(&mut report, build_root, &format!("core-formal-{backend}"));
    let written = write_reports_with_aliases(
        build_root.join("reports"),
        "core-formal",
        &["formal_report"],
        &mut report,
    )?;
    match backend_report.status {
        BackendStatus::Passed => Ok(CliOutput {
            human: format!(
                "core formal passed with sby (reports: {}, {})",
                written.json.display(),
                written.markdown.display()
            ),
            json: json!({
                "status": "passed",
                "backend_report": backend_report,
                "reports": written,
            }),
        }),
        BackendStatus::Unavailable => Err(CliError::new(
            "AF_BACKEND_UNAVAILABLE",
            "core formal backend `sby` is unavailable",
            "Install sby or keep [formal].enabled=false.",
            4,
        )
        .with_details(&json!({
            "backend_report": backend_report,
            "reports": written,
        }))),
        BackendStatus::Failed => Err(CliError::new(
            "AF_FORMAL_FAILED",
            "SymbiYosys formal backend command failed",
            "Inspect formal_report command details and fix the failing proof target.",
            8,
        )
        .with_details(&json!({
            "backend_report": backend_report,
            "reports": written,
        }))),
    }
}

#[derive(Clone, Debug, Serialize)]
struct StandardsCheckSummary {
    total_items: usize,
    supported_items: usize,
    blocked_items: usize,
    planned_items: usize,
    foundation_items: usize,
}

#[derive(Clone, Debug, Serialize)]
struct StandardsCheckRow {
    checklist_item_id: u8,
    item: String,
    category: String,
    status: String,
    validation_status: String,
    standards: Vec<af_manifest::standards::StandardMapping>,
    evidence: Vec<String>,
    limitations: Vec<String>,
    required_artifact_kinds: Vec<String>,
    artifact_validations: Vec<StandardsArtifactValidation>,
}

#[derive(Clone, Debug, Serialize)]
struct StandardsArtifactValidation {
    kind: String,
    path: String,
    validation_status: String,
    evidence: Vec<String>,
    limitations: Vec<String>,
}

fn load_standards_profile(profile: &str) -> Result<StandardsProfile, CliError> {
    StandardsProfile::by_id(profile).ok_or_else(|| {
        CliError::new(
            "AF_STANDARDS_PROFILE_UNKNOWN",
            format!("unknown standards profile `{profile}`"),
            "Use --profile fpga-ip-core-v1.",
            2,
        )
    })
}

const STANDARDS_CURRENT_REVIEW_DATE: &str = "2026-05-25";

fn core_standards_doctor(profile_id: &str) -> Result<CliOutput, CliError> {
    let profile = load_standards_profile(profile_id)?;
    let tools = standards_tool_availability(&profile);
    let total_tools = tools.len();
    let available_tools = tools
        .iter()
        .filter(|tool| tool["available"].as_bool().unwrap_or(false))
        .count();
    let missing_tools = total_tools.saturating_sub(available_tools);
    Ok(CliOutput {
        human: format!(
            "standards doctor: {available_tools}/{total_tools} tools available for {}",
            profile.id
        ),
        json: json!({
            "status": "passed",
            "profile": profile.id,
            "profile_version": profile.version,
            "snapshot_date": profile.snapshot_date,
            "summary": {
                "total_tools": total_tools,
                "available_tools": available_tools,
                "missing_tools": missing_tools,
                "strict_ready": missing_tools == 0,
            },
            "tools": tools,
            "limitations": [
                "Tool availability is local PATH probing only; it does not install tools or prove standards conformance.",
                "Missing optional tools are reported for planning and become blocking only for strict validation or release gates that require them."
            ],
        }),
    })
}

fn standards_tool_availability(profile: &StandardsProfile) -> Vec<Value> {
    standards_tool_requirements()
        .into_iter()
        .map(|requirement| {
            let probe = probe_program_version(requirement.program);
            json!({
                "program": requirement.program,
                "available": probe.available,
                "version": probe.version,
                "required_for": requirement.required_for,
                "artifact_kinds": requirement.artifact_kinds,
                "mode": requirement.mode,
                "install_hint": standards_tool_install_hint(requirement.program),
                "container_hint": standards_tool_container_hint(requirement.program),
                "manual_url_hint": standards_tool_manual_url_hint(requirement.program),
                "profile": profile.id,
                "limitations": if probe.available {
                    Vec::<String>::new()
                } else {
                    vec![format!("`{}` was not found in PATH", requirement.program)]
                },
            })
        })
        .collect()
}

fn standards_tool_install_hint(program: &str) -> &'static str {
    match program {
        "xmllint" => "Install libxml2 tooling, for example: apt install libxml2-utils.",
        "peakrdl" => "Install the PeakRDL command line tools, for example: pipx install peakrdl.",
        "verible-verilog-lint" => {
            "Install Verible from OSS CAD Suite, CHIPS Alliance releases, or your package manager."
        }
        "verilator" => "Install Verilator from OSS CAD Suite or your package manager.",
        "sby" => "Install SymbiYosys from OSS CAD Suite or your package manager.",
        "reuse" => "Install REUSE tooling, for example: pipx install reuse.",
        "spdx-sbom-generator" => {
            "Install spdx-sbom-generator from upstream releases or use a container image that includes it."
        }
        _ => "Install the tool through the project container/profile or your package manager.",
    }
}

fn standards_tool_container_hint(program: &str) -> &'static str {
    match program {
        "xmllint" | "verible-verilog-lint" | "verilator" | "sby" => {
            "Use an OSS CAD Suite based container/profile when local installation is undesirable."
        }
        "peakrdl" | "reuse" | "spdx-sbom-generator" => {
            "Use a Python/package-tooling container/profile when local installation is undesirable."
        }
        _ => "Use an af-enabled container/profile when local installation is undesirable.",
    }
}

fn standards_tool_manual_url_hint(program: &str) -> &'static str {
    match program {
        "xmllint" => "https://gitlab.gnome.org/GNOME/libxml2",
        "peakrdl" => "https://peakrdl.readthedocs.io/",
        "verible-verilog-lint" => "https://github.com/chipsalliance/verible",
        "verilator" => "https://verilator.org/",
        "sby" => "https://yosyshq.readthedocs.io/projects/sby/",
        "reuse" => "https://reuse.software/",
        "spdx-sbom-generator" => "https://github.com/opensbom-generator/spdx-sbom-generator",
        _ => "https://accelfury.dev/",
    }
}

#[derive(Clone, Copy)]
struct StandardsToolRequirement {
    program: &'static str,
    required_for: &'static [&'static str],
    artifact_kinds: &'static [&'static str],
    mode: &'static str,
}

fn standards_tool_requirements() -> Vec<StandardsToolRequirement> {
    vec![
        StandardsToolRequirement {
            program: "xmllint",
            required_for: &[
                "IEEE 1685-2022 IP-XACT validation",
                "strict standards check",
            ],
            artifact_kinds: &["ip-xact"],
            mode: "strict-validator",
        },
        StandardsToolRequirement {
            program: "peakrdl",
            required_for: &["SystemRDL semantic validation", "register flow"],
            artifact_kinds: &["systemrdl"],
            mode: "strict-validator",
        },
        StandardsToolRequirement {
            program: "verible-verilog-lint",
            required_for: &["RTL style/lint evidence", "strict standards check"],
            artifact_kinds: &["verible-lint", "native-lint"],
            mode: "strict-validator",
        },
        StandardsToolRequirement {
            program: "verilator",
            required_for: &["open-source lint/simulation CI evidence"],
            artifact_kinds: &["native-lint", "ci"],
            mode: "evidence-producer",
        },
        StandardsToolRequirement {
            program: "sby",
            required_for: &["formal verification evidence"],
            artifact_kinds: &["formal-plan"],
            mode: "evidence-producer",
        },
        StandardsToolRequirement {
            program: "reuse",
            required_for: &["SPDX header audit"],
            artifact_kinds: &["spdx-header-audit"],
            mode: "optional-auditor",
        },
        StandardsToolRequirement {
            program: "spdx-sbom-generator",
            required_for: &["SPDX/HBOM cross-check"],
            artifact_kinds: &["spdx-hbom"],
            mode: "optional-auditor",
        },
    ]
}

#[derive(Clone, Debug)]
struct ToolProbe {
    available: bool,
    version: Option<String>,
}

fn probe_program_version(program: &str) -> ToolProbe {
    match std::process::Command::new(program)
        .arg("--version")
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let version = stdout
                .lines()
                .chain(stderr.lines())
                .map(str::trim)
                .find(|line| !line.is_empty())
                .map(|line| line.to_string());
            ToolProbe {
                available: true,
                version,
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => ToolProbe {
            available: false,
            version: None,
        },
        Err(err) => ToolProbe {
            available: false,
            version: Some(err.to_string()),
        },
    }
}

fn core_standards_drift(profile_id: &str) -> Result<CliOutput, CliError> {
    let profile = load_standards_profile(profile_id)?;
    let age_days = ymd_days_between(&profile.snapshot_date, STANDARDS_CURRENT_REVIEW_DATE);
    let findings = vec![
        standards_drift_finding(
            "SPDX License List",
            "monthly",
            age_days,
            45,
            "Re-pin before commercial releases because SPDX license identifiers drift frequently.",
        ),
        standards_drift_finding(
            "MITRE CWE-1194",
            "quarterly",
            age_days,
            120,
            "Re-pin CWE snapshots quarterly and whenever MIHW changes.",
        ),
        standards_drift_finding(
            "IEEE standards pins",
            "release/manual",
            age_days,
            365,
            "Re-check IEEE/ISO/Accellera editions before major releases.",
        ),
    ];
    let status = if findings
        .iter()
        .any(|finding| finding["severity"] == "warning")
    {
        "warning"
    } else {
        "passed"
    };
    Ok(CliOutput {
        human: format!(
            "standards drift {status}: snapshot {} reviewed against {}",
            profile.snapshot_date, STANDARDS_CURRENT_REVIEW_DATE
        ),
        json: json!({
            "status": status,
            "profile": profile.id,
            "profile_version": profile.version,
            "snapshot_date": profile.snapshot_date,
            "review_date": STANDARDS_CURRENT_REVIEW_DATE,
            "snapshot_age_days": age_days,
            "findings": findings,
            "limitations": [
                "This is an offline freshness check: it compares pinned snapshot dates to review cadences and does not query standards bodies or registries.",
                "Run a manual standards refresh before each major release or when a buyer requires a specific standard edition."
            ],
        }),
    })
}

fn standards_drift_finding(
    standard: &str,
    cadence: &str,
    age_days: Option<i64>,
    max_age_days: i64,
    recommendation: &str,
) -> Value {
    let severity = match age_days {
        Some(age) if age > max_age_days => "warning",
        Some(_) => "ok",
        None => "warning",
    };
    json!({
        "standard": standard,
        "cadence": cadence,
        "severity": severity,
        "max_age_days": max_age_days,
        "age_days": age_days,
        "recommendation": recommendation,
    })
}

fn ymd_days_between(start: &str, end: &str) -> Option<i64> {
    let (sy, sm, sd) = parse_ymd(start)?;
    let (ey, em, ed) = parse_ymd(end)?;
    Some(days_from_civil(ey, em, ed) - days_from_civil(sy, sm, sd))
}

fn parse_ymd(input: &str) -> Option<(i32, u32, u32)> {
    let mut parts = input.split('-');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((year, month, day))
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year - (month <= 2) as i32;
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = month as i32;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146097 + doe - 719468) as i64
}

fn core_standards_export(
    profile_id: &str,
    format: &str,
    output: Option<&Path>,
) -> Result<CliOutput, CliError> {
    let profile = load_standards_profile(profile_id)?;
    let mut rendered = match format {
        "json" => to_pretty_json(&profile.compliance_json()),
        "checklist" | "markdown" | "md" => profile.checklist_markdown(),
        "csv" | "matrix" => profile.compliance_csv(),
        other => {
            return Err(CliError::new(
                "AF_STANDARDS_EXPORT_FORMAT_UNSUPPORTED",
                format!("standards export format `{other}` is unsupported"),
                "Use --format json, checklist, or csv.",
                2,
            ));
        }
    };
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    if let Some(output) = output {
        write_text_file_creating_parent(output, &rendered)?;
    }
    Ok(CliOutput {
        human: output
            .map(|path| format!("standards profile exported: {}", path.display()))
            .unwrap_or_else(|| rendered.clone()),
        json: json!({
            "status": "passed",
            "profile": profile,
            "format": format,
            "output": output,
            "content": if output.is_none() { Some(rendered) } else { None::<String> },
        }),
    })
}

fn validate_safety_domain(domain: &str) -> Result<(), CliError> {
    if matches!(domain, "none" | "automotive" | "industrial" | "avionics") {
        return Ok(());
    }
    Err(CliError::new(
        "AF_STANDARDS_SAFETY_DOMAIN_UNSUPPORTED",
        format!("unsupported standards safety domain `{domain}`"),
        "Use --safety-domain none, automotive, industrial, or avionics.",
        2,
    ))
}

fn core_standards_scaffold(
    core_dir: &Path,
    profile_id: &str,
    declare: bool,
    safety_domain: &str,
) -> Result<CliOutput, CliError> {
    let profile = load_standards_profile(profile_id)?;
    validate_safety_domain(safety_domain)?;
    let checked = check_core(core_dir)?;
    let manifest = checked.manifest;
    let mut written = Vec::new();
    let mut existing = Vec::new();

    write_scaffold_text_if_missing(
        &core_dir.join("docs/spec.md"),
        &standards_spec_scaffold(&profile, &manifest),
        &mut written,
        &mut existing,
    )?;
    write_scaffold_text_if_missing(
        &core_dir.join("docs/datasheet.md"),
        &standards_datasheet_scaffold(&manifest),
        &mut written,
        &mut existing,
    )?;
    write_scaffold_text_if_missing(
        &core_dir.join("docs/acceptance.md"),
        &standards_acceptance_scaffold(&manifest),
        &mut written,
        &mut existing,
    )?;
    write_scaffold_text_if_missing(
        &core_dir.join("docs/risks.md"),
        &standards_risks_scaffold(&manifest),
        &mut written,
        &mut existing,
    )?;
    write_scaffold_text_if_missing(
        &core_dir.join("sim/README.md"),
        &standards_sim_scaffold(&manifest),
        &mut written,
        &mut existing,
    )?;
    write_scaffold_text_if_missing(
        &core_dir.join("formal/README.md"),
        &standards_formal_scaffold(&manifest),
        &mut written,
        &mut existing,
    )?;
    write_scaffold_text_if_missing(
        &core_dir.join("synth/results.md"),
        &standards_synth_scaffold(&manifest),
        &mut written,
        &mut existing,
    )?;
    write_scaffold_text_if_missing(
        &core_dir.join("boards/README.md"),
        &standards_board_scaffold(&manifest),
        &mut written,
        &mut existing,
    )?;
    write_scaffold_text_if_missing(
        &core_dir.join(format!("ipxact/{}.xml", manifest.core)),
        &standards_ipxact_scaffold(&manifest),
        &mut written,
        &mut existing,
    )?;
    write_scaffold_text_if_missing(
        &core_dir.join(format!("regs/{}.rdl", manifest.core)),
        &standards_systemrdl_scaffold(&manifest),
        &mut written,
        &mut existing,
    )?;
    write_scaffold_text_if_missing(
        &core_dir.join("power/README.md"),
        &standards_power_na_scaffold(&manifest),
        &mut written,
        &mut existing,
    )?;
    write_scaffold_text_if_missing(
        &core_dir.join("dft/README.md"),
        &standards_dft_na_scaffold(&manifest),
        &mut written,
        &mut existing,
    )?;
    write_scaffold_text_if_missing(
        &core_dir.join(".verible.rules"),
        &standards_verible_scaffold(),
        &mut written,
        &mut existing,
    )?;
    write_scaffold_text_if_missing(
        &core_dir.join(".github/workflows/ci.yml"),
        &standards_ci_scaffold(&manifest),
        &mut written,
        &mut existing,
    )?;
    write_scaffold_text_if_missing(
        &core_dir.join("safety/safety_manual.md"),
        &standards_safety_scaffold(&manifest, safety_domain),
        &mut written,
        &mut existing,
    )?;
    write_scaffold_text_if_missing(
        &core_dir.join("security/threat_model.md"),
        &standards_threat_model_scaffold(&manifest),
        &mut written,
        &mut existing,
    )?;
    write_scaffold_text_if_missing(
        &core_dir.join("security/cwe_coverage.md"),
        &standards_cwe_scaffold(&manifest),
        &mut written,
        &mut existing,
    )?;
    write_scaffold_text_if_missing(
        &core_dir.join("security/sa-edi.json"),
        &standards_sa_edi_scaffold(&manifest),
        &mut written,
        &mut existing,
    )?;
    write_scaffold_json_if_missing(
        &core_dir.join(format!("hbom/{}.spdx.json", manifest.core)),
        &spdx_hbom_package(core_dir, &manifest, &checked.limitations),
        &mut written,
        &mut existing,
    )?;
    let manifest_artifacts_added = if declare {
        declare_standards_artifacts(core_dir, &profile, &manifest)?
    } else {
        Vec::new()
    };

    written.sort();
    existing.sort();
    Ok(CliOutput {
        human: format!(
            "standards scaffold wrote {} files, left {} existing files unchanged",
            written.len(),
            existing.len()
        ),
        json: json!({
            "status": "passed",
            "profile": profile.id,
            "profile_version": profile.version,
            "core": manifest.vlnv(),
            "written": written,
            "existing": existing,
            "declared": declare,
            "safety_domain": safety_domain,
            "manifest_artifacts_added": manifest_artifacts_added,
            "limitations": [
                "Scaffold files are evidence placeholders; fill in project-specific content before making buyer, safety, or security claims.",
                "Safety and security scaffold files are hooks only and do not claim certification."
            ],
        }),
    })
}

fn core_regs_scaffold(
    core_dir: &Path,
    output: Option<&Path>,
    declare: bool,
) -> Result<CliOutput, CliError> {
    let profile = load_standards_profile(FPGA_IP_CORE_PROFILE_ID)?;
    let manifest = load_manifest_from_core_dir(core_dir)?;
    let output_path = output
        .map(|path| resolve_core_output_path(core_dir, path))
        .unwrap_or_else(|| core_dir.join(format!("regs/{}.rdl", manifest.core)));
    let content = standards_systemrdl_scaffold(&manifest);
    write_text_file_creating_parent(&output_path, &content)?;
    let relative = relative_core_path(core_dir, &output_path).unwrap_or_else(|| {
        output_path
            .iter()
            .map(|component| component.to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    });
    let manifest_artifacts_added = if declare {
        let required_for = required_for_artifact_kind(&profile, "systemrdl");
        let sha256 = fs::read(&output_path)
            .map(|bytes| sha256_hex(&bytes))
            .unwrap_or_default();
        append_standard_artifacts(
            core_dir,
            &profile,
            &manifest,
            vec![StandardsArtifactDeclaration {
                kind: "systemrdl".to_string(),
                path: relative.clone(),
                category: category_for_required_for(&profile, &required_for),
                required_for,
                conclusion: "present".to_string(),
                sha256,
            }],
        )?
    } else {
        Vec::new()
    };
    Ok(CliOutput {
        human: format!("SystemRDL scaffold written: {}", output_path.display()),
        json: json!({
            "status": "passed",
            "core": manifest.vlnv(),
            "output": output_path,
            "declared": declare,
            "manifest_artifacts_added": manifest_artifacts_added,
            "limitations": [
                "Generated SystemRDL is a skeleton derived from the manifest; make it the single source of truth before CSR codegen claims."
            ],
        }),
    })
}

fn core_regs_check(core_dir: &Path, path: Option<&Path>) -> Result<CliOutput, CliError> {
    let checked = check_core(core_dir)?;
    let manifest = checked.manifest;
    let rdl_path = path
        .map(|path| resolve_core_output_path(core_dir, path))
        .unwrap_or_else(|| core_dir.join(format!("regs/{}.rdl", manifest.core)));
    let (validation_status, details) = if rdl_path.is_file() {
        validate_systemrdl_artifact(&rdl_path)
    } else {
        (
            "missing".to_string(),
            vec![format!(
                "SystemRDL file `{}` does not exist",
                rdl_path.display()
            )],
        )
    };
    let status = if validation_status == "semantic-valid" {
        "passed"
    } else {
        "blocked"
    };
    Ok(CliOutput {
        human: format!("SystemRDL check {status}: {}", rdl_path.display()),
        json: json!({
            "status": status,
            "core": manifest.vlnv(),
            "path": rdl_path,
            "validation_status": validation_status,
            "details": details,
        }),
    })
}

fn core_standards_spdx_audit(
    core_dir: &Path,
    output: Option<&Path>,
    declare: bool,
) -> Result<CliOutput, CliError> {
    let profile = load_standards_profile(FPGA_IP_CORE_PROFILE_ID)?;
    let manifest = load_manifest_from_core_dir(core_dir)?;
    let output_path = output
        .map(|path| resolve_core_output_path(core_dir, path))
        .unwrap_or_else(|| core_dir.join("reports/spdx-header-audit.json"));
    let audit = spdx_header_audit_report(core_dir, &manifest);
    write_json_file_creating_parent(&output_path, &audit)?;
    let status = audit["status"].as_str().unwrap_or("blocked").to_string();
    let relative = relative_core_path(core_dir, &output_path).unwrap_or_else(|| {
        output_path
            .iter()
            .map(|component| component.to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    });
    let manifest_artifacts_added = if declare {
        let required_for = required_for_artifact_kind(&profile, "spdx-header-audit");
        let sha256 = fs::read(&output_path)
            .map(|bytes| sha256_hex(&bytes))
            .unwrap_or_default();
        append_standard_artifacts(
            core_dir,
            &profile,
            &manifest,
            vec![StandardsArtifactDeclaration {
                kind: "spdx-header-audit".to_string(),
                path: relative.clone(),
                category: category_for_required_for(&profile, &required_for),
                required_for,
                conclusion: status.clone(),
                sha256,
            }],
        )?
    } else {
        Vec::new()
    };
    Ok(CliOutput {
        human: format!("SPDX header audit {status}: {}", output_path.display()),
        json: json!({
            "status": status,
            "core": manifest.vlnv(),
            "output": output_path,
            "summary": audit["summary"].clone(),
            "declared": declare,
            "manifest_artifacts_added": manifest_artifacts_added,
            "limitations": audit["limitations"].clone(),
        }),
    })
}

fn core_standards_collect(
    core_dir: &Path,
    build_root: &Path,
    profile_id: &str,
    declare: bool,
) -> Result<CliOutput, CliError> {
    let profile = load_standards_profile(profile_id)?;
    let checked = check_core(core_dir)?;
    let manifest = checked.manifest;
    let mut copied = Vec::new();
    let mut additions = Vec::new();
    for spec in standards_collect_specs(&manifest) {
        let source = build_root.join(spec.source);
        if !source.is_file() {
            continue;
        }
        let destination = core_dir.join(spec.destination);
        ensure_parent_dir(&destination)?;
        fs::copy(&source, &destination).map_err(|err| {
            CliError::new(
                "AF_STANDARDS_COLLECT_COPY_FAILED",
                format!(
                    "failed to copy `{}` to `{}`: {err}",
                    source.display(),
                    destination.display()
                ),
                "Check filesystem permissions and the selected --build-root.",
                5,
            )
        })?;
        copied.push(json!({
            "kind": spec.kind,
            "source": source,
            "destination": destination,
        }));
        if let Some(relative) = relative_core_path(core_dir, &destination) {
            let required_for = required_for_artifact_kind(&profile, spec.kind);
            let sha256 = fs::read(&destination)
                .map(|bytes| sha256_hex(&bytes))
                .unwrap_or_default();
            additions.push(StandardsArtifactDeclaration {
                kind: spec.kind.to_string(),
                path: relative,
                category: category_for_required_for(&profile, &required_for),
                required_for,
                conclusion: "present".to_string(),
                sha256,
            });
        }
    }
    let manifest_artifacts_added = if declare {
        append_standard_artifacts(core_dir, &profile, &manifest, additions)?
    } else {
        Vec::new()
    };
    let status = if copied.is_empty() {
        "blocked"
    } else {
        "passed"
    };
    Ok(CliOutput {
        human: format!(
            "standards collect {status}: copied {} artifacts",
            copied.len()
        ),
        json: json!({
            "status": status,
            "profile": profile.id,
            "profile_version": profile.version,
            "core": manifest.vlnv(),
            "build_root": build_root,
            "copied": copied,
            "declared": declare,
            "manifest_artifacts_added": manifest_artifacts_added,
            "limitations": if copied.is_empty() {
                vec!["No known standards artifacts were found under --build-root.".to_string()]
            } else {
                vec!["Collected CI/build outputs are linked into standards evidence; their internal verdicts remain owned by the producing tools.".to_string()]
            },
        }),
    })
}

#[derive(Clone, Debug)]
struct StandardsCollectSpec {
    kind: &'static str,
    source: String,
    destination: String,
}

fn standards_collect_specs(manifest: &CoreManifest) -> Vec<StandardsCollectSpec> {
    let core = manifest.core.as_str();
    vec![
        StandardsCollectSpec {
            kind: "native-lint",
            source: "reports/core-lint.json".to_string(),
            destination: "reports/standards/core-lint.json".to_string(),
        },
        StandardsCollectSpec {
            kind: "simulation-report",
            source: "reports/core-sim.json".to_string(),
            destination: "reports/standards/core-sim.json".to_string(),
        },
        StandardsCollectSpec {
            kind: "formal-report",
            source: "reports/core-formal.json".to_string(),
            destination: "reports/standards/core-formal.json".to_string(),
        },
        StandardsCollectSpec {
            kind: "spdx-hbom",
            source: format!("package/{core}.hbom.spdx.json"),
            destination: format!("hbom/{core}.spdx.json"),
        },
    ]
}

fn resolve_core_output_path(core_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        core_dir.join(path)
    }
}

fn declare_standards_artifacts(
    core_dir: &Path,
    profile: &StandardsProfile,
    manifest: &CoreManifest,
) -> Result<Vec<String>, CliError> {
    append_standard_artifacts(
        core_dir,
        profile,
        manifest,
        conventional_standard_artifact_declarations(core_dir, profile, manifest),
    )
}

fn conventional_standard_artifact_declarations(
    core_dir: &Path,
    profile: &StandardsProfile,
    manifest: &CoreManifest,
) -> Vec<StandardsArtifactDeclaration> {
    let mut additions = Vec::new();
    for (kind, path) in conventional_standard_artifact_paths(core_dir, manifest) {
        if !path.is_file() {
            continue;
        }
        let Some(relative) = relative_core_path(core_dir, &path) else {
            continue;
        };
        let required_for = required_for_artifact_kind(profile, &kind);
        if required_for.is_empty() {
            continue;
        }
        let category = category_for_required_for(profile, &required_for);
        let conclusion = if matches!(kind.as_str(), "power-na" | "jtag-na") {
            "not-applicable"
        } else {
            "present"
        };
        let sha256 = fs::read(&path)
            .map(|bytes| sha256_hex(&bytes))
            .unwrap_or_default();
        additions.push(StandardsArtifactDeclaration {
            kind,
            path: relative,
            category,
            required_for,
            conclusion: conclusion.to_string(),
            sha256,
        });
    }
    additions
}

fn append_standard_artifacts(
    core_dir: &Path,
    profile: &StandardsProfile,
    manifest: &CoreManifest,
    mut additions: Vec<StandardsArtifactDeclaration>,
) -> Result<Vec<String>, CliError> {
    let manifest_path = core_dir.join("af-core.toml");
    let mut raw = fs::read_to_string(&manifest_path).map_err(|err| {
        CliError::new(
            "AF_TOML_READ_FAILED",
            format!("failed to read `{}`: {err}", manifest_path.display()),
            "Check that the TOML file exists and is readable.",
            2,
        )
    })?;
    let existing = declared_standard_artifacts(manifest)
        .iter()
        .map(|artifact| format!("{}:{}", artifact.kind, artifact.path))
        .collect::<std::collections::BTreeSet<_>>();
    additions
        .retain(|artifact| !existing.contains(&format!("{}:{}", artifact.kind, artifact.path)));
    if additions.is_empty()
        && manifest
            .standards
            .as_ref()
            .and_then(|standards| standards.profile.as_ref())
            .is_some()
    {
        return Ok(Vec::new());
    }

    if !raw.ends_with('\n') {
        raw.push('\n');
    }
    if manifest.standards.is_none() {
        raw.push_str(&format!("\n[standards]\nprofile = \"{}\"\n", profile.id));
    } else if manifest
        .standards
        .as_ref()
        .and_then(|standards| standards.profile.as_ref())
        .is_none()
    {
        raw = insert_standards_profile(&raw, &profile.id);
    }
    for artifact in &additions {
        raw.push_str("\n[[standards.artifacts]]\n");
        raw.push_str(&format!("kind = \"{}\"\n", artifact.kind));
        raw.push_str(&format!("path = \"{}\"\n", artifact.path));
        raw.push_str(&format!("category = \"{}\"\n", artifact.category));
        raw.push_str(&format!(
            "required_for = [{}]\n",
            artifact
                .required_for
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
        raw.push_str(&format!("conclusion = \"{}\"\n", artifact.conclusion));
        if !artifact.sha256.is_empty() {
            raw.push_str(&format!("sha256 = \"{}\"\n", artifact.sha256));
        }
    }
    write_text_file_creating_parent(&manifest_path, &raw)?;
    Ok(additions
        .into_iter()
        .map(|artifact| format!("{}:{}", artifact.kind, artifact.path))
        .collect())
}

fn required_for_artifact_kind(profile: &StandardsProfile, kind: &str) -> Vec<u8> {
    profile
        .items
        .iter()
        .filter(|item| item.required_artifact_kinds.iter().any(|k| k == kind))
        .map(|item| item.id)
        .collect()
}

fn category_for_required_for(profile: &StandardsProfile, required_for: &[u8]) -> String {
    if required_for.iter().any(|id| {
        profile
            .items
            .iter()
            .any(|item| item.id == *id && item.category.contains("now"))
    }) {
        "now".to_string()
    } else {
        "foundation".to_string()
    }
}

#[derive(Clone, Debug)]
struct StandardsArtifactDeclaration {
    kind: String,
    path: String,
    category: String,
    required_for: Vec<u8>,
    conclusion: String,
    sha256: String,
}

fn insert_standards_profile(raw: &str, profile_id: &str) -> String {
    let mut out = String::new();
    let mut inserted = false;
    for line in raw.lines() {
        out.push_str(line);
        out.push('\n');
        if !inserted && line.trim() == "[standards]" {
            out.push_str(&format!("profile = \"{profile_id}\"\n"));
            inserted = true;
        }
    }
    if inserted {
        out
    } else {
        format!("{raw}\n[standards]\nprofile = \"{profile_id}\"\n")
    }
}

fn write_scaffold_text_if_missing(
    path: &Path,
    content: &str,
    written: &mut Vec<String>,
    existing: &mut Vec<String>,
) -> Result<(), CliError> {
    if path.exists() {
        existing.push(path.display().to_string());
        return Ok(());
    }
    write_text_file_creating_parent(path, content)?;
    written.push(path.display().to_string());
    Ok(())
}

fn write_scaffold_json_if_missing(
    path: &Path,
    content: &Value,
    written: &mut Vec<String>,
    existing: &mut Vec<String>,
) -> Result<(), CliError> {
    if path.exists() {
        existing.push(path.display().to_string());
        return Ok(());
    }
    write_json_file_creating_parent(path, content)?;
    written.push(path.display().to_string());
    Ok(())
}

fn standards_spec_scaffold(profile: &StandardsProfile, manifest: &CoreManifest) -> String {
    let mut out = format!(
        "# {} Standards Evidence Spec\n\nCore: `{}`\nProfile: `{}` version `{}`\n\n",
        manifest.core,
        manifest.vlnv(),
        profile.id,
        profile.version
    );
    out.push_str(
        "This scaffold records evidence anchors for the FPGA/IP-core checklist. Replace TODO text before release.\n\n",
    );
    for item in &profile.items {
        out.push_str(&format!("## {}. {}\n\n", item.id, item.item));
        out.push_str(&format!("- Category: `{}`\n", item.category));
        out.push_str(&format!("- Tier relevance: `{}`\n", item.tier_relevance));
        out.push_str(&format!(
            "- Required evidence: `{}`\n",
            item.required_evidence
        ));
        if item.category.contains("foundation") {
            out.push_str(
                "- Evidence status: N/A placeholder unless this core targets the domain.\n\n",
            );
        } else {
            out.push_str("- Evidence status: TODO - populate with core-specific evidence.\n\n");
        }
    }
    out
}

fn standards_datasheet_scaffold(manifest: &CoreManifest) -> String {
    format!(
        "# {} Datasheet\n\n## Overview\n\nTODO: describe purpose, interfaces, parameters, latency, resource targets, and integration notes for `{}`.\n",
        manifest.core,
        manifest.vlnv()
    )
}

fn standards_acceptance_scaffold(manifest: &CoreManifest) -> String {
    format!(
        "# {} Acceptance Criteria\n\n- CI must pass for lint, simulation, formal checks that are enabled, packaging, and standards evidence validation.\n- Release evidence must reference the exact commit and tool versions.\n",
        manifest.core
    )
}

fn standards_risks_scaffold(manifest: &CoreManifest) -> String {
    format!(
        "# {} Risks and Mitigations\n\n| Risk | Mitigation | Evidence |\n|---|---|---|\n| TODO | TODO | TODO |\n",
        manifest.core
    )
}

fn standards_sim_scaffold(manifest: &CoreManifest) -> String {
    format!(
        "# {} Simulation Plan\n\n- Exercise reset, nominal transfers, backpressure, parameter bounds, and protocol corner cases.\n- Record simulator, seed, pass/fail status, and generated artifacts.\n",
        manifest.core
    )
}

fn standards_formal_scaffold(manifest: &CoreManifest) -> String {
    format!(
        "# {} Formal Verification Plan\n\n- Define safety/liveness properties for handshakes, counters, FSM transitions, and reset convergence.\n- Mark properties as TODO until proof logs are captured.\n",
        manifest.core
    )
}

fn standards_synth_scaffold(manifest: &CoreManifest) -> String {
    format!(
        "# {} Synthesis Results\n\n| Tool | Target | Status | Fmax | LUT/ALM | FF | RAM | DSP |\n|---|---|---|---|---|---|---|---|\n| TODO | TODO | NOT MEASURED | NOT MEASURED | NOT MEASURED | NOT MEASURED | NOT MEASURED | NOT MEASURED |\n",
        manifest.core
    )
}

fn standards_board_scaffold(manifest: &CoreManifest) -> String {
    format!(
        "# {} Board Demo Plan\n\nNo board demo evidence is captured yet. Add board, constraints, programming flow, and observed behavior when available.\n",
        manifest.core
    )
}

fn standards_ipxact_scaffold(manifest: &CoreManifest) -> String {
    generate_ipxact_skeleton(manifest, None).content
}

fn standards_systemrdl_scaffold(manifest: &CoreManifest) -> String {
    let field_name = manifest
        .ports
        .iter()
        .find(|port| {
            let name = port.name.to_ascii_lowercase();
            name.contains("status") || name.contains("counter") || name.contains("cfg")
        })
        .map(|port| sanitize_rdl_identifier(&port.name))
        .unwrap_or_else(|| "generated_placeholder".to_string());
    format!(
        "// SPDX-License-Identifier: Apache-2.0\n// SystemRDL 2.0 skeleton generated by af from af-core.toml.\n// Treat this as the register single source of truth before generating RTL headers or UVM RAL.\naddrmap {} {{\n  reg {{\n    field {{ sw = r; hw = w; }} {}[1] = 0;\n  }} standards_status @ 0x0;\n}};\n",
        sanitize_rdl_identifier(&manifest.core),
        field_name
    )
}

fn standards_power_na_scaffold(manifest: &CoreManifest) -> String {
    format!(
        "# {} Power Intent\n\nN/A - generic FPGA core is treated as a single power domain unless an ASIC or low-power integration supplies IEEE 1801 UPF evidence.\n",
        manifest.core
    )
}

fn standards_dft_na_scaffold(manifest: &CoreManifest) -> String {
    format!(
        "# {} DFT / JTAG\n\nN/A - generic FPGA core does not include a JTAG TAP, BSDL fragment, or IEEE 1500 core test wrapper. Add DFT evidence only for SoC/ASIC integration variants.\n",
        manifest.core
    )
}

fn standards_verible_scaffold() -> String {
    "# Verible lint policy placeholder.\n# Keep rules aligned with the project Verilog-2001 portable subset.\n".to_string()
}

fn standards_ci_scaffold(manifest: &CoreManifest) -> String {
    format!(
        "name: {} CI\n\non:\n  push:\n  pull_request:\n\njobs:\n  af-core-check:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - name: Standards evidence placeholder\n        run: echo \"Wire af core standards check for {} in project CI\"\n",
        manifest.core, manifest.core
    )
}

fn standards_safety_scaffold(manifest: &CoreManifest, safety_domain: &str) -> String {
    let domain_section = match safety_domain {
        "automotive" => {
            "## Automotive Hook\n\n- Reference domain: ISO 26262:2018.\n- Reusable-IP pattern: Safety Element out of Context (SEooC).\n- Metrics to populate when requested: FIT target, SPFM, LFM, PMHF, fault model assumptions, integration constraints.\n"
        }
        "industrial" => {
            "## Industrial Hook\n\n- Reference domain: IEC 61508-2:2010.\n- Metrics to populate when requested: SIL target assumptions, diagnostic coverage, proof-test assumptions, fault model assumptions, integration constraints.\n"
        }
        "avionics" => {
            "## Avionics Hook\n\n- Reference domain: DO-254 / FAA AC 20-152A.\n- Evidence to populate when requested: DAL assumptions, planning traceability, verification objectives, tool qualification assumptions, integration constraints.\n"
        }
        _ => {
            "## Domain Hook\n\n- No target safety domain selected. Populate only when a buyer selects automotive, industrial, avionics, or another domain.\n"
        }
    };
    format!(
        "# {} Safety Manual Placeholder\n\nThis core is not safety-certified. This file is a forward-looking evidence hook only and must not be used as a certification claim.\n\n{}\n## Common Assumptions\n\n- Fault model: TODO.\n- Integration constraints: TODO.\n- Evidence owner: downstream safety program unless explicitly contracted.\n",
        manifest.core, domain_section
    )
}

fn standards_threat_model_scaffold(manifest: &CoreManifest) -> String {
    let assets = security_assets_from_manifest(manifest);
    let asset_rows = if assets.is_empty() {
        "| Asset | Kind | Direction | Width | Interface |\n|---|---|---|---|---|\n| TODO | TODO | TODO | TODO | TODO |\n".to_string()
    } else {
        let mut rows =
            "| Asset | Kind | Direction | Width | Interface |\n|---|---|---|---|---|\n".to_string();
        for asset in assets {
            rows.push_str(&format!(
                "| `{}` | {} | {} | {} | {} |\n",
                asset.name,
                asset.kind,
                asset.direction.unwrap_or_else(|| "n/a".to_string()),
                asset.width.unwrap_or_else(|| "n/a".to_string()),
                asset.interface.unwrap_or_else(|| "n/a".to_string())
            ));
        }
        rows
    };
    format!(
        "# {} Threat Model\n\nThis scaffold is not a security certification. It lists manifest-derived assets so CWE/SA-EDI coverage starts from real ports and interfaces.\n\n## Assets\n\n{}\n## Threats\n\n- TODO: list threat scenarios.\n\n## Risks\n\n- TODO: map risks to mitigations and evidence.\n",
        manifest.core, asset_rows
    )
}

fn standards_cwe_scaffold(manifest: &CoreManifest) -> String {
    format!(
        "# {} CWE Coverage\n\n| CWE | Applicability | Mitigation | Evidence |\n|---|---|---|---|\n| CWE-1194 | TODO | TODO | TODO |\n",
        manifest.core
    )
}

fn standards_sa_edi_scaffold(manifest: &CoreManifest) -> String {
    let assets = security_assets_from_manifest(manifest)
        .into_iter()
        .map(|asset| {
            json!({
                "name": asset.name,
                "kind": asset.kind,
                "direction": asset.direction,
                "width": asset.width,
                "interface": asset.interface,
            })
        })
        .collect::<Vec<_>>();
    to_pretty_json(&json!({
        "schema": "accellera.sa-edi.1.0.placeholder",
        "core": manifest.vlnv(),
        "assets": assets,
        "security_annotations": [],
        "limitations": [
            "Placeholder only; populate according to Accellera SA-EDI 1.0 before security claims."
        ],
    }))
}

#[derive(Clone, Debug)]
struct SecurityAsset {
    name: String,
    kind: String,
    direction: Option<String>,
    width: Option<String>,
    interface: Option<String>,
}

fn security_assets_from_manifest(manifest: &CoreManifest) -> Vec<SecurityAsset> {
    let mut assets = Vec::new();
    for port in &manifest.ports {
        if manifest.clocks.iter().any(|clock| {
            clock.name == port.name || clock.port.as_deref() == Some(port.name.as_str())
        }) || manifest.resets.iter().any(|reset| {
            reset.name == port.name || reset.port.as_deref() == Some(port.name.as_str())
        }) {
            continue;
        }
        assets.push(SecurityAsset {
            name: port.name.clone(),
            kind: port.kind.clone().unwrap_or_else(|| "port".to_string()),
            direction: Some(port.direction.clone()),
            width: port.width.as_ref().map(port_width_to_string),
            interface: port.interface.clone(),
        });
    }
    for interface in &manifest.interfaces {
        assets.push(SecurityAsset {
            name: interface.name.clone(),
            kind: format!("interface:{}", interface.kind),
            direction: None,
            width: None,
            interface: Some(interface.name.clone()),
        });
    }
    assets
}

fn port_width_to_string(width: &af_manifest::PortWidth) -> String {
    match width {
        af_manifest::PortWidth::Integer(value) => value.to_string(),
        af_manifest::PortWidth::Parameter(value) => value.clone(),
    }
}

fn sanitize_rdl_identifier(input: &str) -> String {
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
        "generated_placeholder".to_string()
    } else {
        out
    }
}

fn core_standards_check(
    core_dir: &Path,
    profile_id: &str,
    strict: bool,
) -> Result<CliOutput, CliError> {
    let profile = load_standards_profile(profile_id)?;
    let checked = check_core(core_dir)?;
    let manifest = checked.manifest;
    let mut rows = Vec::new();
    for item in &profile.items {
        rows.push(evaluate_standards_item(core_dir, &manifest, item, strict));
    }
    let summary = StandardsCheckSummary {
        total_items: rows.len(),
        supported_items: rows.iter().filter(|row| row.status == "supported").count(),
        blocked_items: rows.iter().filter(|row| row.status == "blocked").count(),
        planned_items: rows.iter().filter(|row| row.status == "planned").count(),
        foundation_items: rows
            .iter()
            .filter(|row| row.category.contains("foundation"))
            .count(),
    };
    let status = if summary.blocked_items == 0 {
        "passed"
    } else {
        "blocked"
    };
    let gates = standards_release_gates(&rows);
    let tool_availability = json!({
        "tools": standards_tool_availability(&profile),
    });
    Ok(CliOutput {
        human: format!(
            "standards check {status}: {} supported, {} blocked, {} planned",
            summary.supported_items, summary.blocked_items, summary.planned_items
        ),
        json: json!({
            "status": status,
            "profile": profile.id,
            "profile_version": profile.version,
            "core": manifest.vlnv(),
            "summary": summary,
            "rows": rows,
            "gates": gates,
            "tool_availability": tool_availability,
            "strict": strict,
            "limitations": [
                "Safety and security rows are evidence hooks only; this command does not claim certification.",
                if strict {
                    "Strict mode requires supported external validators for selected artifact kinds and fails closed when they are unavailable."
                } else {
                    "Artifact validation is deterministic and fail-closed. Full external certification and complete vendor-tool schema signoff remain outside this command."
                }
            ],
        }),
    })
}

fn standards_release_gates(rows: &[StandardsCheckRow]) -> Value {
    let blocked_now_rows = rows
        .iter()
        .filter(|row| row.category.contains("now") && row.status != "supported")
        .map(|row| {
            json!({
                "checklist_item_id": row.checklist_item_id,
                "item": row.item,
                "status": row.status,
                "validation_status": row.validation_status,
            })
        })
        .collect::<Vec<_>>();
    let status = if blocked_now_rows.is_empty() {
        "passed"
    } else {
        "blocked"
    };
    json!({
        "commercial_baseline_ready": {
            "status": status,
            "required_now_rows_missing": blocked_now_rows.len(),
            "blocked_rows": blocked_now_rows,
            "limitations": [
                "commercial-baseline-ready means all `now` rows have evidence; it is not a certification, safety approval, security evaluation, or vendor signoff.",
                "foundation rows may remain N/A or planned unless a buyer selects the corresponding domain."
            ],
        }
    })
}

fn evaluate_standards_item(
    core_dir: &Path,
    manifest: &CoreManifest,
    item: &StandardsChecklistItem,
    strict: bool,
) -> StandardsCheckRow {
    let artifact_validations = standard_item_artifact_validations(core_dir, manifest, item, strict);
    let aggregate_status = aggregate_validation_status(&artifact_validations);
    let missing_required_kinds = missing_required_artifact_kinds(item, &artifact_validations);
    let validation_status = if missing_required_kinds.is_empty()
        || matches!(aggregate_status.as_str(), "invalid" | "missing")
    {
        aggregate_status
    } else {
        "partial".to_string()
    };
    let evidence = artifact_validations
        .iter()
        .filter(|validation| validation.validation_status != "invalid")
        .flat_map(|validation| validation.evidence.clone())
        .collect::<Vec<_>>();
    let foundation = item.category.contains("foundation");
    let status = if matches!(
        validation_status.as_str(),
        "presence" | "schema-valid" | "semantic-valid" | "not-applicable"
    ) {
        "supported"
    } else if foundation {
        "planned"
    } else {
        "blocked"
    };
    let mut artifact_limitations = artifact_validations
        .iter()
        .flat_map(|validation| validation.limitations.clone())
        .collect::<Vec<_>>();
    let limitations = if artifact_validations.is_empty() {
        let kinds = if item.required_artifact_kinds.is_empty() {
            "no artifact kind".to_string()
        } else {
            item.required_artifact_kinds.join(", ")
        };
        if foundation {
            vec![format!(
                "Missing foundation hook evidence for checklist item {} ({kinds}); no certification claim is made.",
                item.id
            )]
        } else {
            vec![format!(
                "Missing standards evidence for checklist item {} ({kinds}).",
                item.id
            )]
        }
    } else {
        artifact_limitations.sort();
        artifact_limitations.dedup();
        artifact_limitations
    };
    let mut limitations = limitations;
    if !missing_required_kinds.is_empty() && !artifact_validations.is_empty() {
        limitations.push(format!(
            "Missing standards evidence for checklist item {} ({}).",
            item.id,
            missing_required_kinds.join(", ")
        ));
        limitations.sort();
        limitations.dedup();
    }
    StandardsCheckRow {
        checklist_item_id: item.id,
        item: item.item.clone(),
        category: item.category.clone(),
        status: status.to_string(),
        validation_status,
        standards: item.standards.clone(),
        evidence,
        limitations,
        required_artifact_kinds: item.required_artifact_kinds.clone(),
        artifact_validations,
    }
}

fn standard_item_artifact_validations(
    core_dir: &Path,
    manifest: &CoreManifest,
    item: &StandardsChecklistItem,
    strict: bool,
) -> Vec<StandardsArtifactValidation> {
    let mut validations = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for artifact in declared_standard_artifacts(manifest) {
        if item
            .required_artifact_kinds
            .iter()
            .any(|kind| kind == &artifact.kind)
            || artifact.required_for.contains(&item.id)
        {
            let path = core_dir.join(&artifact.path);
            if path.is_file() {
                let key = format!("{}:{}", artifact.kind, path.display());
                if seen.insert(key) {
                    validations.push(validate_standard_artifact(
                        core_dir,
                        manifest,
                        &artifact.kind,
                        &path,
                        "declared",
                        strict,
                    ));
                }
            }
        }
    }
    for (kind, path) in conventional_standard_artifact_paths(core_dir, manifest) {
        if item
            .required_artifact_kinds
            .iter()
            .any(|required| required == &kind)
            && path.is_file()
        {
            let key = format!("{kind}:{}", path.display());
            if seen.insert(key) {
                validations.push(validate_standard_artifact(
                    core_dir,
                    manifest,
                    &kind,
                    &path,
                    "conventional",
                    strict,
                ));
            }
        }
    }
    validations.sort_by(|lhs, rhs| {
        lhs.kind
            .cmp(&rhs.kind)
            .then_with(|| lhs.path.cmp(&rhs.path))
    });
    validations
}

fn aggregate_validation_status(validations: &[StandardsArtifactValidation]) -> String {
    for status in [
        "semantic-valid",
        "schema-valid",
        "presence",
        "not-applicable",
        "invalid",
    ] {
        if validations
            .iter()
            .any(|validation| validation.validation_status == status)
        {
            return status.to_string();
        }
    }
    "missing".to_string()
}

fn missing_required_artifact_kinds(
    item: &StandardsChecklistItem,
    validations: &[StandardsArtifactValidation],
) -> Vec<String> {
    let satisfied = validations
        .iter()
        .filter(|validation| validation.validation_status != "invalid")
        .map(|validation| validation.kind.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let mut missing = Vec::new();
    for group in required_artifact_kind_groups(&item.required_artifact_kinds) {
        if !group.iter().any(|kind| satisfied.contains(kind.as_str())) {
            missing.push(group.join("|"));
        }
    }
    missing
}

fn required_artifact_kind_groups(kinds: &[String]) -> Vec<Vec<String>> {
    let mut remaining = kinds
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let mut groups = Vec::new();
    for alternatives in [
        ["spdx-hbom", "cyclonedx-hbom"].as_slice(),
        ["upf", "power-na"].as_slice(),
        ["dft", "jtag-na"].as_slice(),
        ["verible-lint", "native-lint"].as_slice(),
    ] {
        if alternatives
            .iter()
            .all(|alternative| remaining.contains(*alternative))
        {
            groups.push(
                alternatives
                    .iter()
                    .map(|alternative| (*alternative).to_string())
                    .collect(),
            );
            for alternative in alternatives {
                remaining.remove(*alternative);
            }
        }
    }
    groups.extend(remaining.into_iter().map(|kind| vec![kind]));
    groups
}

fn validate_standard_artifact(
    core_dir: &Path,
    manifest: &CoreManifest,
    kind: &str,
    path: &Path,
    source: &str,
    strict: bool,
) -> StandardsArtifactValidation {
    let (mut validation_status, mut details) = match kind {
        "ip-xact" => validate_ipxact_artifact(manifest, path),
        "systemrdl" => validate_systemrdl_artifact(path),
        "spdx-hbom" => validate_spdx_hbom_artifact(path),
        "spdx-header-audit" => validate_spdx_header_audit_artifact(path),
        "cyclonedx-hbom" => validate_cyclonedx_hbom_artifact(path),
        "sa-edi" => validate_sa_edi_artifact(path),
        "security-threat-model" => validate_text_artifact(path, &["asset", "threat", "risk"]),
        "cwe-coverage" => validate_text_artifact(path, &["CWE", "CWE-"]),
        "upf" => validate_text_artifact(path, &["create_power_domain", "power_domain"]),
        "dft" => validate_text_artifact(path, &["CTL", "BSDL", "Boundary", "TAP"]),
        "power-na" | "jtag-na" => validate_na_artifact(path),
        _ => ("presence".to_string(), Vec::new()),
    };
    if strict && validation_status != "invalid" {
        if let Some((strict_status, strict_details)) = strict_validate_standard_artifact(kind, path)
        {
            validation_status = strict_status;
            details.extend(strict_details);
        }
    }
    let display_path = path.display().to_string();
    let evidence = if validation_status == "invalid" {
        Vec::new()
    } else {
        vec![format!(
            "{source} {kind} artifact: {display_path} ({validation_status})"
        )]
    };
    let limitations = if validation_status == "invalid" {
        vec![format!(
            "{kind} artifact semantic validation failed for `{display_path}`: {}",
            if details.is_empty() {
                "no details".to_string()
            } else {
                details.join("; ")
            }
        )]
    } else if !details.is_empty() {
        details
    } else if kind == "spdx-hbom" && !spdx_hbom_file_paths_match_core(core_dir, path) {
        vec![format!(
            "spdx-hbom artifact `{display_path}` is structurally valid, but file paths were not proven against the current core tree."
        )]
    } else {
        Vec::new()
    };
    StandardsArtifactValidation {
        kind: kind.to_string(),
        path: display_path,
        validation_status,
        evidence,
        limitations,
    }
}

fn strict_validate_standard_artifact(kind: &str, path: &Path) -> Option<(String, Vec<String>)> {
    match kind {
        "ip-xact" => Some(strict_run_file_validator("xmllint", &["--noout"], path)),
        "systemrdl" => Some(strict_run_file_validator("peakrdl", &["dump"], path)),
        "verible-lint" => Some(strict_require_tool("verible-verilog-lint")),
        _ => None,
    }
}

fn strict_require_tool(program: &str) -> (String, Vec<String>) {
    match std::process::Command::new(program)
        .arg("--version")
        .output()
    {
        Ok(output) if output.status.success() => ("semantic-valid".to_string(), Vec::new()),
        Ok(output) => (
            "invalid".to_string(),
            vec![format!(
                "strict validation command `{program} --version` failed with status {}: {}{}",
                output.status,
                String::from_utf8_lossy(&output.stdout).trim(),
                String::from_utf8_lossy(&output.stderr).trim()
            )],
        ),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => (
            "invalid".to_string(),
            vec![format!("strict validation requires `{program}` in PATH")],
        ),
        Err(err) => (
            "invalid".to_string(),
            vec![format!(
                "strict validation failed to run `{program}`: {err}"
            )],
        ),
    }
}

fn strict_run_file_validator(program: &str, args: &[&str], path: &Path) -> (String, Vec<String>) {
    let mut command = std::process::Command::new(program);
    for arg in args {
        command.arg(arg);
    }
    command.arg(path);
    match command.output() {
        Ok(output) if output.status.success() => ("semantic-valid".to_string(), Vec::new()),
        Ok(output) => (
            "invalid".to_string(),
            vec![format!(
                "strict validation command `{program}` failed for `{}` with status {}: {}{}",
                path.display(),
                output.status,
                String::from_utf8_lossy(&output.stdout).trim(),
                String::from_utf8_lossy(&output.stderr).trim()
            )],
        ),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => (
            "semantic-valid".to_string(),
            vec![format!(
                "external validator `{program}` unavailable; kept built-in semantic validation result"
            )],
        ),
        Err(err) => (
            "invalid".to_string(),
            vec![format!(
                "strict validation failed to run `{program}`: {err}"
            )],
        ),
    }
}

fn read_artifact_text(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|err| format!("failed to read artifact: {err}"))
}

fn validate_ipxact_artifact(manifest: &CoreManifest, path: &Path) -> (String, Vec<String>) {
    let text = match read_artifact_text(path) {
        Ok(text) => text,
        Err(err) => return ("invalid".to_string(), vec![err]),
    };
    let mut missing = Vec::new();
    if !text.contains("1685-2022") {
        missing.push("missing IEEE 1685-2022 namespace/version marker".to_string());
    }
    for tag in [
        "component",
        "vendor",
        "library",
        "name",
        "version",
        "busInterfaces",
        "busInterface",
        "model",
        "modelName",
        "fileSets",
        "fileSet",
        "file",
    ] {
        if !contains_xml_local_tag(&text, tag) {
            missing.push(format!("missing `{tag}` element"));
        }
    }
    for (tag, expected) in [
        ("vendor", manifest.vendor.as_str()),
        ("library", manifest.library.as_str()),
        ("name", manifest.core.as_str()),
        ("version", manifest.version.as_str()),
    ] {
        if !contains_xml_element_value(&text, tag, expected) {
            missing.push(format!("missing `{tag}` value `{expected}`"));
        }
    }
    if missing.is_empty() {
        ("semantic-valid".to_string(), Vec::new())
    } else {
        ("invalid".to_string(), missing)
    }
}

fn contains_xml_local_tag(text: &str, tag: &str) -> bool {
    text.contains(&format!("<{tag}"))
        || text.contains(&format!(":{tag}"))
        || text.contains(&format!("</{tag}>"))
        || text.contains(&format!(":{tag}>"))
}

fn contains_xml_element_value(text: &str, tag: &str, expected: &str) -> bool {
    ["ipxact", "spirit", ""].iter().any(|prefix| {
        let open = if prefix.is_empty() {
            format!("<{tag}>")
        } else {
            format!("<{prefix}:{tag}>")
        };
        let close = if prefix.is_empty() {
            format!("</{tag}>")
        } else {
            format!("</{prefix}:{tag}>")
        };
        text.contains(&format!("{open}{expected}{close}"))
    })
}

fn validate_systemrdl_artifact(path: &Path) -> (String, Vec<String>) {
    let text = match read_artifact_text(path) {
        Ok(text) => text,
        Err(err) => return ("invalid".to_string(), vec![err]),
    };
    if text.contains("addrmap") && (text.contains("reg ") || text.contains("field ")) {
        ("semantic-valid".to_string(), Vec::new())
    } else {
        (
            "invalid".to_string(),
            vec![
                "SystemRDL artifact must declare an addrmap and at least one reg/field".to_string(),
            ],
        )
    }
}

fn validate_spdx_header_audit_artifact(path: &Path) -> (String, Vec<String>) {
    let raw = match read_artifact_text(path) {
        Ok(text) => text,
        Err(err) => return ("invalid".to_string(), vec![err]),
    };
    let value: Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(err) => return ("invalid".to_string(), vec![format!("invalid JSON: {err}")]),
    };
    if value["kind"] != "accelfury.spdx_header_audit" {
        return (
            "invalid".to_string(),
            vec!["kind must be `accelfury.spdx_header_audit`".to_string()],
        );
    }
    let missing = value["summary"]["missing_headers"].as_u64().unwrap_or(1);
    if missing == 0 && value["files"].as_array().is_some() {
        ("semantic-valid".to_string(), Vec::new())
    } else {
        (
            "invalid".to_string(),
            vec![format!(
                "SPDX header audit reports {missing} files without SPDX-License-Identifier"
            )],
        )
    }
}

fn validate_spdx_hbom_artifact(path: &Path) -> (String, Vec<String>) {
    let raw = match read_artifact_text(path) {
        Ok(text) => text,
        Err(err) => return ("invalid".to_string(), vec![err]),
    };
    let value: Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(err) => return ("invalid".to_string(), vec![format!("invalid JSON: {err}")]),
    };
    let mut missing = Vec::new();
    if value["kind"] != "accelfury.hbom.spdx" {
        missing.push("kind must be `accelfury.hbom.spdx`".to_string());
    }
    if value["spdx_version"].as_str().is_none() {
        missing.push("missing `spdx_version`".to_string());
    }
    let Some(files) = value["files"].as_array() else {
        return (
            "invalid".to_string(),
            vec!["missing `files` array".to_string()],
        );
    };
    if files.is_empty() {
        missing.push("files array must not be empty".to_string());
    }
    for file in files {
        if file["path"].as_str().is_none() {
            missing.push("file entry missing `path`".to_string());
        }
        let checksum = &file["checksum"];
        if checksum["algorithm"] != "SHA256" {
            missing.push("file checksum algorithm must be SHA256".to_string());
        }
        let value = checksum["value"].as_str().unwrap_or_default();
        if value.len() != 64 || !value.chars().all(|ch| ch.is_ascii_hexdigit()) {
            missing.push("file checksum value must be 64 hex characters".to_string());
        }
    }
    if missing.is_empty() {
        ("semantic-valid".to_string(), Vec::new())
    } else {
        missing.sort();
        missing.dedup();
        ("invalid".to_string(), missing)
    }
}

fn spdx_header_audit_report(core_dir: &Path, manifest: &CoreManifest) -> Value {
    let mut files = standards_audit_candidate_files(core_dir, manifest)
        .into_iter()
        .map(|path| {
            let relative = relative_core_path(core_dir, &path).unwrap_or_else(|| {
                path.iter()
                    .map(|component| component.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/")
            });
            let identifier = file_spdx_license_identifier(&path);
            json!({
                "path": relative,
                "has_spdx_header": identifier.is_some(),
                "spdx_license_identifier": identifier,
            })
        })
        .collect::<Vec<_>>();
    files.sort_by(|lhs, rhs| {
        lhs["path"]
            .as_str()
            .unwrap_or_default()
            .cmp(rhs["path"].as_str().unwrap_or_default())
    });
    let checked_files = files.len();
    let missing_headers = files
        .iter()
        .filter(|file| !file["has_spdx_header"].as_bool().unwrap_or(false))
        .count();
    let status = if missing_headers == 0 {
        "passed"
    } else {
        "blocked"
    };
    json!({
        "generated_by": af_report::GENERATED_BY,
        "schema_version": "0.1",
        "kind": "accelfury.spdx_header_audit",
        "status": status,
        "core": manifest.vlnv(),
        "summary": {
            "checked_files": checked_files,
            "missing_headers": missing_headers,
        },
        "files": files,
        "limitations": [
            "Header audit checks SPDX-License-Identifier presence only; license compatibility still requires policy/legal review.",
            "Generated reports and external dependency trees are intentionally excluded from the scan."
        ],
    })
}

fn standards_audit_candidate_files(core_dir: &Path, manifest: &CoreManifest) -> Vec<PathBuf> {
    let mut seen = std::collections::BTreeSet::new();
    let mut files = Vec::new();
    for rel in &manifest.sources.files {
        let path = core_dir.join(rel);
        if path.is_file() && audited_extension(&path) && seen.insert(path.clone()) {
            files.push(path);
        }
    }
    for testbench in &manifest.testbenches {
        for rel in &testbench.sources {
            let path = core_dir.join(rel);
            if path.is_file() && audited_extension(&path) && seen.insert(path.clone()) {
                files.push(path);
            }
        }
    }
    collect_audit_files_recursive(core_dir, core_dir, &mut seen, &mut files);
    files
}

fn collect_audit_files_recursive(
    root: &Path,
    dir: &Path,
    seen: &mut std::collections::BTreeSet<PathBuf>,
    files: &mut Vec<PathBuf>,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if matches!(
                name.as_ref(),
                ".git" | ".af-build" | "target" | "node_modules" | "reports" | "hbom"
            ) {
                continue;
            }
            collect_audit_files_recursive(root, &path, seen, files);
        } else if audited_extension(&path)
            && path.strip_prefix(root).is_ok()
            && seen.insert(path.clone())
        {
            files.push(path);
        }
    }
}

fn audited_extension(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("v" | "sv" | "vh" | "svh" | "md" | "toml" | "rs")
    )
}

fn file_spdx_license_identifier(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    for line in text.lines().take(12) {
        if let Some((_, tail)) = line.split_once("SPDX-License-Identifier:") {
            let identifier = tail
                .trim()
                .trim_start_matches('#')
                .trim_start_matches("//")
                .trim_start_matches("/*")
                .trim_end_matches("*/")
                .trim();
            if !identifier.is_empty() {
                return Some(identifier.to_string());
            }
        }
    }
    None
}

fn spdx_hbom_file_paths_match_core(core_dir: &Path, path: &Path) -> bool {
    let Ok(raw) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<Value>(&raw) else {
        return false;
    };
    let Some(files) = value["files"].as_array() else {
        return false;
    };
    files
        .iter()
        .filter_map(|file| file["path"].as_str())
        .all(|rel| core_dir.join(rel).is_file())
}

fn validate_cyclonedx_hbom_artifact(path: &Path) -> (String, Vec<String>) {
    let raw = match read_artifact_text(path) {
        Ok(text) => text,
        Err(err) => return ("invalid".to_string(), vec![err]),
    };
    let value: Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(err) => return ("invalid".to_string(), vec![format!("invalid JSON: {err}")]),
    };
    if value["bomFormat"] == "CycloneDX" && value["components"].as_array().is_some() {
        ("schema-valid".to_string(), Vec::new())
    } else {
        (
            "invalid".to_string(),
            vec![
                "CycloneDX HBOM must contain bomFormat=CycloneDX and components array".to_string(),
            ],
        )
    }
}

fn validate_sa_edi_artifact(path: &Path) -> (String, Vec<String>) {
    let raw = match read_artifact_text(path) {
        Ok(text) => text,
        Err(err) => return ("invalid".to_string(), vec![err]),
    };
    let value: Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(err) => return ("invalid".to_string(), vec![format!("invalid JSON: {err}")]),
    };
    if value["assets"].as_array().is_some()
        || value["securityAnnotations"].as_array().is_some()
        || value["security_annotations"].as_array().is_some()
    {
        ("schema-valid".to_string(), Vec::new())
    } else {
        (
            "invalid".to_string(),
            vec!["SA-EDI artifact must contain assets or security annotations".to_string()],
        )
    }
}

fn validate_text_artifact(path: &Path, markers: &[&str]) -> (String, Vec<String>) {
    let text = match read_artifact_text(path) {
        Ok(text) => text,
        Err(err) => return ("invalid".to_string(), vec![err]),
    };
    if markers.iter().any(|marker| text.contains(marker)) {
        ("semantic-valid".to_string(), Vec::new())
    } else {
        (
            "invalid".to_string(),
            vec![format!(
                "artifact must contain one of: {}",
                markers.join(", ")
            )],
        )
    }
}

fn validate_na_artifact(path: &Path) -> (String, Vec<String>) {
    let text = match read_artifact_text(path) {
        Ok(text) => text.to_ascii_lowercase(),
        Err(err) => return ("invalid".to_string(), vec![err]),
    };
    if text.contains("n/a")
        || text.contains("not applicable")
        || text.contains("single power domain")
        || text.contains("fpga")
    {
        ("not-applicable".to_string(), Vec::new())
    } else {
        (
            "invalid".to_string(),
            vec!["N/A artifact must explicitly justify why the row is not applicable".to_string()],
        )
    }
}

fn declared_standard_artifacts(manifest: &CoreManifest) -> &[StandardsArtifact] {
    manifest
        .standards
        .as_ref()
        .map(|standards| standards.artifacts.as_slice())
        .unwrap_or(&[])
}

fn conventional_standard_artifact_paths(
    core_dir: &Path,
    manifest: &CoreManifest,
) -> Vec<(String, PathBuf)> {
    let core = manifest.core.as_str();
    vec![
        ("spec".to_string(), core_dir.join("docs/spec.md")),
        (
            "simulation-plan".to_string(),
            core_dir.join("sim/README.md"),
        ),
        ("formal-plan".to_string(), core_dir.join("formal/README.md")),
        (
            "synthesis-report".to_string(),
            core_dir.join("synth/results.md"),
        ),
        ("board-demo".to_string(), core_dir.join("boards/README.md")),
        ("datasheet".to_string(), core_dir.join("docs/datasheet.md")),
        ("license".to_string(), core_dir.join("LICENSE")),
        (
            "acceptance".to_string(),
            core_dir.join("docs/acceptance.md"),
        ),
        ("risks".to_string(), core_dir.join("docs/risks.md")),
        (
            "ip-xact".to_string(),
            core_dir.join(format!("ipxact/{core}.xml")),
        ),
        (
            "systemrdl".to_string(),
            core_dir.join(format!("regs/{core}.rdl")),
        ),
        (
            "upf".to_string(),
            core_dir.join(format!("power/{core}.upf")),
        ),
        ("power-na".to_string(), core_dir.join("power/README.md")),
        ("dft".to_string(), core_dir.join(format!("dft/{core}.ctl"))),
        ("jtag-na".to_string(), core_dir.join("dft/README.md")),
        ("verible-lint".to_string(), core_dir.join(".verible.rules")),
        (
            "native-lint".to_string(),
            core_dir.join("reports/core-lint.json"),
        ),
        (
            "native-lint".to_string(),
            core_dir.join("reports/standards/core-lint.json"),
        ),
        (
            "spdx-header-audit".to_string(),
            core_dir.join("reports/spdx-header-audit.json"),
        ),
        ("ci".to_string(), core_dir.join(".github/workflows/ci.yml")),
        (
            "safety-manual".to_string(),
            core_dir.join("safety/safety_manual.md"),
        ),
        (
            "security-threat-model".to_string(),
            core_dir.join("security/threat_model.md"),
        ),
        ("sa-edi".to_string(), core_dir.join("security/sa-edi.json")),
        (
            "cwe-coverage".to_string(),
            core_dir.join("security/cwe_coverage.md"),
        ),
        (
            "spdx-hbom".to_string(),
            core_dir.join(format!("hbom/{core}.spdx.json")),
        ),
        (
            "cyclonedx-hbom".to_string(),
            core_dir.join(format!("hbom/{core}.cdx.json")),
        ),
    ]
}

fn core_package(core_dir: &Path, build_root: &Path, format: &str) -> Result<CliOutput, CliError> {
    if !matches!(format, "manifest" | "tar.zst" | "spdx-hbom") {
        return Err(CliError::new(
            "AF_PACKAGE_FORMAT_UNSUPPORTED",
            format!("package format `{format}` is unsupported"),
            "Use --format manifest, tar.zst, or spdx-hbom.",
            2,
        ));
    }
    let checked = check_core(core_dir)?;
    let package_dir = build_root.join("package");
    fs::create_dir_all(&package_dir).map_err(|err| {
        CliError::new(
            "AF_PACKAGE_CREATE_DIR_FAILED",
            format!("failed to create `{}`: {err}", package_dir.display()),
            "Check filesystem permissions and the selected build root.",
            5,
        )
    })?;
    let package_path = if format == "spdx-hbom" {
        package_dir.join(format!("{}.hbom.spdx.json", checked.manifest.core))
    } else {
        package_dir.join(format!("{}-package-manifest.json", checked.manifest.core))
    };
    let package = if format == "spdx-hbom" {
        spdx_hbom_package(core_dir, &checked.manifest, &checked.limitations)
    } else {
        json!({
            "generated_by": af_report::GENERATED_BY,
            "schema_version": "0.1",
            "kind": "accelfury.package_manifest",
            "format": format,
            "core": checked.manifest.vlnv(),
            "sources": checked.manifest.sources.files.clone(),
            "testbenches": checked.manifest.testbenches.clone(),
            "limitations": checked.limitations.clone(),
        })
    };
    write_json_file(&package_path, &package)?;
    let mut report = AfReport::for_core("passed", &checked.manifest);
    report.artifacts.push(package_path.display().to_string());
    if format == "spdx-hbom" {
        report.limitations.push(
            "HBOM captures declared source/provenance metadata; it is not legal advice or a safety/security certification artifact."
                .to_string(),
        );
    } else {
        report.limitations.push(
            "MVP package command writes a package manifest descriptor; archive signing is future work."
                .to_string(),
        );
    }
    report.command_payload = Some(CommandPayload::Package(PackagePayload {
        format: format.to_string(),
        manifest_path: package_path.clone(),
    }));
    let written = write_reports_with_aliases(
        build_root.join("reports"),
        "core-package",
        &["package_report"],
        &mut report,
    )?;
    Ok(CliOutput {
        human: format!(
            "core package descriptor written: {}",
            package_path.display()
        ),
        json: json!({
            "status": "passed",
            "command_payload": report.command_payload,
            "package": package_path,
            "reports": written,
        }),
    })
}

fn spdx_hbom_package(core_dir: &Path, manifest: &CoreManifest, limitations: &[String]) -> Value {
    let mut file_roles = std::collections::BTreeMap::<String, String>::new();
    for path in &manifest.sources.files {
        let role = manifest
            .sources
            .roles
            .get(path)
            .map(String::as_str)
            .unwrap_or("rtl");
        file_roles.insert(path.clone(), role.to_string());
    }
    for artifact in declared_standard_artifacts(manifest) {
        if core_dir.join(&artifact.path).is_file() {
            file_roles
                .entry(artifact.path.clone())
                .or_insert_with(|| "standards-evidence".to_string());
        }
    }
    for (_kind, path) in conventional_standard_artifact_paths(core_dir, manifest) {
        if path.is_file() {
            if let Some(relative) = relative_core_path(core_dir, &path) {
                file_roles
                    .entry(relative)
                    .or_insert_with(|| "standards-evidence".to_string());
            }
        }
    }

    let files = file_roles
        .into_iter()
        .map(|(path, role)| {
            let absolute_path = core_dir.join(&path);
            let checksum = fs::read(&absolute_path)
                .map(|bytes| {
                    json!({
                        "algorithm": "SHA256",
                        "value": sha256_hex(&bytes),
                    })
                })
                .unwrap_or(Value::Null);
            let spdx_license_identifier = file_spdx_license_identifier(&absolute_path)
                .unwrap_or_else(|| "NOASSERTION".to_string());
            json!({
                "path": path,
                "role": role,
                "spdx_license_identifier": spdx_license_identifier,
                "checksum": checksum,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "generated_by": af_report::GENERATED_BY,
        "schema_version": "0.1",
        "kind": "accelfury.hbom.spdx",
        "spdx_version": "SPDX-3.0.1-compatible",
        "data_license": "CC0-1.0",
        "profile": FPGA_IP_CORE_PROFILE_ID,
        "core": manifest.vlnv(),
        "package_name": manifest.core,
        "package_version": manifest.version,
        "supplier": manifest.vendor,
        "release": {
            "semver": manifest.version,
            "signed_tag_required": true,
            "commit_sha": current_commit_sha(core_dir).unwrap_or_else(|| "unknown".to_string()),
            "dirty_tree": git_worktree_dirty(core_dir).unwrap_or(true),
            "tag": current_git_tag(core_dir),
            "tag_signature_status": git_tag_signature_status(core_dir),
        },
        "files": files,
        "limitations": limitations,
    })
}

fn git_worktree_dirty(core_dir: &Path) -> Option<bool> {
    let dir = git_command_dir(core_dir)?;
    let unstaged = std::process::Command::new("git")
        .args(["diff", "--quiet", "--ignore-submodules", "--"])
        .current_dir(&dir)
        .status()
        .ok()?;
    let staged = std::process::Command::new("git")
        .args(["diff", "--cached", "--quiet", "--ignore-submodules", "--"])
        .current_dir(&dir)
        .status()
        .ok()?;
    Some(!unstaged.success() || !staged.success())
}

fn current_git_tag(core_dir: &Path) -> Option<String> {
    let dir = git_command_dir(core_dir)?;
    let output = std::process::Command::new("git")
        .args(["describe", "--tags", "--exact-match", "HEAD"])
        .current_dir(&dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let tag = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if tag.is_empty() {
        None
    } else {
        Some(tag)
    }
}

fn git_tag_signature_status(core_dir: &Path) -> String {
    let Some(tag) = current_git_tag(core_dir) else {
        return "not-tagged".to_string();
    };
    let Some(dir) = git_command_dir(core_dir) else {
        return "unknown".to_string();
    };
    match std::process::Command::new("git")
        .args(["tag", "-v", &tag])
        .current_dir(&dir)
        .output()
    {
        Ok(output) if output.status.success() => "verified".to_string(),
        Ok(_) => "unverified".to_string(),
        Err(_) => "unknown".to_string(),
    }
}

fn git_command_dir(path: &Path) -> Option<PathBuf> {
    if path.is_dir() {
        Some(path.to_path_buf())
    } else {
        path.parent().map(Path::to_path_buf)
    }
}

fn relative_core_path(core_dir: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(core_dir).ok()?;
    Some(
        relative
            .iter()
            .map(|component| component.to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
    )
}

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

fn sha256_hex(input: &[u8]) -> String {
    let mut h = [
        0x6a09e667_u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (input.len() as u64) * 8;
    let mut data = input.to_vec();
    data.push(0x80);
    while (data.len() % 64) != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in data.chunks_exact(64) {
        let mut w = [0_u32; 64];
        for (idx, word) in chunk.chunks_exact(4).take(16).enumerate() {
            w[idx] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for idx in 16..64 {
            let s0 =
                w[idx - 15].rotate_right(7) ^ w[idx - 15].rotate_right(18) ^ (w[idx - 15] >> 3);
            let s1 = w[idx - 2].rotate_right(17) ^ w[idx - 2].rotate_right(19) ^ (w[idx - 2] >> 10);
            w[idx] = w[idx - 16]
                .wrapping_add(s0)
                .wrapping_add(w[idx - 7])
                .wrapping_add(s1);
        }
        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];
        for idx in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[idx])
                .wrapping_add(w[idx]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        for (slot, value) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *slot = slot.wrapping_add(value);
        }
    }
    h.iter().map(|word| format!("{word:08x}")).collect()
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
            .artifacts
            .extend(dependency_artifacts(&checked.dependency_resolutions));
        report
            .artifacts
            .extend(collect_core_report_surfaces(input, build_root));
        report.artifacts.sort();
        report.artifacts.dedup();
        let (ci_evidence, ci_warnings) = load_ci_evidence_records(build_root);
        report.warnings.extend(ci_warnings);
        let current_sha = current_commit_sha(input);
        let (placeholder_boards, board_warnings) = placeholder_boards_for(&checked.manifest);
        report.warnings.extend(board_warnings);
        report.maturity = Some(reusable_core_maturity(&MaturityInputs {
            manifest: Some(&checked.manifest),
            artifacts: &report.artifacts,
            warnings: &report.warnings,
            limitations: &report.limitations,
            ci_evidence: &ci_evidence,
            current_commit_sha: current_sha.as_deref(),
            placeholder_boards: &placeholder_boards,
        }));
        report.standards = standards_report_payload(input, &checked.manifest)?;
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
        let (ci_evidence, ci_warnings) = load_ci_evidence_records(build_root);
        report.warnings.extend(ci_warnings);
        let current_sha = current_commit_sha(input);
        report.maturity = Some(reusable_core_maturity(&MaturityInputs {
            manifest: None,
            artifacts: &report.artifacts,
            warnings: &report.warnings,
            limitations: &report.limitations,
            ci_evidence: &ci_evidence,
            current_commit_sha: current_sha.as_deref(),
            placeholder_boards: &[],
        }));
        report
    };
    if report.artifacts.is_empty() {
        report
            .warnings
            .push("No artifacts were discovered for the report input.".to_string());
    }
    let input_kind = if input.join("af-core.toml").is_file() {
        "core_with_manifest"
    } else {
        "build_dir_only"
    };
    let (maturity_verdict, blocked_rows) = match report.maturity.as_ref() {
        Some(m) => (
            m.verdict.clone(),
            m.rows.iter().filter(|r| r.status == "blocked").count(),
        ),
        None => ("unknown".to_string(), 0),
    };
    report.command_payload = Some(CommandPayload::Report(ReportPayload {
        input_kind: input_kind.to_string(),
        maturity_verdict,
        maturity_blocked_rows: blocked_rows,
        artifact_count: report.artifacts.len(),
    }));
    let standards_payload = report.standards.clone();
    let written = write_reports_with_aliases(
        build_root.join("reports"),
        "core-report",
        &["core_report"],
        &mut report,
    )?;
    Ok(CliOutput {
        human: format!(
            "core report written: {}, {}",
            written.json.display(),
            written.markdown.display()
        ),
        json: json!({
            "status": "passed",
            "command_payload": report.command_payload,
            "standards": standards_payload,
            "report": report,
            "reports": written,
        }),
    })
}

fn standards_report_payload(
    core_dir: &Path,
    manifest: &CoreManifest,
) -> Result<Option<Value>, CliError> {
    let Some(standards) = manifest.standards.as_ref() else {
        return Ok(None);
    };
    let profile_id = standards
        .profile
        .as_deref()
        .unwrap_or(FPGA_IP_CORE_PROFILE_ID);
    let profile = load_standards_profile(profile_id)?;
    let rows = profile
        .items
        .iter()
        .map(|item| evaluate_standards_item(core_dir, manifest, item, false))
        .collect::<Vec<_>>();
    let summary = StandardsCheckSummary {
        total_items: rows.len(),
        supported_items: rows.iter().filter(|row| row.status == "supported").count(),
        blocked_items: rows.iter().filter(|row| row.status == "blocked").count(),
        planned_items: rows.iter().filter(|row| row.status == "planned").count(),
        foundation_items: rows
            .iter()
            .filter(|row| row.category.contains("foundation"))
            .count(),
    };
    let status = if summary.blocked_items == 0 {
        "passed"
    } else {
        "blocked"
    };
    let gates = standards_release_gates(&rows);
    Ok(Some(json!({
        "status": status,
        "profile": profile.id,
        "profile_version": profile.version,
        "summary": summary,
        "rows": rows,
        "gates": gates,
        "limitations": [
            "Safety and security rows are evidence hooks only; this report does not claim certification.",
            "Standards evidence in this report is deterministic and fail-closed; external certification and vendor signoff remain out of scope."
        ],
    })))
}

fn tier_required_rows(tier: &str) -> Result<&'static [&'static str], CliError> {
    match tier {
        "community" => Ok(&["manifest_contract", "source_portability"]),
        "verified-package" => Ok(&[
            "manifest_contract",
            "source_portability",
            "open_source_tool_evidence",
            "wrapper_package_compatibility",
            "docker_ci_cd_evidence",
        ]),
        "enterprise" => Ok(&[
            "manifest_contract",
            "source_portability",
            "open_source_tool_evidence",
            "wrapper_package_compatibility",
            "docker_ci_cd_evidence",
            "vendor_tool_evidence",
            "board_hardware_evidence",
            "release_support_legal_evidence",
        ]),
        other => Err(CliError::new(
            "AF_TIER_UNKNOWN",
            format!("unknown tier `{other}`"),
            "Use --tier community, verified-package, or enterprise.",
            2,
        )),
    }
}

fn core_verify(core_dir: &Path, tier: &str, build_root: &Path) -> Result<CliOutput, CliError> {
    let required = tier_required_rows(tier)?;

    let checked = check_core(core_dir)?;
    let mut artifacts: Vec<String> = checked
        .inspection
        .scanned_files
        .iter()
        .map(|path| path.display().to_string())
        .collect();
    artifacts.extend(dependency_artifacts(&checked.dependency_resolutions));
    artifacts.extend(collect_core_report_surfaces(core_dir, build_root));
    artifacts.sort();
    artifacts.dedup();

    let mut warnings = checked.warnings.clone();
    let (ci_evidence, ci_warnings) = load_ci_evidence_records(build_root);
    warnings.extend(ci_warnings);
    let current_sha = current_commit_sha(core_dir);
    let (placeholder_boards, board_warnings) = placeholder_boards_for(&checked.manifest);
    warnings.extend(board_warnings);

    let maturity = reusable_core_maturity(&MaturityInputs {
        manifest: Some(&checked.manifest),
        artifacts: &artifacts,
        warnings: &warnings,
        limitations: &checked.manifest.known_limitations,
        ci_evidence: &ci_evidence,
        current_commit_sha: current_sha.as_deref(),
        placeholder_boards: &placeholder_boards,
    });

    let mut missing: Vec<serde_json::Value> = Vec::new();
    for area in required {
        let Some(row) = maturity.rows.iter().find(|r| r.area == *area) else {
            missing.push(json!({
                "area": area,
                "status": "absent",
                "limitations": ["evidence row not produced by core report pipeline"],
            }));
            continue;
        };
        if row.status != "supported" {
            missing.push(json!({
                "area": row.area,
                "status": row.status,
                "evidence": row.evidence,
                "limitations": row.limitations,
            }));
        }
    }

    let payload = json!({
        "tier": tier,
        "required_rows": required,
        "missing": missing,
        "core": checked.manifest.vlnv(),
        "maturity_verdict": maturity.verdict,
    });

    if missing.is_empty() {
        Ok(CliOutput {
            human: format!(
                "{} satisfies the `{tier}` tier (all {} required rows supported)",
                checked.manifest.vlnv(),
                required.len()
            ),
            json: json!({
                "status": "passed",
                "tier_verification": payload,
            }),
        })
    } else {
        Err(CliError::new(
            "AF_TIER_REQUIREMENTS_UNMET",
            format!(
                "{} does not satisfy `{tier}` tier ({} required row(s) unmet)",
                checked.manifest.vlnv(),
                missing.len()
            ),
            "Generate evidence for the missing rows (see docs/licensing.md::Commercial tiers) or pick a lower tier.",
            2,
        )
        .with_details(&payload))
    }
}

fn build(
    core_dir: &Path,
    build_root: &Path,
    board: &str,
    backend: &str,
) -> Result<CliOutput, CliError> {
    match backend {
        "litex" => {
            let target = WrapperTarget::parse("litex")?;
            let wrapper = generate_wrapper(core_dir, build_root, target, Some(board))?;
            let checked = check_core(core_dir)?;
            let mut report = AfReport::for_core("passed", &checked.manifest);
            report.artifacts.extend(
                wrapper
                    .artifacts
                    .iter()
                    .map(|path| path.display().to_string()),
            );
            report.warnings.extend(wrapper.warnings);
            report.limitations.extend(wrapper.limitations);
            report.limitations.push(
                "LiteX build is a reference dry-run skeleton; no vendor timing or bitstream is produced."
                    .to_string(),
            );
            report.command_payload = Some(CommandPayload::Build(BuildPayload {
                backend: backend.to_string(),
                backend_status: "passed".to_string(),
                board: board.to_string(),
            }));
            let written = write_reports_with_aliases(
                build_root.join("reports"),
                "build-report",
                &["build_report"],
                &mut report,
            )?;
            Ok(CliOutput {
                human: format!("build dry-run passed with litex for board `{board}`"),
                json: json!({
                    "status": "passed",
                    "backend": backend,
                    "board": board,
                    "artifacts": wrapper.artifacts,
                    "reports": written,
                }),
            })
        }
        "yosys" => {
            let checked = check_core(core_dir)?;
            let backend_report = YosysBackend::process()
                .lint(&checked.manifest, core_dir, build_root)
                .map_err(|err| {
                    CliError::new(err.code(), err.to_string(), err.hint(), err.exit_code())
                })?;
            let mut report =
                AfReport::for_core(status_text(&backend_report.status), &checked.manifest);
            report.merge_backend(&backend_report);
            report.command_payload = Some(CommandPayload::Build(BuildPayload {
                backend: backend.to_string(),
                backend_status: status_text(&backend_report.status).to_string(),
                board: board.to_string(),
            }));
            report.limitations.push(
                "Yosys build mode is a syntax/synthesis smoke check; it does not produce a bitstream."
                    .to_string(),
            );
            let written = write_reports_with_aliases(
                build_root.join("reports"),
                "build-report",
                &["build_report"],
                &mut report,
            )?;
            match backend_report.status {
                BackendStatus::Passed => Ok(CliOutput {
                    human: format!("build smoke passed with yosys for board `{board}`"),
                    json: json!({
                        "status": "passed",
                        "backend": backend,
                        "board": board,
                        "backend_report": backend_report,
                        "reports": written,
                    }),
                }),
                BackendStatus::Unavailable => Err(CliError::new(
                    "AF_BACKEND_UNAVAILABLE",
                    "build backend `yosys` is unavailable",
                    "Use the Docker runtime or install yosys in PATH.",
                    4,
                )
                .with_details(&json!({
                    "backend_report": backend_report,
                    "reports": written,
                }))),
                BackendStatus::Failed => Err(CliError::new(
                    "AF_BUILD_FAILED",
                    "Yosys build smoke failed",
                    "Inspect backend command details in the report.",
                    9,
                )
                .with_details(&json!({
                    "backend_report": backend_report,
                    "reports": written,
                }))),
            }
        }
        "nextpnr" => {
            let checked = check_core(core_dir)?;
            let backend_report = NextpnrBackend::process().doctor().map_err(|err| {
                CliError::new(err.code(), err.to_string(), err.hint(), err.exit_code())
            })?;
            let plan = af_backend_nextpnr::plan_nextpnr(board, build_root);
            let mut report =
                AfReport::for_core(status_text(&backend_report.status), &checked.manifest);
            report.merge_backend(&backend_report);
            report.artifacts.extend(
                plan.expected_artifacts
                    .iter()
                    .map(|path| path.display().to_string()),
            );
            report.limitations.extend(plan.limitations.clone());
            let plan_path = build_root.join("reports/nextpnr-plan.json");
            write_json_file(&plan_path, &plan)?;
            report.artifacts.push(plan_path.display().to_string());
            let written = write_reports_with_aliases(
                build_root.join("reports"),
                "build-report",
                &["build_report"],
                &mut report,
            )?;
            Ok(CliOutput {
                human: format!("nextpnr P&R plan written for board `{board}`"),
                json: json!({
                    "status": status_text(&backend_report.status),
                    "backend": backend,
                    "board": board,
                    "plan": plan,
                    "reports": written,
                }),
            })
        }
        other => Err(CliError::new(
            "AF_BUILD_BACKEND_UNAVAILABLE",
            format!("build backend `{other}` is unavailable"),
            "Use --backend litex, --backend yosys, or --backend nextpnr for first-release open-source dry-run paths; vendor production backends are planned later.",
            9,
        )),
    }
}

fn flash(build_dir: &Path, backend: &str) -> Result<CliOutput, CliError> {
    let payload = CommandPayload::Flash(FlashPayload {
        backend: backend.to_string(),
        backend_status: "unavailable".to_string(),
    });
    Err(CliError::new(
        "AF_FLASH_UNAVAILABLE",
        format!(
            "flash backend `{backend}` is unavailable for `{}`",
            build_dir.display()
        ),
        "Flash support requires a produced bitstream artifact and is staged for MVP-2/3.",
        10,
    )
    .with_details(&json!({
        "command_payload": payload,
        "capabilities": af_backend_flash::capabilities(),
    })))
}

fn clean(build_root: &Path, yes: bool) -> Result<CliOutput, CliError> {
    if !yes {
        return Err(CliError::new(
            "AF_CLEAN_CONFIRMATION_REQUIRED",
            format!("refusing to clean `{}` without --yes", build_root.display()),
            "Pass --yes to remove the selected build root.",
            2,
        ));
    }
    if !build_root.exists() {
        return Ok(CliOutput {
            human: format!("build root already clean: {}", build_root.display()),
            json: json!({
                "status": "passed",
                "removed": false,
                "build_root": build_root,
            }),
        });
    }
    fs::remove_dir_all(build_root).map_err(|err| {
        CliError::new(
            "AF_CLEAN_FAILED",
            format!("failed to remove `{}`: {err}", build_root.display()),
            "Check filesystem permissions or choose a writable build root.",
            5,
        )
    })?;
    Ok(CliOutput {
        human: format!("build root removed: {}", build_root.display()),
        json: json!({
            "status": "passed",
            "removed": true,
            "build_root": build_root,
        }),
    })
}

fn backend_run(
    backend: &str,
    target: &str,
    core_dir: Option<&PathBuf>,
    build_root: &Path,
) -> Result<CliOutput, CliError> {
    match (backend, target) {
        ("native" | "af-native", "check" | "lint" | "portable-check") => {
            let core_dir = required_backend_core(core_dir, "native portable-check")?;
            core_lint(core_dir, build_root, "native")
        }
        ("verilator", "lint") => {
            let core_dir = required_backend_core(core_dir, "verilator lint")?;
            core_lint(core_dir, build_root, backend)
        }
        ("verilator", "sim") | ("verilator", "simulate") => {
            let core_dir = required_backend_core(core_dir, "verilator sim")?;
            core_sim(core_dir, build_root, backend)
        }
        ("icarus" | "iverilog", "lint" | "check" | "elaborate") => {
            let core_dir = required_backend_core(core_dir, "icarus lint")?;
            core_lint(core_dir, build_root, "icarus")
        }
        ("icarus" | "iverilog", "sim" | "simulate") => {
            let core_dir = required_backend_core(core_dir, "icarus sim")?;
            core_sim(core_dir, build_root, "icarus")
        }
        ("yosys", "lint") | ("yosys", "syntax") | ("yosys", "synth") => {
            let core_dir = required_backend_core(core_dir, "yosys lint")?;
            core_lint(core_dir, build_root, backend)
        }
        ("sby", "formal" | "prove") => {
            let core_dir = required_backend_core(core_dir, "sby formal")?;
            core_formal(core_dir, build_root, "sby")
        }
        ("nextpnr", "doctor" | "check") => {
            let backend_report = NextpnrBackend::process().doctor().map_err(|err| {
                CliError::new(err.code(), err.to_string(), err.hint(), err.exit_code())
            })?;
            Ok(CliOutput {
                human: format!("nextpnr backend {:?}", backend_report.status),
                json: json!({
                    "status": status_text(&backend_report.status),
                    "backend_report": backend_report,
                }),
            })
        }
        ("litex", "generate-wrapper") => {
            let core_dir = required_backend_core(core_dir, "litex generate-wrapper")?;
            commands::wrapper::wrapper_generate(core_dir, build_root, "litex", None)
        }
        _ => Err(CliError::new(
            "AF_BACKEND_RUN_UNSUPPORTED",
            format!("backend run target `{backend}:{target}` is unsupported"),
            "Use `af backend list` to inspect the available MVP backend capabilities.",
            2,
        )),
    }
}

fn required_backend_core<'a>(
    core_dir: Option<&'a PathBuf>,
    target: &str,
) -> Result<&'a PathBuf, CliError> {
    core_dir.ok_or_else(|| {
        CliError::new(
            "AF_BACKEND_RUN_CORE_REQUIRED",
            format!("backend run {target} requires --core-dir"),
            "Pass --core-dir <path> so the backend can load af-core.toml.",
            2,
        )
    })
}

fn ci_generate(
    target: &str,
    output: Option<&PathBuf>,
    backends: &[String],
    optional_fail_closed: bool,
) -> Result<CliOutput, CliError> {
    let output = output
        .cloned()
        .unwrap_or_else(|| PathBuf::from(".github/workflows/accelfury.yml"));
    let artifact = af_ci::write_with_options(
        target,
        &output,
        &af_ci::CiGenerateOptions {
            backends: backends.to_vec(),
            optional_fail_closed,
        },
    )?;
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

/// Persist stdout/stderr of every command in `report.commands` to sidecar
/// log files under `<build_root>/logs/<base_name>-NN.{out,err}.log`, and
/// reference each artifact from the corresponding `CommandRecord`. Best-effort:
/// I/O failures are silently ignored — log persistence must never fail the
/// underlying command. M3 contract from docs/dev-roadmap.md ("write
/// stdout/stderr logs as artifacts and reference them from reports").
fn persist_backend_logs(report: &mut AfReport, build_root: &Path, base_name: &str) {
    let log_dir = build_root.join("logs");
    if let Ok(paths) = af_backend::persist_command_logs(&mut report.commands, &log_dir, base_name) {
        for path in paths {
            report.artifacts.push(path.display().to_string());
        }
    }
}

fn write_reports_with_aliases(
    output_dir: impl AsRef<Path>,
    base_name: &str,
    aliases: &[&str],
    report: &mut AfReport,
) -> Result<WrittenReports, ReportError> {
    let output_dir = output_dir.as_ref();
    // M3 contract: every report written to disk carries deterministic
    // reproducibility metadata (host OS/arch + env hash of tool versions).
    // Mutating in place propagates the metadata to any subsequent json!(report)
    // that the caller serialises to stdout.
    if report.reproducibility.is_none() {
        report.reproducibility = Some(af_report::Reproducibility::capture(&report.tool_versions));
    }
    let written = write_reports(output_dir, base_name, report)?;
    for alias in aliases {
        if *alias != base_name {
            write_reports(output_dir, alias, report)?;
        }
    }
    Ok(written)
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
            let version = if output.exit_code == Some(0) {
                ToolVersion::available(program, text)
            } else {
                ToolVersion::unavailable(
                    program,
                    format!("command exited with {:?}: {text}", output.exit_code),
                )
            };
            (version, vec![CommandRecord::from(output)])
        }
        Err(err) => (
            ToolVersion::unavailable(program, err.to_string()),
            Vec::new(),
        ),
    }
}

fn probe_python_module(
    runner: &impl CommandRunner,
    module: &str,
) -> (ToolVersion, Vec<CommandRecord>) {
    let code = format!("import {module}; print(getattr({module}, '__version__', 'import ok'))");
    let spec = CommandSpec::new("python3").args(["-c".to_string(), code]);
    match runner.run(&spec) {
        Ok(output) => {
            let text = output
                .stdout
                .lines()
                .chain(output.stderr.lines())
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or("python module probe output was empty")
                .to_string();
            let version = if output.exit_code == Some(0) {
                ToolVersion::available(module, text)
            } else {
                ToolVersion::unavailable(
                    module,
                    format!("python import exited with {:?}: {text}", output.exit_code),
                )
            };
            (version, vec![CommandRecord::from(output)])
        }
        Err(err) => (
            ToolVersion::unavailable(module, err.to_string()),
            Vec::new(),
        ),
    }
}

fn probe_deno_audit_readiness(runner: &impl CommandRunner) -> (ToolVersion, Vec<CommandRecord>) {
    let Some(root) = find_deno_workspace_root() else {
        return (
            ToolVersion::unavailable(
                "deno-audit-repo",
                "deno.json with an audit:repo task was not found from the current directory",
            ),
            Vec::new(),
        );
    };
    let spec = CommandSpec::new("deno").args(["task"]).cwd(root);
    match runner.run(&spec) {
        Ok(output) => {
            let text = output
                .stdout
                .lines()
                .chain(output.stderr.lines())
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            let version = if output.exit_code == Some(0) && text.contains("audit:repo") {
                ToolVersion::available(
                    "deno-audit-repo",
                    "deno task audit:repo is declared in deno.json",
                )
            } else if output.exit_code == Some(0) {
                ToolVersion::unavailable(
                    "deno-audit-repo",
                    "deno task audit:repo is not declared in deno.json",
                )
            } else {
                ToolVersion::unavailable(
                    "deno-audit-repo",
                    format!(
                        "deno task listing exited with {:?}: {}",
                        output.exit_code,
                        if text.is_empty() {
                            "no output"
                        } else {
                            text.as_str()
                        }
                    ),
                )
            };
            (version, vec![CommandRecord::from(output)])
        }
        Err(err) => (
            ToolVersion::unavailable("deno-audit-repo", err.to_string()),
            Vec::new(),
        ),
    }
}

fn find_deno_workspace_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir.join("deno.json").is_file() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
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

fn load_ci_evidence_records(build_root: &Path) -> (Vec<CiEvidenceRecord>, Vec<String>) {
    let mut records = Vec::new();
    let mut warnings = Vec::new();
    let evidence_dir = build_root.join("reports").join("evidence");
    let Ok(entries) = fs::read_dir(&evidence_dir) else {
        return (records, warnings);
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.starts_with("ci_run_report-") && name.ends_with(".json"))
                    .unwrap_or(false)
        })
        .collect();
    paths.sort();
    for path in paths {
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(err) => {
                warnings.push(format!(
                    "failed to read CI evidence report `{}`: {err}",
                    path.display()
                ));
                continue;
            }
        };
        let value: Value = match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(err) => {
                warnings.push(format!(
                    "failed to parse CI evidence report `{}`: {err}",
                    path.display()
                ));
                continue;
            }
        };
        let Some(ci_run) = value.get("ci_run") else {
            warnings.push(format!(
                "CI evidence report `{}` does not contain a `ci_run` block",
                path.display()
            ));
            continue;
        };
        let mut record: CiEvidenceRecord = match serde_json::from_value(ci_run.clone()) {
            Ok(record) => record,
            Err(err) => {
                warnings.push(format!(
                    "failed to decode `ci_run` block from `{}`: {err}",
                    path.display()
                ));
                continue;
            }
        };
        record.source_path = Some(path.display().to_string());
        records.push(record);
    }
    (records, warnings)
}

/// Determine which of `manifest.boards` are not `verified_on_hardware` in the
/// registry. Best-effort: if the registry cannot be loaded, emit a warning but
/// do not fail report generation.
fn placeholder_boards_for(manifest: &CoreManifest) -> (Vec<String>, Vec<String>) {
    if manifest.boards.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let registry_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut placeholders = Vec::new();
    let mut warnings = Vec::new();
    match af_board_db::list_boards(&registry_root) {
        Ok(entries) => {
            use std::collections::BTreeMap;
            let by_id: BTreeMap<&str, &af_board_db::BoardEntry> =
                entries.iter().map(|e| (e.board_id.as_str(), e)).collect();
            for board in &manifest.boards {
                match by_id.get(board.as_str()) {
                    Some(entry) => {
                        if !board_is_verified(&entry.exact_pinout_status) {
                            placeholders.push(board.clone());
                        }
                    }
                    None => {
                        // Unknown to registry: treat as placeholder for honesty.
                        placeholders.push(board.clone());
                        warnings.push(format!(
                            "Manifest declares board `{board}` that is not present in registries/boards.registry.json."
                        ));
                    }
                }
            }
        }
        Err(err) => {
            warnings.push(format!(
                "Could not load board registry to evaluate manifest boards: {err}; treating all declared boards as draft for the maturity row."
            ));
            placeholders.extend(manifest.boards.iter().cloned());
        }
    }
    (placeholders, warnings)
}

fn current_commit_sha(core_dir: &Path) -> Option<String> {
    let dir = if core_dir.is_dir() {
        core_dir.to_path_buf()
    } else {
        core_dir.parent()?.to_path_buf()
    };
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let sha = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if sha.is_empty() {
        None
    } else {
        Some(sha)
    }
}

fn dependency_artifacts(resolutions: &[CoreDependencyResolution]) -> Vec<String> {
    let mut artifacts = Vec::new();
    for resolution in resolutions {
        artifacts.push(format!(
            "dependency:{}:{}",
            resolution.vlnv,
            resolution.manifest_path.display()
        ));
        for source in &resolution.source_files {
            artifacts.push(format!(
                "dependency:{}:{}",
                resolution.vlnv,
                source.display()
            ));
        }
    }
    artifacts
}

fn collect_core_report_surfaces(core_dir: &Path, build_root: &Path) -> Vec<String> {
    let mut artifacts = Vec::new();
    for rel in [
        "af-ci.toml",
        ".github/workflows",
        "reports",
        "vendor",
        "constructor",
    ] {
        collect_path_if_present(&core_dir.join(rel), &mut artifacts);
    }
    for rel in [
        "reports",
        "evidence",
        "fusesoc",
        "litex",
        "ipxact",
        "constructor",
    ] {
        collect_path_if_present(&build_root.join(rel), &mut artifacts);
    }
    artifacts.sort();
    artifacts.dedup();
    artifacts
}

fn collect_path_if_present(path: &Path, artifacts: &mut Vec<String>) {
    if path.is_file() {
        artifacts.push(path.display().to_string());
        return;
    }
    if !path.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_path_if_present(&path, artifacts);
        } else if path.is_file() {
            artifacts.push(path.display().to_string());
        }
    }
}

pub(crate) fn write_new_file(path: &Path, contents: &[u8]) -> Result<(), CliError> {
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

pub(crate) fn read_toml_file<T: DeserializeOwned>(path: &Path) -> Result<T, CliError> {
    let raw = fs::read_to_string(path).map_err(|err| {
        CliError::new(
            "AF_TOML_READ_FAILED",
            format!("failed to read `{}`: {err}", path.display()),
            "Check that the TOML file exists and is readable.",
            2,
        )
    })?;
    toml::from_str(&raw).map_err(|err| {
        CliError::new(
            "AF_TOML_PARSE_FAILED",
            format!("failed to parse `{}`: {err}", path.display()),
            "Fix the TOML syntax before retrying.",
            2,
        )
    })
}

pub(crate) fn write_json_file<T: Serialize>(path: &Path, value: &T) -> Result<(), CliError> {
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

fn write_json_file_creating_parent<T: Serialize>(path: &Path, value: &T) -> Result<(), CliError> {
    ensure_parent_dir(path)?;
    write_json_file(path, value)
}

fn write_text_file_creating_parent(path: &Path, value: &str) -> Result<(), CliError> {
    ensure_parent_dir(path)?;
    fs::write(path, value).map_err(|err| {
        CliError::new(
            "AF_WRITE_FAILED",
            format!("failed to write `{}`: {err}", path.display()),
            "Check filesystem permissions and the selected output path.",
            5,
        )
    })
}

fn ensure_parent_dir(path: &Path) -> Result<(), CliError> {
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
    Ok(())
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

pub(crate) fn to_pretty_json<T: Serialize>(value: &T) -> String {
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
