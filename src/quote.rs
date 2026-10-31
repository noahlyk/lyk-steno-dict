//! `quote`: grab a random quote from the web and show it stroke by stroke, lyk next to Plover
//! and Lapwing, so the dictionaries can be compared on text nobody picked to flatter one of them.

use crate::analysis::Setup;
use crate::generate::{self, Briefs};
use crate::stroke::{Stroke, render_outline};
use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// zenquotes.io needs no key and returns a one-element array: `[{"q": "...", "a": "..."}]`.
const SOURCE: &str = "https://zenquotes.io/api/random";

#[derive(Deserialize)]
struct Zen {
    q: String,
    a: String,
}

/// A short quote and who said it. Tries a few times for one under the word limit, so the
/// side-by-side table stays readable; settles for the shortest it saw otherwise.
fn fetch(max_words: usize, tries: u32) -> Result<(String, String)> {
    let mut shortest: Option<(String, String)> = None;
    for _ in 0..tries {
        let body = ureq::get(SOURCE)
            .call()
            .context("fetching a quote")?
            .into_string()
            .context("reading the quote response")?;
        let zen: Vec<Zen> = serde_json::from_str(&body).context("parsing the quote response")?;
        let Some(Zen { q, a }) = zen.into_iter().next() else {
            continue;
        };
        let words = q.split_whitespace().count();
        if words <= max_words {
            return Ok((q, a));
        }
        if shortest
            .as_ref()
            .is_none_or(|(s, _)| words < s.split_whitespace().count())
        {
            shortest = Some((q, a));
        }
    }
    shortest.context("no quote came back")
}

/// A word stripped to letters and apostrophes, lowercased: the form the rules and gold
/// dictionaries key on.
fn clean(word: &str) -> Option<String> {
    let word: String = word
        .chars()
        .filter(|c| c.is_alphabetic() || *c == '\'')
        .collect();
    (!word.is_empty()).then(|| word.to_lowercase())
}

fn outline_of(
    setup: &Setup,
    lyk: &std::collections::HashMap<&str, &[Stroke]>,
    word: &str,
) -> String {
    lyk.get(word)
        .map(|o| render_outline(o))
        .or_else(|| {
            setup
                .pron
                .get(word)
                .into_iter()
                .flatten()
                .find_map(|sounds| setup.rules.outline(word, sounds))
                .map(|o| render_outline(&o))
        })
        // No pronunciation on record at all (`stenography`-class words): spell it out letter by
        // letter rather than reporting the word as simply untypeable.
        .or_else(|| setup.rules.spell_word(word).map(|o| render_outline(&o)))
        .unwrap_or_else(|| "-".to_owned())
}

/// Grab a random quote, build the dictionary, and print it stroke by stroke next to Plover and
/// Lapwing: the keys each theory spends on the same, unpicked text.
pub fn run(setup: &Setup, briefs: &Briefs, common: usize, max_words: usize) -> Result<()> {
    let (quote, author) = fetch(max_words, 5)?;
    println!("\"{quote}\"\n  — {author}\n");

    let generated = generate::generate(setup, briefs, common)?;
    let lyk = generate::cheapest(&generated.entries);

    let rows: Vec<(String, String, String, String)> = quote
        .split_whitespace()
        .filter_map(clean)
        .map(|word| {
            let plover = setup
                .gold
                .plover
                .get(&word)
                .and_then(|o| generate::shortest(o))
                .map_or_else(|| "-".to_owned(), |o| render_outline(&o));
            let lapwing = setup
                .gold
                .lapwing
                .get(&word)
                .and_then(|o| generate::shortest(o))
                .map_or_else(|| "-".to_owned(), |o| render_outline(&o));
            let ours = outline_of(setup, &lyk, &word);
            (word, ours, plover, lapwing)
        })
        .collect();
    if rows.is_empty() {
        bail!("the quote had no plain words to stroke");
    }

    let width = rows.iter().map(|(w, ..)| w.len()).max().unwrap_or(4).max(4);
    println!(
        "{:<width$}  {:<12}  {:<12}  {:<12}",
        "word", "lyk", "plover", "lapwing"
    );
    let mut total = [0u32; 3];
    for (word, ours, plover, lapwing) in &rows {
        for (n, text) in [ours, plover, lapwing].into_iter().enumerate() {
            if text != "-" {
                total[n] +=
                    crate::generate::keys(&crate::stroke::parse_outline(text).unwrap_or_default());
            }
        }
        println!("{word:<width$}  {ours:<12}  {plover:<12}  {lapwing:<12}");
    }
    println!(
        "{:<width$}  {:<12}  {:<12}  {:<12}",
        "— keys —", total[0], total[1], total[2]
    );
    Ok(())
}
