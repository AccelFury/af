// SPDX-License-Identifier: Apache-2.0
//
// `af wrapper generate` handler with the registry-aware board status
// warning.

use crate::{board_is_verified, CliError, CliOutput};
use af_wrapper_gen::{generate_wrapper, WrapperTarget};
use serde_json::json;
use std::path::{Path, PathBuf};

pub fn wrapper_generate(
    core_dir: &Path,
    build_root: &Path,
    target: &str,
    board: Option<&str>,
) -> Result<CliOutput, CliError> {
    let target = WrapperTarget::parse(target)?;
    let mut report = generate_wrapper(core_dir, build_root, target, board)?;
    if let Some(board_id) = board {
        append_board_status_warnings(&mut report.warnings, board_id);
    }
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

/// Append a `WrapperReport.warnings` entry for `board_id` when its registry
/// status is not `verified_on_hardware`. Best-effort: if the registry is
/// unreadable, emit a generic warning rather than failing the wrapper.
fn append_board_status_warnings(warnings: &mut Vec<String>, board_id: &str) {
    let registry_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    match af_board_db::list_boards(&registry_root) {
        Ok(boards) => match boards.iter().find(|b| b.board_id == board_id) {
            Some(entry) => {
                if !board_is_verified(&entry.exact_pinout_status) {
                    warnings.push(format!(
                        "Board `{board_id}` has `{}` pinout status; generated wrapper inherits placeholder pin/clock/resource metadata and must not be treated as hardware-verified.",
                        entry.exact_pinout_status
                    ));
                }
            }
            None => {
                warnings.push(format!(
                    "Board `{board_id}` was not found in registries/boards.registry.json; wrapper output cannot be cross-checked against a known pinout status."
                ));
            }
        },
        Err(err) => {
            warnings.push(format!(
                "Could not load board registry to verify `{board_id}` status: {err}; treat wrapper output as unverified for hardware integration."
            ));
        }
    }
}
