// SPDX-License-Identifier: AGPL-3.0-or-later
mod cli;
mod serial;

use clap::Parser;

#[derive(clap::Parser)]
#[command(name = "af-host")]
struct Args {
    #[command(subcommand)]
    cmd: cli::Command,
}

fn main() -> Result<(), String> {
    let args = Args::parse();
    match args.cmd {
        cli::Command::Ping => {
            println!("af-host ready");
            serial::hello()
        }
    }
}
