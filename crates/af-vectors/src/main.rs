// SPDX-License-Identifier: AGPL-3.0-or-later
use std::path::PathBuf;

use af_vectors::{generate_mod_add_vectors, GenerateConfig};
use clap::Parser;

type DynError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Debug, Parser)]
#[command(name = "af-vectors", version)]
struct Args {
    #[arg(long, default_value = "vectors/af_mod_add_basic.json")]
    basic_out: PathBuf,
    #[arg(long, default_value = "vectors/af_mod_add_random.json")]
    random_out: PathBuf,
    #[arg(long, default_value = "vectors/af_mod_add_random.svh")]
    svh_out: PathBuf,
    #[arg(long, default_value_t = 64)]
    count: usize,
    #[arg(long, default_value = "0x1234567890ABCDEF")]
    seed: String,
}

fn main() -> Result<(), DynError> {
    let args = Args::parse();
    let report = generate_mod_add_vectors(&GenerateConfig {
        basic_out: args.basic_out,
        random_out: args.random_out,
        svh_out: args.svh_out,
        count: args.count,
        seed: args.seed,
    })?;
    println!(
        "generated {} basic vectors and {} random vectors",
        report.basic_count, report.random_count
    );
    Ok(())
}
