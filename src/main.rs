//! lyk-steno-theory: generates a steno dictionary from phonetic rules.
//!
//! The rules (`rules/plover.toml`) say where each sound goes on the keyboard, in Plover theory.
//! They are judged by how many of the words Plover and Lapwing write they reproduce.

mod analysis;
mod data;
mod fetch;
mod generate;
mod gold;
mod quote;
mod reverse;
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
        /// Check that README.md's score table matches this, instead of printing it; exits
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
    /// Generate the keymux WASM plugin that reads a stroke back the way these rules would,
    /// for whatever a dictionary misses: the single source of this logic, so the deployed
    /// plugin can never drift from the theory it is supposed to mirror
    Plugin {
        /// Where to write the plugin's Rust source
        #[arg(long, default_value = "out/lyk-orthographic-fallback.rs")]
        out: PathBuf,
        /// Words to cover when building the dictionary's single-stroke stem table, most common
        /// first
        #[arg(long, default_value_t = 30_000)]
        common: usize,
        /// Briefs to use instead of the built-in ones
        #[arg(long)]
        briefs: Option<PathBuf>,
    },
    /// Build the dictionary: briefs first, then the rules, and say what it costs to type
    Build {
        /// Where to write the Plover JSON dictionary
        #[arg(long, default_value = "out/lyk.json")]
        out: PathBuf,
        /// Words to cover, most common first. 100,000 is already in Plover/Lapwing's own
        /// ballpark and builds in a few seconds; raising it further has diminishing returns,
        /// bounded by how many words the pronunciation data covers, not by build time
        #[arg(long, default_value_t = 100_000)]
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
        /// A stroke outline, such as `KAT` or `TPAEU/+HR`
        outline: String,
        /// Words to cover when building the dictionary, most common first
        #[arg(long, default_value_t = 30_000)]
        common: usize,
        /// Briefs to use instead of the built-in ones
        #[arg(long)]
        briefs: Option<PathBuf>,
    },
    /// Look a word, phrase, or stroke up, whichever it is: `lookup cat` finds lyk's outline next
    /// to Plover's and Lapwing's, `lookup "tell me how"` does the same word by word for a whole
    /// phrase, and `lookup KAT` (all caps, a valid outline) finds what that stroke resolves to
    /// instead, the same as `resolve`. Mirrors keymux's own `steno lookup`.
    Lookup {
        /// The word, phrase, or stroke to look up
        query: String,
        /// Words to cover when building the dictionary, most common first (only matters for a
        /// stroke query, which has to generate the dictionary to tell a real entry from a miss)
        #[arg(long, default_value_t = 30_000)]
        common: usize,
        /// Briefs to use instead of the built-in ones
        #[arg(long)]
        briefs: Option<PathBuf>,
    },
    /// Build the dictionary and the plugin, then copy both straight into the live keymux config:
    /// the one command for everything `build` and `plugin` otherwise leave as a manual copy/paste
    Deploy {
        /// Where keymux's lyk config lives
        #[arg(long, default_value = "~/.config/keymux/steno/lyk")]
        to: PathBuf,
        /// Words to cover, most common first
        #[arg(long, default_value_t = 100_000)]
        common: usize,
        /// Briefs to use instead of the built-in ones
        #[arg(long)]
        briefs: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let setup =
        |common: Option<usize>| analysis::Setup::load(&cli.data, cli.rules.as_deref(), common);
    match &cli.command {
        Command::Fetch => fetch::run(&cli.data),
        Command::Report => data::report(&cli.data),
        Command::Score {
            common,
            briefs,
            check_readme,
        } => {
            let setup = setup(Some(*common))?;
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
        } => analysis::mismatches(&setup(None)?, *syllables, *limit, *missing, *after),
        Command::Learn { min, limit } => analysis::learn(&setup(None)?, *min, *limit),
        Command::Explain { word } => analysis::explain(&setup(None)?, word),
        Command::Plugin { out, common, briefs } => {
            let setup = setup(Some(*common))?;
            let briefs = match briefs {
                Some(path) => generate::Briefs::load(path)?,
                None => generate::Briefs::built_in()?,
            };
            write_plugin(&setup, &briefs, *common, cli.rules.as_deref(), out)
        }
        Command::Build {
            out,
            common,
            briefs,
            against,
        } => {
            let setup = setup(Some(*common))?;
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
            let setup = setup(Some(*common))?;
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
            let setup = setup(Some(*common))?;
            let briefs = match briefs {
                Some(path) => generate::Briefs::load(path)?,
                None => generate::Briefs::built_in()?,
            };
            let strokes = stroke::parse_outline(outline)
                .with_context(|| format!("{outline:?} is not a stroke outline"))?;
            print_resolve(&setup, &briefs, &strokes, *common)
        }
        Command::Lookup {
            query,
            common,
            briefs,
        } => run_lookup(&setup, query, *common, briefs.as_deref()),
        Command::Deploy { to, common, briefs } => {
            let setup = setup(Some(*common))?;
            let briefs = match briefs {
                Some(path) => generate::Briefs::load(path)?,
                None => generate::Briefs::built_in()?,
            };
            deploy(&setup, &briefs, *common, cli.rules.as_deref(), to)
        }
    }
}

/// Every file lyk's keymux config is made of, besides the two this binary generates fresh each
/// time (the words dictionary and the fallback plugin): the reserved stroke tables this repo
/// already tracks as its own source of truth (`rules/reserved/`), verbatim, under the numbered
/// name keymux loads them by, plus the one piece of generic keymux scaffolding (the theory-name
/// marker) every theory folder needs to be recognized and reloadable at all. Static, not derived
/// from the rules, so `deploy` can rebuild the whole folder from nothing rather than silently
/// dropping anything it doesn't happen to find already there.
///
/// `rules/reserved/emilys-modifiers.py` and its `regenerate.py` tool are deliberately left out of
/// this list: keymux only ever reads the `.json` a `.py` dictionary already has to be converted
/// to, and never runs the `.py` itself (see keymux's own `setup::LAPWING_FILES` comment — a `.py`
/// "needs Plover's plugin runtime, so it isn't used here"). They stay in `rules/reserved/` as this
/// repo's own source and dev tool for hand-editing that one file, not as something to ship into a
/// live config: run `python3 rules/reserved/regenerate.py` there and commit the result if that
/// file ever changes.
const STATIC_FILES: &[(&str, &str)] = &[
    ("01-lyk-commands.json", include_str!("../rules/reserved/commands.json")),
    ("03-lyk-punctuation.json", include_str!("../rules/reserved/punctuation.json")),
    ("04-lyk-emilys-modifiers.json", include_str!("../rules/reserved/emilys-modifiers.json")),
    (".keymux-theory", "lyk\n"),
];

/// `build` and `plugin`, plus every reserved stroke table and the generic scaffolding keymux
/// needs, straight into keymux's live config: the whole folder from one command, not a partial
/// update that assumes the rest is already sitting there correctly.
fn deploy(
    setup: &analysis::Setup,
    briefs: &generate::Briefs,
    common: usize,
    rules_path: Option<&Path>,
    to: &Path,
) -> Result<()> {
    let to = expand_home(to)?;
    std::fs::create_dir_all(&to).with_context(|| to.display().to_string())?;

    let words_out = PathBuf::from("out/lyk.json");
    generate::build(setup, briefs, common, &words_out, None)?;
    let plugin_out = PathBuf::from("out/lyk-orthographic-fallback.rs");
    write_plugin(setup, briefs, common, rules_path, &plugin_out)?;

    let words_to = to.join("02-lyk-words.json");
    let plugin_to = to.join("05-lyk-orthographic-fallback.rs");
    std::fs::copy(&words_out, &words_to).with_context(|| words_to.display().to_string())?;
    std::fs::copy(&plugin_out, &plugin_to).with_context(|| plugin_to.display().to_string())?;
    for (from, dest) in [(&words_out, &words_to), (&plugin_out, &plugin_to)] {
        let (a, b) = (std::fs::read(from)?, std::fs::read(dest)?);
        anyhow::ensure!(a == b, "{} and {} differ after copying", from.display(), dest.display());
    }

    for (name, content) in STATIC_FILES {
        let dest = to.join(name);
        std::fs::write(&dest, content).with_context(|| dest.display().to_string())?;
        let written = std::fs::read_to_string(&dest).with_context(|| dest.display().to_string())?;
        anyhow::ensure!(written == *content, "{} differs after writing", dest.display());
    }

    // An earlier `deploy` shipped these two here by mistake (see `STATIC_FILES`'s own comment on
    // why they never belonged in a live config); remove them if a past run already left them
    // behind, rather than leaving dead files for `deploy` to never again mention or explain.
    for stale in ["04-lyk-emilys-modifiers.py", "regenerate.py"] {
        let path = to.join(stale);
        if path.exists() {
            std::fs::remove_file(&path).with_context(|| path.display().to_string())?;
        }
    }

    println!(
        "deployed to {} ({} generated, {} static)",
        to.display(),
        2,
        STATIC_FILES.len()
    );
    Ok(())
}

/// A leading `~/` or bare `~`, expanded against `$HOME`; any other path is returned as-is.
fn expand_home(path: &Path) -> Result<PathBuf> {
    let Ok(rest) = path.strip_prefix("~") else {
        return Ok(path.to_path_buf());
    };
    let home = std::env::var("HOME").context("$HOME is not set, and the path starts with ~")?;
    Ok(PathBuf::from(home).join(rest))
}

/// The one lookup, forward or backward: see `reverse::classify`.
fn run_lookup(
    setup: &impl Fn(Option<usize>) -> Result<analysis::Setup>,
    query: &str,
    common: usize,
    briefs_path: Option<&Path>,
) -> Result<()> {
    match reverse::classify(query) {
        reverse::Query::Stroke(strokes) => {
            let setup = setup(Some(common))?;
            let briefs = match briefs_path {
                Some(path) => generate::Briefs::load(path)?,
                None => generate::Briefs::built_in()?,
            };
            print_resolve(&setup, &briefs, &strokes, common)
        }
        reverse::Query::Text(phrase) => {
            if phrase.is_empty() {
                println!("Nothing to look up");
                return Ok(());
            }
            let setup = setup(None)?;
            for found in reverse::reverse_phrase(&setup, &phrase) {
                println!(
                    "{}\n  lyk      {}\n  plover   {}\n  lapwing  {}",
                    found.word,
                    found.lyk.as_deref().unwrap_or("?"),
                    found.plover.as_deref().unwrap_or("?"),
                    found.lapwing.as_deref().unwrap_or("?"),
                );
            }
            Ok(())
        }
    }
}

/// What a stroke outline resolves to: a real dictionary entry, the rules' best phonetic reading
/// of a miss, or a true miss nothing can read. Shared by `resolve` and `lookup`'s stroke mode.
fn print_resolve(
    setup: &analysis::Setup,
    briefs: &generate::Briefs,
    strokes: &[stroke::Stroke],
    common: usize,
) -> Result<()> {
    let text = stroke::render_outline(strokes);
    let generated = generate::generate(setup, briefs, common)?;
    let index = generate::index(&generated);
    if let Some(word) = index.get(&text) {
        println!("{text}  ->  {word}  (in the dictionary)");
    } else if let Some(reading) = setup.rules.resolve(strokes, Some(&generate::single_stroke_stems(&generated))) {
        println!("{text}  ->  {reading}  (not in the dictionary; the rules' best reading)");
    } else {
        println!("{text}  ->  no reading the rules allow: a true miss");
    }
    Ok(())
}

/// Generate the orthographic-fallback plugin's source and write it to `out`. Builds the
/// dictionary (same `setup`/`briefs`/`common` as `build`/`deploy`) to embed its single-stroke
/// stem table in the plugin — see [`rules::Rules::generate_plugin`].
fn write_plugin(
    setup: &analysis::Setup,
    briefs: &generate::Briefs,
    common: usize,
    rules_path: Option<&Path>,
    out: &Path,
) -> Result<()> {
    let rules = match rules_path {
        Some(path) => rules::Rules::load(path)?,
        None => rules::Rules::built_in()?,
    };
    let generated = generate::generate(setup, briefs, common)?;
    let stems = generate::single_stroke_stems(&generated);
    let source = rules.generate_plugin(&stems);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(out, source).with_context(|| out.display().to_string())?;
    println!("wrote {}", out.display());
    Ok(())
}
