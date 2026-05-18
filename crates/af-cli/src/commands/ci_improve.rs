// SPDX-License-Identifier: Apache-2.0

use crate::ci::{detect_profile, from_toml_file, merge_workflow, render_workflow, scan_repo};
use crate::{CliError, CliOutput};
use clap::Args;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct CiImproveArgs {
    #[arg(long, default_value = ".")]
    pub repo: PathBuf,
    #[arg(long, default_value = ".github/workflows/hdl-ci.yml")]
    pub workflow: PathBuf,
    #[arg(long)]
    pub allow_rewrite: bool,
    #[arg(long)]
    pub dry_run: bool,
}

pub fn run(args: &CiImproveArgs) -> Result<CliOutput, CliError> {
    let config = from_toml_file(&args.repo.join("af-ci.toml")).map_err(|err| {
        CliError::new(
            "AF_CI_IMPROVE_CONFIG",
            format!("cannot read af-ci.toml: {err}"),
            "Run af ci init first.",
            3,
        )
    })?;

    let scan = scan_repo(&args.repo, &config.paths);
    let profile = detect_profile(&scan, &config.project.hdl);
    let generated = render_workflow(&config, &scan, profile);
    let output_workflow = args.repo.join(&args.workflow);

    let existing = fs::read_to_string(&output_workflow).map_err(|err| {
        CliError::new(
            "AF_CI_IMPROVE_WORKFLOW",
            format!(
                "cannot read workflow `{}`: {err}",
                output_workflow.display()
            ),
            "Set --output to an existing path or run af ci init first.",
            3,
        )
    })?;

    let (merged, added, conflicts) = merge_workflow(&existing, &generated, args.allow_rewrite)
        .map_err(|err| {
            CliError::new(
                "AF_CI_IMPROVE_MERGE",
                format!("merge failed: {err}"),
                "Run af ci render into a new file and inspect diff manually.",
                3,
            )
        })?;

    if !args.dry_run {
        fs::write(&output_workflow, merged).map_err(|err| {
            CliError::new(
                "AF_CI_IMPROVE_WRITE",
                format!(
                    "cannot write workflow `{}`: {err}",
                    output_workflow.display()
                ),
                "Check write permissions and retry.",
                5,
            )
        })?;
    }

    let output = json!({
        "status": if conflicts.is_empty() { "passed" } else { "warning" },
        "added_jobs": added,
        "conflicts": conflicts,
        "workflow": output_workflow.display().to_string(),
        "allow_rewrite": args.allow_rewrite,
        "dry_run": args.dry_run,
    });

    Ok(CliOutput {
        human: "af ci improve completed".to_string(),
        json: output,
    })
}
