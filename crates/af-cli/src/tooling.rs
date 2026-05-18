// SPDX-License-Identifier: Apache-2.0
use super::{probe_deno_audit_readiness, probe_python_module, probe_tool, CliError, CliOutput};
use af_backend::{CommandRecord, CommandRunner, CommandSpec, ProcessCommandRunner, ToolVersion};
use af_security::ToolchainManifest;
use clap::{Args, Subcommand, ValueEnum};
use serde::Serialize;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Subcommand, Debug)]
pub enum ToolingCommand {
    Check(ToolingCheckArgs),
    Plan(ToolingPlanArgs),
    Ensure(ToolingEnsureArgs),
}

#[derive(Args, Debug)]
pub struct ToolingCheckArgs {
    #[command(flatten)]
    selection: ToolingSelection,
}

#[derive(Args, Debug)]
pub struct ToolingPlanArgs {
    #[command(flatten)]
    selection: ToolingSelection,
    #[arg(long)]
    allow_network: bool,
    #[arg(long)]
    allow_system: bool,
}

#[derive(Args, Debug)]
pub struct ToolingEnsureArgs {
    #[command(flatten)]
    selection: ToolingSelection,
    #[arg(long)]
    yes: bool,
    #[arg(long)]
    allow_network: bool,
    #[arg(long)]
    allow_system: bool,
}

#[derive(Args, Clone, Debug)]
struct ToolingSelection {
    #[arg(long, value_enum, default_value = "oss")]
    profile: ToolingProfile,
    #[arg(long, value_enum, default_value = "docker")]
    install_mode: InstallMode,
    #[arg(long, value_delimiter = ',')]
    tools: Vec<String>,
    #[arg(long, default_value = "af-toolchain.toml")]
    toolchain_manifest: PathBuf,
    #[arg(long, default_value = ".af-tools")]
    tool_root: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum ToolingProfile {
    Oss,
    Full,
    Vendor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum InstallMode {
    Docker,
    User,
    System,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ToolingActionKind {
    Check,
    Plan,
    Ensure,
}

#[derive(Clone, Debug)]
enum ProbeKind {
    Command {
        program: &'static str,
        args: &'static [&'static str],
    },
    PythonModule {
        module: &'static str,
    },
    DenoAuditReadiness,
}

#[derive(Clone, Debug)]
struct ToolDefinition {
    id: &'static str,
    manifest_key: Option<&'static str>,
    probe: ProbeKind,
    capability: &'static str,
    docker_runtime: bool,
    user_python_package: Option<&'static str>,
    system_package: Option<&'static str>,
    vendor_manual: bool,
}

#[derive(Debug, Serialize)]
struct ToolingReport {
    schema_version: &'static str,
    kind: &'static str,
    status: String,
    action: ToolingActionKind,
    profile: ToolingProfile,
    install_mode: InstallMode,
    toolchain_manifest: PathBuf,
    tool_root: PathBuf,
    policy: ToolingPolicySnapshot,
    tools: Vec<ToolingToolStatus>,
    planned_actions: Vec<ToolingInstallAction>,
    executed_commands: Vec<CommandRecord>,
    probe_commands: Vec<CommandRecord>,
    warnings: Vec<String>,
    blockers: Vec<ToolingBlocker>,
    limitations: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ToolingPolicySnapshot {
    offline: bool,
    allow_network: bool,
    allow_untrusted_scripts: bool,
    allow_shell: bool,
    effective_allow_network: bool,
}

#[derive(Debug, Serialize)]
struct ToolingToolStatus {
    tool: String,
    status: String,
    required: bool,
    available: bool,
    capability: String,
    install_strategy: String,
    probe: ToolVersion,
}

#[derive(Clone, Debug, Serialize)]
struct ToolingInstallAction {
    id: String,
    provider: String,
    tools: Vec<String>,
    commands: Vec<CommandSpec>,
    writes_to: Vec<String>,
    requires_network: bool,
    requires_sudo: bool,
    executable: bool,
    reason: String,
}

#[derive(Debug, Serialize)]
struct ToolingBlocker {
    code: String,
    tool: Option<String>,
    message: String,
    hint: String,
}

pub fn execute(command: &ToolingCommand) -> Result<CliOutput, CliError> {
    match command {
        ToolingCommand::Check(args) => tooling_check(&args.selection),
        ToolingCommand::Plan(args) => tooling_plan(args),
        ToolingCommand::Ensure(args) => tooling_ensure(args),
    }
}

fn tooling_check(selection: &ToolingSelection) -> Result<CliOutput, CliError> {
    let report = build_tooling_report(
        ToolingActionKind::Check,
        selection,
        false,
        false,
        &ProcessCommandRunner,
    )?;
    Ok(CliOutput {
        human: human_summary("tooling check", &report),
        json: json!(report),
    })
}

fn tooling_plan(args: &ToolingPlanArgs) -> Result<CliOutput, CliError> {
    let report = build_tooling_report(
        ToolingActionKind::Plan,
        &args.selection,
        args.allow_network,
        args.allow_system,
        &ProcessCommandRunner,
    )?;
    Ok(CliOutput {
        human: human_summary("tooling plan", &report),
        json: json!(report),
    })
}

fn tooling_ensure(args: &ToolingEnsureArgs) -> Result<CliOutput, CliError> {
    let runner = ProcessCommandRunner;
    let mut report = build_tooling_report(
        ToolingActionKind::Ensure,
        &args.selection,
        args.allow_network,
        args.allow_system,
        &runner,
    )?;
    let runnable_actions: Vec<_> = report
        .planned_actions
        .iter()
        .filter(|action| action.executable)
        .collect();

    if !args.yes && !runnable_actions.is_empty() {
        return Err(CliError::new(
            "AF_TOOLING_CONFIRMATION_REQUIRED",
            "tooling ensure refuses to install tools without --yes",
            "Review `af tooling plan --json`, then rerun with --yes and any required policy flags.",
            2,
        )
        .with_details(&report));
    }

    if args.selection.install_mode == InstallMode::System && !args.allow_system {
        return Err(CliError::new(
            "AF_TOOLING_SYSTEM_CONFIRMATION_REQUIRED",
            "system package installation requires --allow-system",
            "Prefer --install-mode docker or --install-mode user, or pass --allow-system intentionally.",
            2,
        )
        .with_details(&report));
    }

    if report
        .blockers
        .iter()
        .any(|blocker| blocker.code == "AF_TOOLING_POLICY_BLOCKED")
    {
        return Err(CliError::new(
            "AF_TOOLING_POLICY_BLOCKED",
            "tooling installation is blocked by af-toolchain policy",
            "Pass --allow-network for this explicit run or update af-toolchain.toml policy intentionally.",
            11,
        )
        .with_details(&report));
    }

    if runnable_actions.is_empty() {
        if report.tools.iter().all(|tool| tool.available) {
            report.status = "passed".to_string();
            return Ok(CliOutput {
                human: "tooling ensure passed: selected tools are already available".to_string(),
                json: json!(report),
            });
        }
        return Err(CliError::new(
            "AF_TOOLING_BLOCKED",
            "selected missing tools cannot be installed automatically by this mode",
            "Use `af tooling plan --json` and follow the manual blockers, or choose a different --install-mode.",
            4,
        )
        .with_details(&report));
    }

    let actions = report.planned_actions.clone();
    for action in &actions {
        if !action.executable {
            continue;
        }
        for spec in &action.commands {
            let output = runner.run(spec).map_err(|err| {
                CliError::new(
                    "AF_TOOLING_INSTALL_FAILED",
                    format!("tooling install command `{}` failed: {err}", spec.program),
                    "Inspect the command record and rerun after fixing host permissions, PATH, or network access.",
                    err.exit_code(),
                )
            })?;
            let record = CommandRecord::from(output);
            if record.exit_code != Some(0) {
                report.executed_commands.push(record);
                return Err(CliError::new(
                    "AF_TOOLING_INSTALL_FAILED",
                    "tooling install command exited unsuccessfully",
                    "Inspect executed_commands in the JSON details and rerun after fixing the command failure.",
                    4,
                )
                .with_details(&report));
            }
            report.executed_commands.push(record);
        }
    }
    report.status = if report.blockers.is_empty() {
        "passed".to_string()
    } else {
        "warning".to_string()
    };

    Ok(CliOutput {
        human: human_summary("tooling ensure", &report),
        json: json!(report),
    })
}

fn build_tooling_report(
    action: ToolingActionKind,
    selection: &ToolingSelection,
    allow_network_override: bool,
    allow_system: bool,
    runner: &impl CommandRunner,
) -> Result<ToolingReport, CliError> {
    let manifest = load_toolchain_manifest(&selection.toolchain_manifest)?;
    let effective_allow_network =
        allow_network_override || (!manifest.policy.offline && manifest.policy.allow_network);
    let selected_ids = selected_tool_ids(selection)?;
    let definitions = tool_definitions();
    let definitions_by_id: BTreeMap<&str, ToolDefinition> = definitions
        .into_iter()
        .map(|definition| (definition.id, definition))
        .collect();

    let mut tools = Vec::new();
    let mut probe_commands = Vec::new();
    for id in selected_ids {
        let definition = definitions_by_id.get(id.as_str()).ok_or_else(|| {
            CliError::new(
                "AF_TOOLING_TOOL_UNKNOWN",
                format!("unknown tooling id `{id}`"),
                "Use `af tooling check --json` without --tools to inspect supported tool ids.",
                2,
            )
        })?;
        let (probe, commands) = probe_definition(runner, definition);
        probe_commands.extend(commands);
        let available = probe.available;
        let required = required_by_manifest(&manifest, definition);
        let install_strategy = install_strategy(selection.install_mode, definition);
        let status = if available { "available" } else { "missing" };
        tools.push(ToolingToolStatus {
            tool: definition.id.to_string(),
            status: status.to_string(),
            required,
            available,
            capability: definition.capability.to_string(),
            install_strategy,
            probe,
        });
    }

    let mut warnings = Vec::new();
    let mut blockers = Vec::new();
    let mut planned_actions = if action == ToolingActionKind::Check {
        Vec::new()
    } else {
        plan_install_actions(
            selection,
            &tools,
            &definitions_by_id,
            effective_allow_network,
            allow_system,
            &mut blockers,
        )
    };

    if tools.iter().any(|tool| !tool.available) {
        warnings.push(
            "One or more selected tools are unavailable; affected af flows remain blocked until tooling is installed or a container runtime is used."
                .to_string(),
        );
    }

    if !effective_allow_network {
        for action in &mut planned_actions {
            if action.requires_network {
                action.executable = false;
                blockers.push(ToolingBlocker {
                    code: "AF_TOOLING_POLICY_BLOCKED".to_string(),
                    tool: None,
                    message: format!(
                        "action `{}` requires network access but af-toolchain policy is offline/no-network",
                        action.id
                    ),
                    hint: "Pass --allow-network for this explicit run or update af-toolchain.toml intentionally."
                        .to_string(),
                });
            }
        }
    }

    let status = if blockers
        .iter()
        .any(|blocker| blocker.code == "AF_TOOLING_POLICY_BLOCKED")
    {
        "blocked"
    } else if !warnings.is_empty() || !blockers.is_empty() {
        "warning"
    } else {
        "passed"
    };

    Ok(ToolingReport {
        schema_version: "0.1",
        kind: "accelfury.tooling_report",
        status: status.to_string(),
        action,
        profile: selection.profile,
        install_mode: selection.install_mode,
        toolchain_manifest: selection.toolchain_manifest.clone(),
        tool_root: selection.tool_root.clone(),
        policy: ToolingPolicySnapshot {
            offline: manifest.policy.offline,
            allow_network: manifest.policy.allow_network,
            allow_untrusted_scripts: manifest.policy.allow_untrusted_scripts,
            allow_shell: manifest.policy.allow_shell,
            effective_allow_network,
        },
        tools,
        planned_actions,
        executed_commands: Vec::new(),
        probe_commands,
        warnings,
        blockers,
        limitations: vec![
            "Vendor tools such as gw_sh and programmer_cli are detect-only because licenses, installers, and EULAs are outside af ownership.".to_string(),
            "User-local Python installs are isolated under --tool-root and require PATH integration by the caller before external tools can see their console scripts.".to_string(),
        ],
    })
}

fn load_toolchain_manifest(path: &Path) -> Result<ToolchainManifest, CliError> {
    ToolchainManifest::from_path(path).map_err(|err| {
        CliError::new(
            "AF_TOOLING_MANIFEST_UNREADABLE",
            format!(
                "failed to read toolchain manifest `{}`: {err}",
                path.display()
            ),
            "Run from the repository root or pass --toolchain-manifest <path>.",
            2,
        )
    })
}

fn selected_tool_ids(selection: &ToolingSelection) -> Result<Vec<String>, CliError> {
    if !selection.tools.is_empty() {
        return selection
            .tools
            .iter()
            .map(|tool| normalize_tool_id(tool))
            .collect();
    }

    const OSS_TOOLS: &[&str] = &[
        "iverilog",
        "vvp",
        "verilator",
        "yosys",
        "nextpnr-ice40",
        "nextpnr-ecp5",
        "nextpnr-gowin",
        "fusesoc",
        "edalize",
        "xmllint",
        "sby",
        "boolector",
        "z3",
        "yices-smt2",
        "bitwuzla",
        "cvc5",
        "deno",
        "deno-audit-repo",
        "litex",
        "openfpgaloader",
    ];
    const FULL_TOOLS: &[&str] = &[
        "iverilog",
        "vvp",
        "verilator",
        "yosys",
        "nextpnr-ice40",
        "nextpnr-ecp5",
        "nextpnr-gowin",
        "fusesoc",
        "edalize",
        "xmllint",
        "sby",
        "boolector",
        "z3",
        "yices-smt2",
        "bitwuzla",
        "cvc5",
        "deno",
        "deno-audit-repo",
        "litex",
        "openfpgaloader",
        "gw_sh",
        "programmer_cli",
    ];
    const VENDOR_TOOLS: &[&str] = &["gw_sh", "programmer_cli"];
    let ids = match selection.profile {
        ToolingProfile::Oss => OSS_TOOLS,
        ToolingProfile::Full => FULL_TOOLS,
        ToolingProfile::Vendor => VENDOR_TOOLS,
    };
    Ok(ids.iter().map(|id| (*id).to_string()).collect())
}

fn normalize_tool_id(tool: &str) -> Result<String, CliError> {
    let normalized = tool.trim().to_ascii_lowercase().replace(['_', ' '], "-");
    let id = match normalized.as_str() {
        "iverilog" | "icarus" | "icarus-verilog" => "iverilog",
        "vvp" => "vvp",
        "verilator" => "verilator",
        "yosys" => "yosys",
        "nextpnr-ice40" | "ice40" => "nextpnr-ice40",
        "nextpnr-ecp5" | "ecp5" => "nextpnr-ecp5",
        "nextpnr-gowin" | "gowin-nextpnr" => "nextpnr-gowin",
        "fusesoc" => "fusesoc",
        "edalize" => "edalize",
        "xmllint" | "xml-lint" | "libxml2-utils" => "xmllint",
        "sby" | "symbiyosys" => "sby",
        "boolector" => "boolector",
        "z3" => "z3",
        "yices" | "yices2" | "yices-smt2" => "yices-smt2",
        "bitwuzla" => "bitwuzla",
        "cvc5" => "cvc5",
        "deno" => "deno",
        "deno-audit-repo" | "audit-repo" => "deno-audit-repo",
        "litex" => "litex",
        "openfpgaloader" | "open-fpga-loader" => "openfpgaloader",
        "gw-sh" => "gw_sh",
        "programmer-cli" => "programmer_cli",
        _ => {
            return Err(CliError::new(
                "AF_TOOLING_TOOL_UNKNOWN",
                format!("unknown tooling id `{tool}`"),
                "Use one of: iverilog, vvp, verilator, yosys, nextpnr-ice40, nextpnr-ecp5, nextpnr-gowin, fusesoc, edalize, xmllint, sby, boolector, z3, yices-smt2, bitwuzla, cvc5, deno, deno-audit-repo, litex, openfpgaloader, gw_sh, programmer_cli.",
                2,
            ));
        }
    };
    Ok(id.to_string())
}

fn probe_definition(
    runner: &impl CommandRunner,
    definition: &ToolDefinition,
) -> (ToolVersion, Vec<CommandRecord>) {
    match &definition.probe {
        ProbeKind::Command { program, args } => probe_tool(runner, program, args),
        ProbeKind::PythonModule { module } => probe_python_module(runner, module),
        ProbeKind::DenoAuditReadiness => probe_deno_audit_readiness(runner),
    }
}

fn required_by_manifest(manifest: &ToolchainManifest, definition: &ToolDefinition) -> bool {
    definition
        .manifest_key
        .and_then(|key| manifest.tools.get(key))
        .is_some_and(|tool| tool.required)
}

fn install_strategy(mode: InstallMode, definition: &ToolDefinition) -> String {
    if definition.vendor_manual {
        return "manual-vendor-detect-only".to_string();
    }
    match mode {
        InstallMode::Docker if definition.docker_runtime => "docker-runtime".to_string(),
        InstallMode::Docker => "manual-or-user-local".to_string(),
        InstallMode::User if definition.user_python_package.is_some() => {
            "af-managed-python-venv".to_string()
        }
        InstallMode::User => "manual-or-docker-runtime".to_string(),
        InstallMode::System if definition.system_package.is_some() => "system-package".to_string(),
        InstallMode::System => "manual".to_string(),
    }
}

fn plan_install_actions(
    selection: &ToolingSelection,
    tools: &[ToolingToolStatus],
    definitions_by_id: &BTreeMap<&str, ToolDefinition>,
    effective_allow_network: bool,
    allow_system: bool,
    blockers: &mut Vec<ToolingBlocker>,
) -> Vec<ToolingInstallAction> {
    let missing: Vec<_> = tools.iter().filter(|tool| !tool.available).collect();
    for tool in &missing {
        let definition = definitions_by_id
            .get(tool.tool.as_str())
            .expect("selected ids are validated before planning");
        if definition.vendor_manual {
            blockers.push(ToolingBlocker {
                code: "AF_TOOLING_VENDOR_MANUAL".to_string(),
                tool: Some(tool.tool.clone()),
                message: format!("`{}` is vendor tooling and is detect-only", tool.tool),
                hint: "Install vendor EDA/programmer tools manually according to the vendor license and add them to PATH."
                    .to_string(),
            });
        }
    }

    match selection.install_mode {
        InstallMode::Docker => {
            plan_docker_actions(selection, &missing, definitions_by_id, blockers)
        }
        InstallMode::User => plan_user_actions(selection, &missing, definitions_by_id, blockers),
        InstallMode::System => plan_system_actions(
            selection,
            &missing,
            definitions_by_id,
            effective_allow_network,
            allow_system,
            blockers,
        ),
    }
}

fn plan_docker_actions(
    selection: &ToolingSelection,
    missing: &[&ToolingToolStatus],
    definitions_by_id: &BTreeMap<&str, ToolDefinition>,
    blockers: &mut Vec<ToolingBlocker>,
) -> Vec<ToolingInstallAction> {
    let mut docker_tools = Vec::new();
    for tool in missing {
        let definition = definitions_by_id
            .get(tool.tool.as_str())
            .expect("selected ids are validated before planning");
        if definition.docker_runtime {
            docker_tools.push(tool.tool.clone());
        } else if !definition.vendor_manual {
            blockers.push(ToolingBlocker {
                code: "AF_TOOLING_DOCKER_UNSUPPORTED".to_string(),
                tool: Some(tool.tool.clone()),
                message: format!("`{}` is not installed by the Docker runtime action", tool.tool),
                hint: "Use --install-mode user for Python-local tools where supported, or install this host tool manually."
                    .to_string(),
            });
        }
    }
    if docker_tools.is_empty() {
        return Vec::new();
    }

    let needs_litex = docker_tools.iter().any(|tool| tool == "litex");
    let command = if needs_litex {
        CommandSpec::new("docker")
            .args([
                "build",
                "--build-arg",
                "AF_INSTALL_LITEX=true",
                "-t",
                "accelfury-af:oss-litex",
                ".",
            ])
            .allow_network(true)
    } else {
        CommandSpec::new("make")
            .arg("docker-build")
            .allow_network(true)
    };

    vec![ToolingInstallAction {
        id: "docker-runtime".to_string(),
        provider: "docker".to_string(),
        tools: unique_sorted(docker_tools),
        commands: vec![command],
        writes_to: vec![
            "Docker image accelfury-af:oss or accelfury-af:oss-litex".to_string(),
            "Docker layer cache outside the repository".to_string(),
        ],
        requires_network: true,
        requires_sudo: false,
        executable: true,
        reason: format!(
            "Install heavy open-source EDA tooling in a container instead of writing host OS packages; repository tool root remains `{}`.",
            selection.tool_root.display()
        ),
    }]
}

fn plan_user_actions(
    selection: &ToolingSelection,
    missing: &[&ToolingToolStatus],
    definitions_by_id: &BTreeMap<&str, ToolDefinition>,
    blockers: &mut Vec<ToolingBlocker>,
) -> Vec<ToolingInstallAction> {
    let mut python_packages = Vec::new();
    for tool in missing {
        let definition = definitions_by_id
            .get(tool.tool.as_str())
            .expect("selected ids are validated before planning");
        if let Some(package) = definition.user_python_package {
            python_packages.push(package.to_string());
        } else if !definition.vendor_manual {
            blockers.push(ToolingBlocker {
                code: "AF_TOOLING_USER_INSTALL_UNSUPPORTED".to_string(),
                tool: Some(tool.tool.clone()),
                message: format!(
                    "`{}` has no safe af-managed user-local installer",
                    tool.tool
                ),
                hint:
                    "Use --install-mode docker for EDA tools, or install the host package manually."
                        .to_string(),
            });
        }
    }
    if python_packages.is_empty() {
        return Vec::new();
    }

    let venv = selection.tool_root.join("python");
    let python = venv.join("bin/python");
    let mut pip_args = vec![
        "-m".to_string(),
        "pip".to_string(),
        "install".to_string(),
        "--upgrade".to_string(),
    ];
    pip_args.extend(unique_sorted(python_packages));

    vec![ToolingInstallAction {
        id: "af-managed-python-venv".to_string(),
        provider: "python-venv".to_string(),
        tools: missing
            .iter()
            .filter(|tool| {
                definitions_by_id
                    .get(tool.tool.as_str())
                    .is_some_and(|definition| definition.user_python_package.is_some())
            })
            .map(|tool| tool.tool.clone())
            .collect(),
        commands: vec![
            CommandSpec::new("python3").args([
                "-m".to_string(),
                "venv".to_string(),
                venv.display().to_string(),
            ]),
            CommandSpec::new(python.display().to_string())
                .args(pip_args)
                .allow_network(true),
        ],
        writes_to: vec![venv.display().to_string()],
        requires_network: true,
        requires_sudo: false,
        executable: true,
        reason: "Install Python package tooling into an af-managed venv instead of global pip."
            .to_string(),
    }]
}

fn plan_system_actions(
    _selection: &ToolingSelection,
    missing: &[&ToolingToolStatus],
    definitions_by_id: &BTreeMap<&str, ToolDefinition>,
    _effective_allow_network: bool,
    allow_system: bool,
    blockers: &mut Vec<ToolingBlocker>,
) -> Vec<ToolingInstallAction> {
    let mut packages = Vec::new();
    for tool in missing {
        let definition = definitions_by_id
            .get(tool.tool.as_str())
            .expect("selected ids are validated before planning");
        if let Some(package) = definition.system_package {
            packages.push(package.to_string());
        } else if !definition.vendor_manual {
            blockers.push(ToolingBlocker {
                code: "AF_TOOLING_SYSTEM_INSTALL_UNSUPPORTED".to_string(),
                tool: Some(tool.tool.clone()),
                message: format!("`{}` has no portable system package action", tool.tool),
                hint: "Use --install-mode docker or --install-mode user where supported, or install this tool manually."
                    .to_string(),
            });
        }
    }
    if packages.is_empty() {
        return Vec::new();
    }

    let mut install_args = vec![
        "apt-get".to_string(),
        "install".to_string(),
        "-y".to_string(),
        "--no-install-recommends".to_string(),
    ];
    install_args.extend(unique_sorted(packages));

    vec![ToolingInstallAction {
        id: "apt-system-packages".to_string(),
        provider: "apt".to_string(),
        tools: missing
            .iter()
            .filter(|tool| {
                definitions_by_id
                    .get(tool.tool.as_str())
                    .is_some_and(|definition| definition.system_package.is_some())
            })
            .map(|tool| tool.tool.clone())
            .collect(),
        commands: vec![
            CommandSpec::new("sudo")
                .args(["apt-get".to_string(), "update".to_string()])
                .allow_network(true),
            CommandSpec::new("sudo")
                .args(install_args)
                .allow_network(true),
        ],
        writes_to: vec!["host OS package database".to_string()],
        requires_network: true,
        requires_sudo: true,
        executable: allow_system,
        reason:
            "Install host OS packages only after explicit --install-mode system and --allow-system."
                .to_string(),
    }]
}

fn unique_sorted(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn human_summary(prefix: &str, report: &ToolingReport) -> String {
    let missing = report
        .tools
        .iter()
        .filter(|tool| !tool.available)
        .map(|tool| tool.tool.as_str())
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return format!("{prefix} passed: selected tools are available");
    }
    format!(
        "{prefix} {}: missing {}; planned actions: {}; blockers: {}",
        report.status,
        missing.join(", "),
        report.planned_actions.len(),
        report.blockers.len()
    )
}

fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            id: "iverilog",
            manifest_key: Some("iverilog"),
            probe: ProbeKind::Command {
                program: "iverilog",
                args: &["-V"],
            },
            capability:
                "Icarus Verilog compile step for generated or user-provided simulation flows",
            docker_runtime: true,
            user_python_package: None,
            system_package: Some("iverilog"),
            vendor_manual: false,
        },
        ToolDefinition {
            id: "vvp",
            manifest_key: Some("vvp"),
            probe: ProbeKind::Command {
                program: "vvp",
                args: &["-V"],
            },
            capability: "Icarus Verilog runtime for vvp simulation execution",
            docker_runtime: true,
            user_python_package: None,
            system_package: Some("iverilog"),
            vendor_manual: false,
        },
        ToolDefinition {
            id: "verilator",
            manifest_key: Some("verilator"),
            probe: ProbeKind::Command {
                program: "verilator",
                args: &["--version"],
            },
            capability: "af core lint/sim --backend verilator",
            docker_runtime: true,
            user_python_package: None,
            system_package: Some("verilator"),
            vendor_manual: false,
        },
        ToolDefinition {
            id: "yosys",
            manifest_key: Some("yosys"),
            probe: ProbeKind::Command {
                program: "yosys",
                args: &["-V"],
            },
            capability: "af build/core lint syntax smoke with --backend yosys",
            docker_runtime: true,
            user_python_package: None,
            system_package: Some("yosys"),
            vendor_manual: false,
        },
        ToolDefinition {
            id: "nextpnr-ice40",
            manifest_key: Some("nextpnr_ice40"),
            probe: ProbeKind::Command {
                program: "nextpnr-ice40",
                args: &["--version"],
            },
            capability: "iCE40 place-and-route planning and report capture",
            docker_runtime: true,
            user_python_package: None,
            system_package: Some("nextpnr-ice40"),
            vendor_manual: false,
        },
        ToolDefinition {
            id: "nextpnr-ecp5",
            manifest_key: Some("nextpnr_ecp5"),
            probe: ProbeKind::Command {
                program: "nextpnr-ecp5",
                args: &["--version"],
            },
            capability: "ECP5 place-and-route planning and report capture",
            docker_runtime: true,
            user_python_package: None,
            system_package: Some("nextpnr-ecp5"),
            vendor_manual: false,
        },
        ToolDefinition {
            id: "nextpnr-gowin",
            manifest_key: Some("nextpnr_gowin"),
            probe: ProbeKind::Command {
                program: "nextpnr-gowin",
                args: &["--version"],
            },
            capability: "Gowin place-and-route planning and report capture",
            docker_runtime: true,
            user_python_package: None,
            system_package: Some("nextpnr-gowin"),
            vendor_manual: false,
        },
        ToolDefinition {
            id: "fusesoc",
            manifest_key: Some("fusesoc"),
            probe: ProbeKind::Command {
                program: "fusesoc",
                args: &["--version"],
            },
            capability: "FuseSoC package/integration checks",
            docker_runtime: true,
            user_python_package: Some("fusesoc"),
            system_package: None,
            vendor_manual: false,
        },
        ToolDefinition {
            id: "edalize",
            manifest_key: Some("edalize"),
            probe: ProbeKind::PythonModule { module: "edalize" },
            capability: "Edalize Python backend API used by FuseSoC/export integration flows",
            docker_runtime: true,
            user_python_package: Some("edalize"),
            system_package: None,
            vendor_manual: false,
        },
        ToolDefinition {
            id: "xmllint",
            manifest_key: Some("xmllint"),
            probe: ProbeKind::Command {
                program: "xmllint",
                args: &["--version"],
            },
            capability: "XML schema/package validation helper for IP-XACT and metadata checks",
            docker_runtime: true,
            user_python_package: None,
            system_package: Some("libxml2-utils"),
            vendor_manual: false,
        },
        ToolDefinition {
            id: "sby",
            manifest_key: Some("sby"),
            probe: ProbeKind::Command {
                program: "sby",
                args: &["--version"],
            },
            capability: "formal verification through SymbiYosys",
            docker_runtime: true,
            user_python_package: None,
            system_package: None,
            vendor_manual: false,
        },
        ToolDefinition {
            id: "boolector",
            manifest_key: Some("boolector"),
            probe: ProbeKind::Command {
                program: "boolector",
                args: &["--version"],
            },
            capability: "SMT solver for yosys-smtbmc and SymbiYosys bit-vector proofs",
            docker_runtime: true,
            user_python_package: None,
            system_package: Some("boolector"),
            vendor_manual: false,
        },
        ToolDefinition {
            id: "z3",
            manifest_key: Some("z3"),
            probe: ProbeKind::Command {
                program: "z3",
                args: &["--version"],
            },
            capability: "SMT solver for yosys-smtbmc and general SMT-LIB checks",
            docker_runtime: true,
            user_python_package: None,
            system_package: Some("z3"),
            vendor_manual: false,
        },
        ToolDefinition {
            id: "yices-smt2",
            manifest_key: Some("yices_smt2"),
            probe: ProbeKind::Command {
                program: "yices-smt2",
                args: &["--version"],
            },
            capability: "Yices SMT-LIB solver for yosys-smtbmc formal runs",
            docker_runtime: true,
            user_python_package: None,
            system_package: None,
            vendor_manual: false,
        },
        ToolDefinition {
            id: "bitwuzla",
            manifest_key: Some("bitwuzla"),
            probe: ProbeKind::Command {
                program: "bitwuzla",
                args: &["--version"],
            },
            capability: "Bitwuzla SMT solver for bit-vector and array proofs",
            docker_runtime: true,
            user_python_package: None,
            system_package: Some("bitwuzla"),
            vendor_manual: false,
        },
        ToolDefinition {
            id: "cvc5",
            manifest_key: Some("cvc5"),
            probe: ProbeKind::Command {
                program: "cvc5",
                args: &["--version"],
            },
            capability: "cvc5 SMT solver for SMT-LIB compatibility and formal cross-checks",
            docker_runtime: true,
            user_python_package: None,
            system_package: Some("cvc5"),
            vendor_manual: false,
        },
        ToolDefinition {
            id: "deno",
            manifest_key: None,
            probe: ProbeKind::Command {
                program: "deno",
                args: &["--version"],
            },
            capability: "repo audit flow and TypeScript validation tasks",
            docker_runtime: false,
            user_python_package: None,
            system_package: None,
            vendor_manual: false,
        },
        ToolDefinition {
            id: "deno-audit-repo",
            manifest_key: None,
            probe: ProbeKind::DenoAuditReadiness,
            capability: "deno task audit:repo readiness",
            docker_runtime: false,
            user_python_package: None,
            system_package: None,
            vendor_manual: false,
        },
        ToolDefinition {
            id: "litex",
            manifest_key: Some("litex"),
            probe: ProbeKind::PythonModule { module: "litex" },
            capability: "LiteX wrapper/build surfaces beyond Rust skeleton generation",
            docker_runtime: true,
            user_python_package: Some("litex"),
            system_package: None,
            vendor_manual: false,
        },
        ToolDefinition {
            id: "openfpgaloader",
            manifest_key: Some("openfpgaloader"),
            probe: ProbeKind::Command {
                program: "openFPGALoader",
                args: &["--Version"],
            },
            capability: "open-source FPGA programming through openFPGALoader",
            docker_runtime: true,
            user_python_package: None,
            system_package: Some("openfpgaloader"),
            vendor_manual: false,
        },
        ToolDefinition {
            id: "gw_sh",
            manifest_key: Some("gowin"),
            probe: ProbeKind::Command {
                program: "gw_sh",
                args: &["--version"],
            },
            capability: "Gowin vendor build shell",
            docker_runtime: false,
            user_python_package: None,
            system_package: None,
            vendor_manual: true,
        },
        ToolDefinition {
            id: "programmer_cli",
            manifest_key: Some("gowin"),
            probe: ProbeKind::Command {
                program: "programmer_cli",
                args: &["--version"],
            },
            capability: "Gowin vendor programmer CLI",
            docker_runtime: false,
            user_python_package: None,
            system_package: None,
            vendor_manual: true,
        },
    ]
}
