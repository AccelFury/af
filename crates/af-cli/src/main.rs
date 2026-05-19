// SPDX-License-Identifier: Apache-2.0
mod agent;
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
use af_core::{check_core, load_manifest_from_core_dir, load_validated_manifest, CoreError};
use af_manifest::{CoreManifest, ManifestError, ManifestValidationReport};
use af_report::{
    reusable_core_maturity, write_reports, AfReport, BuildPayload, CheckPayload, CiEvidenceRecord,
    CommandPayload, DoctorPayload, FlashPayload, FormalPayload, LintPayload, MaturityInputs,
    PackagePayload, ReportError, ReportPayload, SimulationPayload, ToolingPayload, WrittenReports,
};
use af_vectors::{generate_mod_add_vectors, GenerateConfig};
use af_wrapper_gen::{generate_wrapper, WrapperGenError, WrapperTarget};
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
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },
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
        Commands::Agent { command } => format!("agent {}", agent_command_name(command)),
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
        CoreCommand::Report { .. } => "report",
        CoreCommand::Verify { .. } => "verify",
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
                portability_level,
                priority,
                maturity,
            } => {
                let axes = commands::core_new::AxesOverride::from_cli(
                    portability_level.as_deref(),
                    priority.as_deref(),
                    maturity.as_deref(),
                )?;
                commands::core_new::core_new(
                    core_dir,
                    name,
                    class.as_deref(),
                    library,
                    language,
                    profile,
                    axes,
                )
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

fn core_package(core_dir: &Path, build_root: &Path, format: &str) -> Result<CliOutput, CliError> {
    if !matches!(format, "manifest" | "tar.zst") {
        return Err(CliError::new(
            "AF_PACKAGE_FORMAT_UNSUPPORTED",
            format!("package format `{format}` is unsupported"),
            "Use --format manifest or --format tar.zst. The MVP writes a deterministic manifest package descriptor.",
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
    let package_path = package_dir.join(format!("{}-package-manifest.json", checked.manifest.core));
    let package = json!({
        "generated_by": af_report::GENERATED_BY,
        "schema_version": "0.1",
        "kind": "accelfury.package_manifest",
        "format": format,
        "core": checked.manifest.vlnv(),
        "sources": checked.manifest.sources.files.clone(),
        "testbenches": checked.manifest.testbenches.clone(),
        "limitations": checked.limitations.clone(),
    });
    write_json_file(&package_path, &package)?;
    let mut report = AfReport::for_core("passed", &checked.manifest);
    report.artifacts.push(package_path.display().to_string());
    report.limitations.push(
        "MVP package command writes a package manifest descriptor; archive signing/SBOM are future work."
            .to_string(),
    );
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
            "report": report,
            "reports": written,
        }),
    })
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
