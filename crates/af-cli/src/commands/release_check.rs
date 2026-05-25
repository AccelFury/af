// SPDX-License-Identifier: Apache-2.0

use af_backend::{CommandRecord, CommandRunner, CommandSpec, ProcessCommandRunner};
use af_report::{
    AfReport, CiEvidenceRecord, CommandPayload, ReleaseGate, ReleaseGateSummary, ReleasePayload,
};
use clap::Args;
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{to_pretty_json, CliError, CliOutput};

#[derive(Args, Debug)]
pub struct ReleaseCheckArgs {
    #[arg(long, default_value = ".")]
    pub repo: PathBuf,
    #[arg(long)]
    pub tag: Option<String>,
    #[arg(long)]
    pub ci_evidence: Option<PathBuf>,
    #[arg(long)]
    pub artifact_dir: Option<PathBuf>,
    #[arg(long)]
    pub docker_evidence: Option<PathBuf>,
    #[arg(long)]
    pub output: Option<PathBuf>,
    #[arg(long)]
    pub skip_local_checks: bool,
    #[arg(long)]
    pub run_docker_smoke: bool,
    #[arg(long)]
    pub allow_blocked: bool,
}

#[derive(Debug, Deserialize)]
struct DockerEvidence {
    image: String,
    digest: String,
    #[serde(default)]
    smoke_report: Option<String>,
    #[serde(default)]
    smoke_sha256sums: Option<String>,
}

#[derive(Debug)]
struct GateAccumulator {
    gates: Vec<ReleaseGate>,
    commands: Vec<CommandRecord>,
    limitations: Vec<String>,
}

impl GateAccumulator {
    fn new() -> Self {
        Self {
            gates: Vec::new(),
            commands: Vec::new(),
            limitations: Vec::new(),
        }
    }

    fn push(&mut self, gate: ReleaseGate) {
        self.limitations.extend(gate.limitations.iter().cloned());
        self.gates.push(gate);
    }
}

pub fn run(args: &ReleaseCheckArgs, build_root: &Path) -> Result<CliOutput, CliError> {
    let repo = normalize_repo(&args.repo);
    let target_version = env!("CARGO_PKG_VERSION").to_string();
    let target_tag = args
        .tag
        .clone()
        .unwrap_or_else(|| format!("v{target_version}"));
    let output = args
        .output
        .clone()
        .unwrap_or_else(|| build_root.join("release").join("release-readiness.json"));
    let ci_evidence = args
        .ci_evidence
        .clone()
        .unwrap_or_else(|| build_root.join("release").join("ci-evidence.json"));
    let artifact_dir = args
        .artifact_dir
        .clone()
        .unwrap_or_else(|| build_root.join("release").join("artifacts"));
    let docker_evidence = args
        .docker_evidence
        .clone()
        .unwrap_or_else(|| build_root.join("release").join("docker-image.json"));

    let runner = ProcessCommandRunner;
    let mut acc = GateAccumulator::new();
    let commit_sha = current_commit(&runner, &repo, &mut acc.commands);
    check_clean_worktree(&runner, &repo, &mut acc);

    if args.skip_local_checks {
        acc.push(blocked_gate(
            "local-quality-gates",
            "local fmt/clippy/test/contract/self-check execution was skipped",
            vec!["Run without --skip-local-checks for production release gating.".to_string()],
        ));
    } else {
        run_local_quality_gates(&runner, &repo, build_root, &mut acc);
    }

    check_ci_evidence(&ci_evidence, &commit_sha, &mut acc);
    check_release_artifacts(&runner, &artifact_dir, &target_tag, &mut acc);
    check_docker_evidence(
        &runner,
        &repo,
        build_root,
        &docker_evidence,
        args.run_docker_smoke,
        &mut acc,
    );
    check_claims(&repo, &mut acc);

    let passed = acc
        .gates
        .iter()
        .filter(|gate| gate.status == "passed")
        .count();
    let blocked = acc
        .gates
        .iter()
        .filter(|gate| gate.status == "blocked")
        .count();
    let status = if blocked == 0 { "passed" } else { "blocked" };
    let payload = ReleasePayload {
        target_version,
        target_tag,
        commit_sha,
        readiness_path: output.display().to_string(),
        gate_summary: ReleaseGateSummary {
            total: acc.gates.len(),
            passed,
            blocked,
        },
        gates: acc.gates,
    };

    let mut report = AfReport::new(status);
    report.commands = acc.commands;
    report.artifacts.push(output.display().to_string());
    report.limitations = acc.limitations;
    report.command_payload = Some(CommandPayload::Release(payload));
    report.reproducibility = Some(af_report::Reproducibility::capture(&report.tool_versions));
    write_json_creating_parent(&output, &report)?;

    if status == "blocked" && !args.allow_blocked {
        return Err(CliError::new(
            "AF_RELEASE_READINESS_BLOCKED",
            "release readiness gate is blocked",
            format!(
                "Open `{}` and fix every gate with status `blocked`, then rerun `af release check --json`.",
                output.display()
            ),
            2,
        )
        .with_details(&json!({
            "release_readiness": output,
            "report": report,
        })));
    }

    Ok(CliOutput {
        human: format!("release readiness {status}: {}", output.display()),
        json: serde_json::to_value(report).map_err(|err| {
            CliError::new(
                "AF_JSON_SERIALIZE_FAILED",
                err.to_string(),
                "Report this bug with the release readiness inputs.",
                1,
            )
        })?,
    })
}

fn normalize_repo(repo: &Path) -> PathBuf {
    if repo.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        repo.to_path_buf()
    }
}

fn current_commit(
    runner: &impl CommandRunner,
    repo: &Path,
    commands: &mut Vec<CommandRecord>,
) -> String {
    let output = run_command(
        runner,
        CommandSpec::new("git")
            .args(["rev-parse", "HEAD"])
            .cwd(repo.to_path_buf()),
        commands,
    );
    output
        .and_then(|record| {
            if record.exit_code == Some(0) {
                Some(record.stdout.trim().to_string())
            } else {
                None
            }
        })
        .filter(|sha| !sha.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn check_clean_worktree(runner: &impl CommandRunner, repo: &Path, acc: &mut GateAccumulator) {
    match run_command(
        runner,
        CommandSpec::new("git")
            .args(["status", "--porcelain"])
            .cwd(repo.to_path_buf()),
        &mut acc.commands,
    ) {
        Some(record) if record.exit_code == Some(0) && record.stdout.trim().is_empty() => {
            acc.push(passed_gate(
                "source-tree-clean",
                "working tree is clean; release evidence maps to an exact commit",
                vec![],
            ));
        }
        Some(record) if record.exit_code == Some(0) => {
            let dirty = record
                .stdout
                .lines()
                .take(20)
                .map(str::to_string)
                .collect::<Vec<_>>();
            acc.push(blocked_gate(
                "source-tree-clean",
                "working tree is dirty; release evidence would not map to an exact commit",
                dirty,
            ));
        }
        Some(record) => acc.push(blocked_gate(
            "source-tree-clean",
            "could not verify working tree cleanliness",
            vec![format!(
                "`git status --porcelain` exited with {:?}",
                record.exit_code
            )],
        )),
        None => acc.push(blocked_gate(
            "source-tree-clean",
            "could not verify working tree cleanliness",
            vec!["`git status --porcelain` could not be executed".to_string()],
        )),
    }
}

fn run_local_quality_gates(
    runner: &impl CommandRunner,
    repo: &Path,
    build_root: &Path,
    acc: &mut GateAccumulator,
) {
    let fmt = run_required_command(
        runner,
        "fmt",
        CommandSpec::new("cargo")
            .args(["fmt", "--all", "--", "--check"])
            .cwd(repo.to_path_buf()),
        acc,
    );
    let clippy = run_required_command(
        runner,
        "clippy",
        CommandSpec::new("cargo")
            .args([
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ])
            .cwd(repo.to_path_buf()),
        acc,
    );
    let tests = run_required_command(
        runner,
        "cargo-test",
        CommandSpec::new("cargo")
            .args(["test", "--workspace"])
            .cwd(repo.to_path_buf()),
        acc,
    );
    let contract = run_required_command(
        runner,
        "contract-guard",
        CommandSpec::new("bash")
            .args([".claude/skills/af-cli-contract-guard/check.sh"])
            .cwd(repo.to_path_buf()),
        acc,
    );
    let self_check = run_required_command(
        runner,
        "self-check",
        CommandSpec::new("cargo")
            .args([
                "run".to_string(),
                "--quiet".to_string(),
                "-p".to_string(),
                "af-cli".to_string(),
                "--bin".to_string(),
                "af".to_string(),
                "--".to_string(),
                "--build-root".to_string(),
                build_root.display().to_string(),
                "self".to_string(),
                "check".to_string(),
                "--json".to_string(),
            ])
            .cwd(repo.to_path_buf()),
        acc,
    );

    let all_passed = [fmt, clippy, tests, contract, self_check]
        .iter()
        .all(|passed| *passed);
    if all_passed {
        acc.push(passed_gate(
            "local-quality-gates",
            "fmt, clippy, cargo test, contract guard, and self check passed",
            vec![],
        ));
    } else {
        acc.push(blocked_gate(
            "local-quality-gates",
            "one or more local quality gates failed",
            vec![
                "Inspect command records in release-readiness.json and rerun the failing command locally.".to_string(),
            ],
        ));
    }
}

fn run_required_command(
    runner: &impl CommandRunner,
    id: &str,
    spec: CommandSpec,
    acc: &mut GateAccumulator,
) -> bool {
    match run_command(runner, spec, &mut acc.commands) {
        Some(record) if record.exit_code == Some(0) => true,
        Some(record) => {
            acc.limitations
                .push(format!("`{id}` command exited with {:?}", record.exit_code));
            false
        }
        None => {
            acc.limitations
                .push(format!("`{id}` command could not be executed"));
            false
        }
    }
}

fn check_ci_evidence(path: &Path, commit_sha: &str, acc: &mut GateAccumulator) {
    let mut limitations = Vec::new();
    let mut evidence = vec![path.display().to_string()];
    match read_json::<CiEvidenceRecord>(path) {
        Ok(record) => {
            if record.commit_sha != commit_sha {
                limitations.push(format!(
                    "CI evidence commit `{}` does not match current commit `{commit_sha}`",
                    record.commit_sha
                ));
            }
            if record.conclusion != "success" {
                limitations.push(format!(
                    "CI evidence conclusion is `{}`, expected `success`",
                    record.conclusion
                ));
            }
            match &record.workflow_run_url {
                Some(url) if !url.trim().is_empty() => evidence.push(url.clone()),
                _ => limitations.push("CI evidence is missing workflow_run_url".to_string()),
            }
            match &record.artifact_bundle {
                Some(bundle) if !bundle.trim().is_empty() => evidence.push(bundle.clone()),
                _ => limitations.push("CI evidence is missing artifact_bundle".to_string()),
            }
            match &record.sha256sums {
                Some(sums) if !sums.trim().is_empty() => evidence.push(sums.clone()),
                _ => limitations.push("CI evidence is missing sha256sums".to_string()),
            }
        }
        Err(message) => limitations.push(message),
    }

    if limitations.is_empty() {
        acc.push(passed_gate(
            "external-ci-evidence",
            "external CI evidence matches this commit and succeeded",
            evidence,
        ));
    } else {
        acc.push(blocked_gate(
            "external-ci-evidence",
            "external CI evidence is missing, stale, or unsuccessful",
            limitations,
        ));
    }
}

fn check_release_artifacts(
    runner: &impl CommandRunner,
    artifact_dir: &Path,
    tag: &str,
    acc: &mut GateAccumulator,
) {
    let sums = artifact_dir.join("SHA256SUMS");
    let expected_binary = artifact_dir.join(format!("af-{tag}-x86_64-unknown-linux-gnu.tar.gz"));
    let mut limitations = Vec::new();
    let mut evidence = vec![artifact_dir.display().to_string()];

    if !artifact_dir.is_dir() {
        limitations.push(format!(
            "release artifact directory `{}` is missing",
            artifact_dir.display()
        ));
    }
    if !expected_binary.is_file() {
        limitations.push(format!(
            "expected Linux x86_64 binary bundle `{}` is missing",
            expected_binary.display()
        ));
    } else {
        evidence.push(expected_binary.display().to_string());
    }
    if !sums.is_file() {
        limitations.push(format!("`{}` is missing", sums.display()));
    } else {
        evidence.push(sums.display().to_string());
        match run_command(
            runner,
            CommandSpec::new("sha256sum")
                .args(["-c", "SHA256SUMS"])
                .cwd(artifact_dir.to_path_buf()),
            &mut acc.commands,
        ) {
            Some(record) if record.exit_code == Some(0) => {}
            Some(record) => limitations.push(format!(
                "`sha256sum -c SHA256SUMS` failed with {:?}",
                record.exit_code
            )),
            None => limitations.push("`sha256sum` could not be executed".to_string()),
        }
    }

    if limitations.is_empty() {
        acc.push(passed_gate(
            "release-artifacts",
            "release binary bundle and SHA256SUMS are present and verified",
            evidence,
        ));
    } else {
        acc.push(blocked_gate(
            "release-artifacts",
            "release artifacts are incomplete or checksums do not verify",
            limitations,
        ));
    }
}

fn check_docker_evidence(
    runner: &impl CommandRunner,
    repo: &Path,
    build_root: &Path,
    docker_evidence: &Path,
    run_docker_smoke: bool,
    acc: &mut GateAccumulator,
) {
    let mut limitations = Vec::new();
    let mut evidence = vec![docker_evidence.display().to_string()];

    if run_docker_smoke {
        match run_command(
            runner,
            CommandSpec::new("make")
                .args(["docker-smoke"])
                .env(
                    "AF_BUILD_ROOT",
                    build_root.join("docker-smoke").display().to_string(),
                )
                .cwd(repo.to_path_buf()),
            &mut acc.commands,
        ) {
            Some(record) if record.exit_code == Some(0) => {
                evidence.push(build_root.join("docker-smoke").display().to_string());
            }
            Some(record) => limitations.push(format!(
                "`make docker-smoke` failed with {:?}",
                record.exit_code
            )),
            None => limitations.push("`make docker-smoke` could not be executed".to_string()),
        }
    }

    match read_json::<DockerEvidence>(docker_evidence) {
        Ok(record) => {
            if record.image.trim().is_empty() {
                limitations.push("Docker evidence image is empty".to_string());
            } else {
                evidence.push(record.image);
            }
            if !record.digest.starts_with("sha256:") {
                limitations.push("Docker evidence digest must start with `sha256:`".to_string());
            } else {
                evidence.push(record.digest);
            }
            if let Some(report) = record.smoke_report {
                if !report.trim().is_empty() {
                    evidence.push(report);
                }
            }
            match record.smoke_sha256sums {
                Some(path) if !path.trim().is_empty() => {
                    let local_sums = resolve_local_reference(repo, &path);
                    if let Some(sums) = local_sums {
                        if !sums.is_file() {
                            limitations.push(format!(
                                "Docker smoke SHA256SUMS `{}` is missing",
                                sums.display()
                            ));
                        } else {
                            match run_command(
                                runner,
                                CommandSpec::new("sha256sum")
                                    .args(["-c".to_string(), sums.display().to_string()])
                                    .cwd(repo.to_path_buf()),
                                &mut acc.commands,
                            ) {
                                Some(check) if check.exit_code == Some(0) => {}
                                Some(check) => limitations.push(format!(
                                    "Docker smoke `sha256sum -c` failed with {:?}",
                                    check.exit_code
                                )),
                                None => limitations.push(
                                    "Docker smoke `sha256sum` could not be executed".to_string(),
                                ),
                            }
                        }
                    }
                    evidence.push(path);
                }
                _ => limitations.push("Docker evidence is missing smoke_sha256sums".to_string()),
            }
        }
        Err(message) => limitations.push(message),
    }

    if limitations.is_empty() {
        acc.push(passed_gate(
            "docker-image",
            "published Docker digest and smoke evidence are present",
            evidence,
        ));
    } else {
        acc.push(blocked_gate(
            "docker-image",
            "Docker image digest or smoke evidence is missing",
            limitations,
        ));
    }
}

fn check_claims(repo: &Path, acc: &mut GateAccumulator) {
    let mut offenders = Vec::new();
    for path in markdown_claim_paths(repo) {
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let lines: Vec<&str> = content.lines().collect();
        for (idx, line) in lines.iter().enumerate() {
            let lower = line.to_ascii_lowercase();
            let context = claim_context(&lines, idx);
            if contains_positive_claim_term(&lower) && !line_limits_claim(&context) {
                offenders.push(format!("{}:{}", path.display(), idx + 1));
            }
        }
    }

    if offenders.is_empty() {
        acc.push(passed_gate(
            "docs-claim-audit",
            "README/docs claim audit found no unsupported positive production claims",
            vec!["README.md".to_string(), "docs/**/*.md".to_string()],
        ));
    } else {
        acc.push(blocked_gate(
            "docs-claim-audit",
            "README/docs contain unsupported positive production claims",
            offenders,
        ));
    }
}

fn markdown_claim_paths(repo: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let readme = repo.join("README.md");
    if readme.is_file() {
        paths.push(readme);
    }
    collect_markdown(&repo.join("docs"), &mut paths);
    paths
}

fn collect_markdown(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_markdown(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            out.push(path);
        }
    }
}

fn contains_positive_claim_term(line: &str) -> bool {
    [
        "hardware-ready",
        "hardware ready",
        "hardware readiness",
        "timing-signoff",
        "timing signoff",
        "timing closure",
        "vendor-production",
        "vendor production",
        "vendor bitstream",
        "vendor implementation signoff",
        "security certification",
    ]
    .iter()
    .any(|term| line.contains(term))
}

fn line_limits_claim(line: &str) -> bool {
    [
        "does not",
        "do not",
        "not ",
        "without evidence",
        "without",
        "unsupported",
        "out of scope",
        "unless",
        "blocked",
        "missing",
        "staged",
        "not part",
        "not prove",
        "not imply",
        "remain",
        "claim boundary",
        "validate",
        "validation",
        "required",
        "non-goals",
        "avoids",
        "не ",
        "без",
    ]
    .iter()
    .any(|term| line.contains(term))
}

fn claim_context(lines: &[&str], idx: usize) -> String {
    let mut context = String::new();
    if idx > 0 {
        context.push_str(&lines[idx - 1].to_ascii_lowercase());
        context.push(' ');
    }
    context.push_str(&lines[idx].to_ascii_lowercase());
    if let Some(next) = lines.get(idx + 1) {
        context.push(' ');
        context.push_str(&next.to_ascii_lowercase());
    }
    context
}

fn run_command(
    runner: &impl CommandRunner,
    spec: CommandSpec,
    commands: &mut Vec<CommandRecord>,
) -> Option<CommandRecord> {
    match runner.run(&spec) {
        Ok(output) => {
            let record = CommandRecord::from(output);
            commands.push(record.clone());
            Some(record)
        }
        Err(err) => {
            commands.push(CommandRecord {
                program: spec.program,
                args: spec.args,
                cwd: spec.cwd,
                exit_code: None,
                stdout: String::new(),
                stderr: err.to_string(),
                env: spec.env,
                allow_network: spec.allow_network,
                timeout_seconds: spec.timeout_seconds,
                stdout_log: None,
                stderr_log: None,
            });
            None
        }
    }
}

fn passed_gate(id: &str, summary: &str, evidence: Vec<String>) -> ReleaseGate {
    ReleaseGate {
        id: id.to_string(),
        status: "passed".to_string(),
        required: true,
        summary: summary.to_string(),
        evidence,
        limitations: Vec::new(),
    }
}

fn blocked_gate(id: &str, summary: &str, limitations: Vec<String>) -> ReleaseGate {
    ReleaseGate {
        id: id.to_string(),
        status: "blocked".to_string(),
        required: true,
        summary: summary.to_string(),
        evidence: Vec::new(),
        limitations,
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("failed to read `{}`: {err}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|err| format!("failed to parse `{}` as JSON: {err}", path.display()))
}

fn resolve_local_reference(repo: &Path, reference: &str) -> Option<PathBuf> {
    if reference.starts_with("http://") || reference.starts_with("https://") {
        return None;
    }
    let path = PathBuf::from(reference);
    if path.is_absolute() {
        Some(path)
    } else {
        Some(repo.join(path))
    }
}

fn write_json_creating_parent(path: &Path, value: &AfReport) -> Result<(), CliError> {
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
    fs::write(path, format!("{}\n", to_pretty_json(value))).map_err(|err| {
        CliError::new(
            "AF_JSON_WRITE_FAILED",
            format!("failed to write `{}`: {err}", path.display()),
            "Check filesystem permissions and the selected output path.",
            5,
        )
    })
}
