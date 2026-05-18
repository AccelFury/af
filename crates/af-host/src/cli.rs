// SPDX-License-Identifier: AGPL-3.0-or-later
use clap::Subcommand;

#[derive(Subcommand)]
pub enum Command {
    Ping,
}
