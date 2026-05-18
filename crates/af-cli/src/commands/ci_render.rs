// SPDX-License-Identifier: Apache-2.0

use crate::ci::{detect_profile, from_toml_file, render_workflow, scan_repo};
use crate::{CliError, CliOutput};
use clap::Args;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct CiRenderArgs {
    #[arg(long, default_value = "af-ci.toml")]
    pub config: PathBuf,
    #[arg(long, default_value = ".github/workflows/hdl-ci.yml")]
    pub output: PathBuf,
    #[arg(long)]
    pub dry_run: bool,
}

pub fn run(args: &CiRenderArgs) -> Result<CliOutput, CliError> {
    let config = from_toml_file(&args.config).map_err(|err| {
        CliError::new(
            "AF_CI_RENDER_CONFIG",
            format!("cannot read {}: {err}", args.config.display()),
            "Pass an existing af-ci.toml file.",
            3,
        )
    })?;

    let repo_root = args
        .config
        .parent()
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let scan = scan_repo(&repo_root, &config.paths);
    let profile = detect_profile(&scan, &config.project.hdl);
    let rendered = render_workflow(&config, &scan, profile);

    if !args.dry_run {
        if let Some(parent) = args.output.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                CliError::new(
                    "AF_CI_RENDER_DIR",
                    format!("cannot create `{}`: {err}", parent.display()),
                    "Check output path permissions.",
                    5,
                )
            })?;
        }
        fs::write(&args.output, rendered.as_bytes()).map_err(|err| {
            CliError::new(
                "AF_CI_RENDER_WRITE",
                format!("cannot write `{}`: {err}", args.output.display()),
                "Use --output with a writable path.",
                5,
            )
        })?;
    }

    Ok(CliOutput {
        human: if args.dry_run {
            format!(
                "af ci render: dry-run completed to {}",
                args.output.display()
            )
        } else {
            format!("af ci render: wrote {}", args.output.display())
        },
        json: json!({
            "status": if args.dry_run { "pass" } else { "passed" },
            "schema_version": "1.0",
            "generated": args.output.display().to_string(),
            "dry_run": args.dry_run,
            "size_bytes": rendered.len(),
            "profile": profile.as_str(),
        }),
    })
}
