//! lyk-steno-dict: generates a steno dictionary from phonetic rules.
//!
//! Step one: load the data and say what is in it, so every later rule can be judged against
//! Plover and Lapwing.

mod data;
mod fetch;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Where the downloaded data lives
    #[arg(long, global = true, default_value = "data")]
    data: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Download the pronunciation, frequency and Plover/Lapwing data (American English)
    Fetch,
    /// Say what was loaded, and how well the pronunciations cover Plover and Lapwing
    Report,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Fetch => fetch::run(&cli.data),
        Command::Report => data::report(&cli.data),
    }
}
