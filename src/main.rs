//! lyk-steno-dict: generates a steno dictionary from phonetic rules.
//!
//! The rules (`rules/plover.toml`) say where each sound goes on the keyboard, in Plover theory.
//! They are judged by how many of the words Plover and Lapwing write they reproduce.

mod analysis;
mod data;
mod fetch;
mod gold;
mod rules;
mod stroke;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Where the downloaded data lives
    #[arg(long, global = true, default_value = "data")]
    data: PathBuf,
    /// Rules to use instead of the built-in ones
    #[arg(long, global = true)]
    rules: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Download the pronunciation, frequency and Plover/Lapwing data (American English)
    Fetch,
    /// Say what was loaded, and how well the pronunciations cover Plover and Lapwing
    Report,
    /// How many words the rules write the way Plover or Lapwing do
    Score {
        /// Only the this many most common words
        #[arg(long, default_value_t = 20_000)]
        common: usize,
    },
    /// The most common words the rules get wrong
    Mismatches {
        /// Only words with this many syllables
        #[arg(long, default_value_t = 1)]
        syllables: usize,
        #[arg(long, default_value_t = 40)]
        limit: usize,
        /// Only words the rules cannot write at all
        #[arg(long)]
        missing: bool,
        /// Skip the this many most common words: they get briefs
        #[arg(long, default_value_t = 0)]
        after: usize,
    },
    /// Which keys Plover and Lapwing use for each group of sounds, where they agree
    Learn {
        /// Hide groups with fewer words than this
        #[arg(long, default_value_t = 8)]
        min: usize,
        #[arg(long, default_value_t = 60)]
        limit: usize,
    },
    /// Show how the rules write a word
    Explain { word: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let setup = || analysis::Setup::load(&cli.data, cli.rules.as_deref());
    match &cli.command {
        Command::Fetch => fetch::run(&cli.data),
        Command::Report => data::report(&cli.data),
        Command::Score { common } => analysis::score(&setup()?, *common),
        Command::Mismatches {
            syllables,
            limit,
            missing,
            after,
        } => analysis::mismatches(&setup()?, *syllables, *limit, *missing, *after),
        Command::Learn { min, limit } => analysis::learn(&setup()?, *min, *limit),
        Command::Explain { word } => analysis::explain(&setup()?, word),
    }
}
