//! Loading the downloaded data: pronunciations, word frequencies, and the dictionaries to match.

use anyhow::{Context, Result};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;

/// A word's pronunciations, each a list of phones.
pub type Pronunciations = HashMap<String, Vec<Vec<String>>>;

/// `CMUdict`: `word PH1 PH2 ...`, with `word(2)` for further pronunciations and `;;;` comments.
pub fn load_cmudict(path: &Path) -> Result<Pronunciations> {
    let text = fs::read_to_string(path).with_context(|| path.display().to_string())?;
    let mut words = Pronunciations::new();
    for line in text.lines().filter(|l| !l.starts_with(";;;")) {
        let mut parts = line.split_whitespace();
        let Some(word) = parts.next() else { continue };
        let word = word.split('(').next().unwrap_or(word).to_lowercase();
        words
            .entry(word)
            .or_default()
            .push(parts.map(str::to_owned).collect());
    }
    Ok(words)
}

/// Montreal Forced Aligner dictionaries: `word<TAB>probabilities...<TAB>phones separated by spaces`.
pub fn load_mfa(path: &Path) -> Result<Pronunciations> {
    let text = fs::read_to_string(path).with_context(|| path.display().to_string())?;
    let mut words = Pronunciations::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        let (Some(word), Some(phones)) = (fields.first(), fields.last()) else {
            continue;
        };
        if fields.len() < 2 {
            continue;
        }
        words
            .entry(word.to_lowercase())
            .or_default()
            .push(phones.split_whitespace().map(str::to_owned).collect());
    }
    Ok(words)
}

/// `WikiPron`: `word<TAB>phones separated by spaces`.
pub fn load_wikipron(path: &Path) -> Result<Pronunciations> {
    let text = fs::read_to_string(path).with_context(|| path.display().to_string())?;
    let mut words = Pronunciations::new();
    for line in text.lines() {
        let Some((word, phones)) = line.split_once('\t') else {
            continue;
        };
        words
            .entry(word.to_lowercase())
            .or_default()
            .push(phones.split_whitespace().map(str::to_owned).collect());
    }
    Ok(words)
}

/// Word counts, most common first: `word count` per line.
pub fn load_frequency(path: &Path) -> Result<Vec<(String, u64)>> {
    let text = fs::read_to_string(path).with_context(|| path.display().to_string())?;
    Ok(text
        .lines()
        .filter_map(|line| {
            let (word, count) = line.rsplit_once(' ')?;
            Some((word.to_owned(), count.parse().ok()?))
        })
        .collect())
}

/// A Plover dictionary: outline (strokes joined by `/`) to translation.
pub fn load_dictionary(path: &Path) -> Result<BTreeMap<String, String>> {
    let text = fs::read_to_string(path).with_context(|| path.display().to_string())?;
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rtf"))
    {
        return Ok(load_rtf(&text));
    }
    serde_json::from_str(&text).with_context(|| path.display().to_string())
}

/// An RTF/CRE dictionary (what court reporting software and Magnum ship): each entry is
/// `{\*\cxs OUTLINE}translation`. Only the plain text of a translation is kept; formatting
/// control words end it.
fn load_rtf(text: &str) -> BTreeMap<String, String> {
    const MARK: &str = "{\\*\\cxs ";
    let mut dict = BTreeMap::new();
    let mut rest = text;
    while let Some(at) = rest.find(MARK) {
        rest = &rest[at + MARK.len()..];
        let Some(end) = rest.find('}') else { break };
        let outline = rest[..end].trim().to_owned();
        rest = &rest[end + 1..];
        let stop = rest
            .find(['{', '}', '\\', '\r', '\n'])
            .unwrap_or(rest.len());
        let translation = rest[..stop].trim();
        if !outline.is_empty() && !translation.is_empty() {
            dict.entry(outline)
                .or_insert_with(|| translation.to_owned());
        }
    }
    dict
}

/// Every plain word a dictionary file (JSON or RTF) writes, with its shortest outline.
pub fn load_words(path: &Path) -> Result<HashMap<String, String>> {
    Ok(word_outlines(&load_dictionary(path)?))
}

/// Is the translation a plain word, with nothing but letters, apostrophes or hyphens?
fn plain_word(translation: &str) -> Option<String> {
    let word = translation.trim();
    (!word.is_empty()
        && word
            .chars()
            .all(|c| c.is_alphabetic() || c == '\'' || c == '-')
        && word.chars().any(char::is_alphabetic))
    .then(|| word.to_lowercase())
}

/// Every plain word a dictionary writes, with its shortest outline (fewest strokes, then keys).
fn word_outlines(dict: &BTreeMap<String, String>) -> HashMap<String, String> {
    let mut best: HashMap<String, String> = HashMap::new();
    for (outline, translation) in dict {
        let Some(word) = plain_word(translation) else {
            continue;
        };
        let rank = |o: &str| (o.split('/').count(), o.len());
        match best.get(&word) {
            Some(current) if rank(current) <= rank(outline) => {}
            _ => {
                best.insert(word, outline.clone());
            }
        }
    }
    best
}

/// Print what was loaded, and how well the pronunciations cover what Plover and Lapwing write.
pub fn report(root: &Path) -> Result<()> {
    let pron = root.join("pronunciation");
    let sources: Vec<(&str, Pronunciations)> = vec![
        ("cmudict", load_cmudict(&pron.join("cmudict.dict"))?),
        ("mfa us", load_mfa(&pron.join("english_us_mfa.dict"))?),
        (
            "wikipron us",
            load_wikipron(&pron.join("wikipron_us_broad.tsv"))?,
        ),
    ];
    println!("pronunciations");
    for (name, words) in &sources {
        let variants: usize = words.values().map(Vec::len).sum();
        println!(
            "  {name:<12} {:>7} words, {variants:>7} pronunciations",
            words.len()
        );
    }
    let mut union: HashSet<&str> = HashSet::new();
    for (_, words) in &sources {
        union.extend(words.keys().map(String::as_str));
    }
    println!("  {:<12} {:>7} words", "all together", union.len());

    let frequency = load_frequency(&root.join("frequency/en_full.txt"))?;
    println!("\nfrequency list: {} words", frequency.len());

    let dicts = root.join("dictionaries");
    let plover = word_outlines(&load_dictionary(&dicts.join("plover-main.json"))?);
    let lapwing = word_outlines(&load_dictionary(&dicts.join("lapwing-base.json"))?);
    println!("\ndictionaries (plain words only, one shortest outline each)");
    println!("  plover  {:>7} words", plover.len());
    println!("  lapwing {:>7} words", lapwing.len());
    let both = plover
        .iter()
        .filter(|(word, outline)| lapwing.get(*word) == Some(outline))
        .count();
    let either = plover
        .keys()
        .chain(lapwing.keys())
        .collect::<HashSet<_>>()
        .len();
    println!("  {both} words have the same shortest outline in both, out of {either} between them");

    println!("\nwords of Plover or Lapwing that have a pronunciation");
    let wanted: HashSet<&String> = plover.keys().chain(lapwing.keys()).collect();
    for (name, words) in &sources {
        let have = wanted
            .iter()
            .filter(|w| words.contains_key(w.as_str()))
            .count();
        println!("  {name:<12} {have:>7} of {}", wanted.len());
    }
    let have_any = wanted.iter().filter(|w| union.contains(w.as_str())).count();
    println!("  {:<12} {have_any:>7} of {}", "all together", wanted.len());

    println!("\nthe 5000 most common words that have a pronunciation");
    let top = frequency.iter().take(5000);
    let (mut covered, mut total) = (0, 0);
    for (word, _) in top {
        total += 1;
        if union.contains(word.as_str()) {
            covered += 1;
        }
    }
    println!("  {covered} of {total}");
    Ok(())
}
