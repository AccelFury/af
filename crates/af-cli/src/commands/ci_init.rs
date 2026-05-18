// SPDX-License-Identifier: Apache-2.0

use crate::ci::{
    artifact_paths, detect_profile, render_docs, render_pr_template, render_workflow, scan_repo,
    to_toml_string, CiConfig, CiPathsConfig, ConfigBuilder, RepoScan,
};
use crate::{CliError, CliOutput};
use clap::Args;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct CiInitArgs {
    #[arg(long, default_value = ".")]
    pub repo: PathBuf,
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub hdl: String,
    #[arg(long)]
    pub rtl: String,
    #[arg(long)]
    pub top: Option<String>,
    #[arg(long)]
    pub sim: Option<String>,
    #[arg(long, default_value = "github")]
    pub provider: String,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug)]
struct InitOutputs {
    workflow: String,
    pr_template: String,
    docs: String,
    config: String,
    report: String,
}

pub fn run(args: &CiInitArgs) -> Result<CliOutput, CliError> {
    let repo_root = &args.repo;
    let paths = CiPathsConfig {
        rtl: vec![args.rtl.clone()],
        tb: vec!["tb".to_string()],
        sim: vec!["sim".to_string()],
        formal: vec!["formal".to_string()],
        boards: vec!["boards".to_string()],
    };

    let scan = scan_repo(repo_root, &paths);
    let top = determine_top(scan.top_candidates(), args.top.as_deref())?;
    let profile = detect_profile(&scan, &args.hdl);
    let config = ConfigBuilder {
        project: args.project.clone(),
        hdl: args.hdl.clone(),
        rtl_path: args.rtl.clone(),
        top: Some(top),
        ci_provider: args.provider.clone(),
        sim_command: args.sim.clone(),
        make_test_detected: args.sim.as_ref().is_some() || scan.has_make_test_target,
    }
    .build();

    let rendered = render_all(&config, &scan, profile);
    let outputs = write_init_outputs(repo_root, &config, &rendered, args.dry_run)?;
    let report = {
        let mut report = crate::ci::CiDiagnosticReport::pass(&config.project.name, &config, &scan);
        report.generated_files = vec![
            outputs.workflow.clone(),
            outputs.pr_template.clone(),
            outputs.docs.clone(),
            outputs.config.clone(),
            outputs.report.clone(),
        ];
        report.next_actions.push(
            "add board profiles with af ci add-board when constraints become available".to_string(),
        );
        report
    };

    Ok(CliOutput {
        human: if args.dry_run {
            "af ci init: dry-run completed".to_string()
        } else {
            format!("af ci init completed: {}", outputs.config)
        },
        json: json!({
            "status": report.status,
            "project": config.project.name,
            "report": report,
        }),
    })
}

fn determine_top(candidates: &[String], provided: Option<&str>) -> Result<String, CliError> {
    if let Some(top) = provided {
        if top.trim().is_empty() {
            return Err(CliError::new(
                "AF_CI_INIT_EMPTY_TOP",
                "--top is empty",
                "Pass the real top module name.",
                2,
            ));
        }
        return Ok(top.to_string());
    }

    if candidates.len() == 1 {
        return Ok(candidates[0].clone());
    }
    if candidates.is_empty() {
        return Err(CliError::new(
            "AF_CI_INIT_TOP_MISSING",
            "Top module not detected from .v/.sv files",
            "Pass --top explicitly or add exactly one module in the selected rtl directory.",
            2,
        ));
    }

    let mut message = String::from("Multiple top candidates found:\n");
    for candidate in candidates {
        message.push_str(" - ");
        message.push_str(candidate);
        message.push('\n');
    }
    message.push_str("Pass --top <module_name>.");
    Err(CliError::new(
        "AF_CI_INIT_TOP_AMBIGUOUS",
        "Could not infer top module uniquely.",
        &message,
        2,
    ))
}

fn render_all(
    config: &CiConfig,
    scan: &RepoScan,
    profile: crate::ci::ProjectProfile,
) -> RenderedInitFiles {
    RenderedInitFiles {
        workflow: render_workflow(config, scan, profile),
        pr_template: render_pr_template(profile),
        docs: render_docs(config, scan, profile),
        config_toml: to_toml_string(config).unwrap_or_default(),
        report: serde_json::to_string_pretty(&crate::ci::CiDiagnosticReport::pass(
            &config.project.name,
            config,
            scan,
        ))
        .unwrap_or_else(|_| "{}".to_string()),
    }
}

fn write_init_outputs(
    repo_root: &Path,
    config: &CiConfig,
    rendered: &RenderedInitFiles,
    dry_run: bool,
) -> Result<InitOutputs, CliError> {
    let output = InitOutputs {
        workflow: ".github/workflows/hdl-ci.yml".to_string(),
        pr_template: ".github/PULL_REQUEST_TEMPLATE.md".to_string(),
        docs: "docs/ci.md".to_string(),
        config: "af-ci.toml".to_string(),
        report: "artifacts/openfpga-ci/reports/af-ci-init-report.json".to_string(),
    };
    if dry_run {
        return Ok(InitOutputs {
            workflow: repo_root.join(&output.workflow).display().to_string(),
            pr_template: repo_root.join(&output.pr_template).display().to_string(),
            docs: repo_root.join(&output.docs).display().to_string(),
            config: repo_root.join(&output.config).display().to_string(),
            report: repo_root.join(&output.report).display().to_string(),
        });
    }

    let workflow_path = repo_root.join(&output.workflow);
    let pr_path = repo_root.join(&output.pr_template);
    let docs_path = repo_root.join(&output.docs);
    let config_path = repo_root.join(&output.config);
    let report_path = repo_root.join(&output.report);
    let scripts_path = repo_root.join("scripts/ci/prepare_paths.sh");

    for dir in [
        workflow_path.parent().unwrap_or(Path::new(".")),
        pr_path.parent().unwrap_or(Path::new(".")),
        docs_path.parent().unwrap_or(Path::new(".")),
        config_path.parent().unwrap_or(Path::new(".")),
        report_path.parent().unwrap_or(Path::new(".")),
        scripts_path.parent().unwrap_or(Path::new(".")),
    ] {
        fs::create_dir_all(dir).map_err(|err| {
            CliError::new(
                "AF_CI_INIT_DIR",
                format!("failed to create `{}`: {err}", dir.display()),
                "Check filesystem permissions.",
                1,
            )
        })?;
    }

    write_file(&workflow_path, &rendered.workflow)?;
    write_file(&pr_path, &rendered.pr_template)?;
    write_file(&docs_path, &rendered.docs)?;
    if !config_path.exists() {
        write_file(&config_path, &rendered.config_toml)?;
    }
    write_file(&report_path, &rendered.report)?;
    if !scripts_path.exists() {
        write_file(
            &scripts_path,
            "#!/usr/bin/env sh\n# Generated by AccelFury IP Toolchain\nset -euo pipefail\n\nmkdir -p artifacts/openfpga-ci\n",
        )?;
    }

    if !artifact_paths(config).is_empty() {
        let artifact_list = artifact_paths(config).join("\n");
        let stamped = format!("# Generated by AccelFury IP Toolchain\n{artifact_list}");
        write_file(
            &report_path.with_file_name("af-ci-init-report.txt"),
            &stamped,
        )?;
    }

    Ok(InitOutputs {
        workflow: workflow_path.display().to_string(),
        pr_template: pr_path.display().to_string(),
        docs: docs_path.display().to_string(),
        config: config_path.display().to_string(),
        report: report_path.display().to_string(),
    })
}

fn write_file(path: &Path, content: &str) -> Result<(), CliError> {
    fs::write(path, content).map_err(|err| {
        CliError::new(
            "AF_CI_INIT_WRITE",
            format!("cannot write `{}`: {err}", path.display()),
            "Check directory permissions.",
            5,
        )
    })
}

struct RenderedInitFiles {
    workflow: String,
    pr_template: String,
    docs: String,
    config_toml: String,
    report: String,
}
