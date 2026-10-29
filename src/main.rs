//! lyk-steno-dict: generates a steno dictionary from phonetic rules.
//!
//! The rules (`rules/plover.toml`) say where each sound goes on the keyboard, in Plover theory.
//! They are judged by how many of the words Plover and Lapwing write they reproduce.

mod analysis;
mod data;
mod fetch;
mod generate;
mod gold;
mod quote;
mod rules;
mod stroke;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

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
        /// Briefs to use instead of the built-in ones
        #[arg(long)]
        briefs: Option<PathBuf>,
        /// Check that the README's score table matches this, instead of printing it; exits
        /// non-zero and shows the up-to-date table when it doesn't
        #[arg(long)]
        check_readme: bool,
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
    /// Build the dictionary: briefs first, then the rules, and say what it costs to type
    Build {
        /// Where to write the Plover JSON dictionary
        #[arg(long, default_value = "out/lyk.json")]
        out: PathBuf,
        /// Words to cover, most common first
        #[arg(long, default_value_t = 30_000)]
        common: usize,
        /// Briefs to use instead of the built-in ones
        #[arg(long)]
        briefs: Option<PathBuf>,
        /// Another dictionary (JSON or RTF/CRE, such as Magnum's) to put in the cost tables
        #[arg(long)]
        against: Option<PathBuf>,
    },
    /// Grab a random quote off the web and stroke it out, lyk next to Plover and Lapwing
    Quote {
        /// Words to cover when building the dictionary, most common first
        #[arg(long, default_value_t = 30_000)]
        common: usize,
        /// Briefs to use instead of the built-in ones
        #[arg(long)]
        briefs: Option<PathBuf>,
        /// Try for a quote with at most this many words
        #[arg(long, default_value_t = 20)]
        words: usize,
    },
    /// Read a stroke outline back into sounds, as the engine's last resort when it misses the
    /// dictionary: the word if it's one the dictionary writes, otherwise the rules' best
    /// phonetic reading of the keys, or a true miss if no reading fits them at all
    Resolve {
        /// A stroke outline, such as `KAT` or `TPAEU/^HR`
        outline: String,
        /// Words to cover when building the dictionary, most common first
        #[arg(long, default_value_t = 30_000)]
        common: usize,
        /// Briefs to use instead of the built-in ones
        #[arg(long)]
        briefs: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let setup = || analysis::Setup::load(&cli.data, cli.rules.as_deref());
    match &cli.command {
        Command::Fetch => fetch::run(&cli.data),
        Command::Report => data::report(&cli.data),
        Command::Score {
            common,
            briefs,
            check_readme,
        } => {
            let setup = setup()?;
            let briefs = match briefs {
                Some(path) => generate::Briefs::load(path)?,
                None => generate::Briefs::built_in()?,
            };
            if *check_readme {
                analysis::check_readme(&setup, &briefs, *common, Path::new("README.md"))
            } else {
                analysis::score(&setup, &briefs, *common)
            }
        }
        Command::Mismatches {
            syllables,
            limit,
            missing,
            after,
        } => analysis::mismatches(&setup()?, *syllables, *limit, *missing, *after),
        Command::Learn { min, limit } => analysis::learn(&setup()?, *min, *limit),
        Command::Explain { word } => analysis::explain(&setup()?, word),
        Command::Build {
            out,
            common,
            briefs,
            against,
        } => {
            let setup = setup()?;
            let briefs = match briefs {
                Some(path) => generate::Briefs::load(path)?,
                None => generate::Briefs::built_in()?,
            };
            let against = match against {
                Some(path) => {
                    let name = path
                        .file_stem()
                        .map_or_else(|| "other".to_owned(), |s| s.to_string_lossy().into_owned());
                    Some((name, data::load_words(path)?))
                }
                None => None,
            };
            generate::build(&setup, &briefs, *common, out, against.as_ref())
        }
        Command::Quote {
            common,
            briefs,
            words,
        } => {
            let setup = setup()?;
            let briefs = match briefs {
                Some(path) => generate::Briefs::load(path)?,
                None => generate::Briefs::built_in()?,
            };
            quote::run(&setup, &briefs, *common, *words)
        }
        Command::Resolve {
            outline,
            common,
            briefs,
        } => {
            let setup = setup()?;
            let briefs = match briefs {
                Some(path) => generate::Briefs::load(path)?,
                None => generate::Briefs::built_in()?,
            };
            let strokes = stroke::parse_outline(outline)
                .with_context(|| format!("{outline:?} is not a stroke outline"))?;
            let text = stroke::render_outline(&strokes);
            let generated = generate::generate(&setup, &briefs, *common)?;
            let index = generate::index(&generated);
            if let Some(word) = index.get(&text) {
                println!("{text}  ->  {word}  (in the dictionary)");
            } else if let Some(reading) = setup.rules.resolve(&strokes) {
                println!("{text}  ->  {reading}  (not in the dictionary; the rules' best reading)");
            } else {
                println!("{text}  ->  no reading the rules allow: a true miss");
            }
            Ok(())
        }
    }
}
