//! Putting the dictionary together: briefs first, then the rules, with clashes settled in a fixed
//! order so the result is predictable.

use crate::analysis::Setup;
use crate::stroke::{Stroke, parse_outline, render_outline};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

const BUILT_IN_BRIEFS: &str = include_str!("../rules/briefs.toml");

#[derive(Deserialize, Default)]
pub struct Briefs {
    /// How many of the most common words get a brief
    #[serde(default)]
    count: usize,
    #[serde(default = "plover")]
    default: String,
    #[serde(default)]
    word: HashMap<String, String>,
}

fn plover() -> String {
    "plover".into()
}

impl Briefs {
    pub fn built_in() -> Result<Self> {
        Ok(toml::from_str(BUILT_IN_BRIEFS)?)
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).with_context(|| path.display().to_string())?;
        Ok(toml::from_str(&text)?)
    }
}

/// How a dictionary entry came about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Origin {
    Brief,
    Rules,
    /// A long word written as one stroke of its consonants
    Skeleton,
    /// Another pronunciation of a word
    Alternate,
    /// The ladder step that settled a clash: 1 is the star on the first stroke
    Ladder(u8),
}

pub struct Entry {
    pub word: String,
    pub outline: Vec<Stroke>,
    pub origin: Origin,
}

pub struct Generated {
    pub entries: Vec<Entry>,
    pub unresolved: Vec<String>,
}

/// The shortest outline (fewest strokes, then fewest keys) from a list.
fn shortest(outlines: &[String]) -> Option<Vec<Stroke>> {
    outlines
        .iter()
        .filter_map(|o| parse_outline(o))
        .min_by_key(|o| (o.len(), keys(o)))
}

/// How many keys an outline takes to press.
pub fn keys(outline: &[Stroke]) -> u32 {
    outline.iter().map(|s| s.0.count_ones()).sum()
}

/// The other outlines to try for a word whose outline is taken, in the order they are tried.
fn ladder(outline: &[Stroke], join: bool) -> Vec<(u8, Vec<Stroke>)> {
    let mut steps = Vec::new();
    let star = |outline: &[Stroke], at: usize| -> Option<Vec<Stroke>> {
        let stroke = outline.get(at)?;
        (stroke.0 & Stroke::STAR == 0).then(|| {
            let mut next = outline.to_vec();
            next[at] = Stroke(stroke.0 | Stroke::STAR);
            next
        })
    };
    if let Some(next) = star(outline, 0) {
        steps.push((1, next));
    }
    if outline.len() > 1
        && let Some(next) = star(outline, outline.len() - 1)
    {
        steps.push((2, next));
    }
    // A stroke of just the star after the word marks it as the other one
    let marker = Stroke(Stroke::STAR | if join { Stroke::JOIN } else { 0 });
    for (n, extra) in [(3u8, 1usize), (4, 2)] {
        let mut next = outline.to_vec();
        next.extend(std::iter::repeat_n(marker, extra));
        steps.push((n, next));
    }
    steps
}

pub fn generate(setup: &Setup, briefs: &Briefs, common: usize) -> Result<Generated> {
    let mut taken: HashMap<String, String> = HashMap::new();
    let mut entries: Vec<Entry> = Vec::new();
    let mut pinned: HashSet<String> = HashSet::new();

    // Words, most common first
    let mut words: Vec<(&String, usize)> = setup
        .rank
        .iter()
        .filter(|(w, r)| **r < common && setup.pron.contains_key(w.as_str()))
        .map(|(w, r)| (w, *r))
        .collect();
    words.sort_by_key(|(w, r)| (*r, w.as_str()));

    // Briefs: the common words Plover or Lapwing already write
    for (word, rank) in &words {
        if *rank >= briefs.count || word.chars().count() < 2 {
            continue;
        }
        let source = briefs.word.get(word.as_str()).unwrap_or(&briefs.default);
        let outline = match source.as_str() {
            "plover" => setup
                .gold
                .plover
                .get(word.as_str())
                .and_then(|o| shortest(o)),
            "lapwing" => setup
                .gold
                .lapwing
                .get(word.as_str())
                .and_then(|o| shortest(o)),
            explicit => parse_outline(explicit),
        };
        // A word only one of them writes still gets its brief
        let outline = outline
            .or_else(|| {
                setup
                    .gold
                    .plover
                    .get(word.as_str())
                    .and_then(|o| shortest(o))
            })
            .or_else(|| {
                setup
                    .gold
                    .lapwing
                    .get(word.as_str())
                    .and_then(|o| shortest(o))
            });
        let Some(outline) = outline else { continue };
        let text = render_outline(&outline);
        if taken.contains_key(&text) {
            continue;
        }
        taken.insert(text, (*word).clone());
        pinned.insert((*word).clone());
        entries.push(Entry {
            word: (*word).clone(),
            outline,
            origin: Origin::Brief,
        });
    }

    let join = setup.rules.join_key();
    let mut unresolved = Vec::new();
    for (word, _) in &words {
        if pinned.contains(word.as_str()) {
            continue;
        }
        let mut outlines: Vec<Vec<Stroke>> = Vec::new();
        for sounds in &setup.pron[word.as_str()] {
            if let Some(outline) = setup.rules.outline(word, sounds)
                && !outlines.contains(&outline)
            {
                outlines.push(outline);
            }
        }
        let Some((first, others)) = outlines.split_first() else {
            unresolved.push((*word).clone());
            continue;
        };
        let mut placed = false;
        // The commonest long words go first for the fewest keys: a single skeleton stroke
        let mut options: Vec<(u8, Vec<Stroke>)> = Vec::new();
        if setup.rank[word.as_str()] < setup.rules.skeleton_rank() {
            for sounds in &setup.pron[word.as_str()] {
                for stroke in setup.rules.skeletons(word, sounds) {
                    if !options.iter().any(|(_, o)| o == &vec![stroke]) {
                        options.push((9, vec![stroke]));
                    }
                }
            }
        }
        options.push((0, first.clone()));
        options.extend(ladder(first, join));
        for (step, outline) in options {
            let text = render_outline(&outline);
            if !taken.contains_key(&text) {
                taken.insert(text, (*word).clone());
                let origin = match step {
                    0 => Origin::Rules,
                    9 => Origin::Skeleton,
                    n => Origin::Ladder(n),
                };
                entries.push(Entry {
                    word: (*word).clone(),
                    outline,
                    origin,
                });
                placed = true;
                break;
            }
        }
        if !placed {
            unresolved.push((*word).clone());
        }
        // Other pronunciations are only added where they clash with nothing
        for outline in others {
            let text = render_outline(outline);
            if !taken.contains_key(&text) {
                taken.insert(text, (*word).clone());
                entries.push(Entry {
                    word: (*word).clone(),
                    outline: outline.clone(),
                    origin: Origin::Alternate,
                });
            }
        }
    }
    Ok(Generated {
        entries,
        unresolved,
    })
}

/// The dictionary as Plover JSON.
pub fn write_json(generated: &Generated, path: &Path) -> Result<()> {
    let mut map: BTreeMap<String, &str> = BTreeMap::new();
    for entry in &generated.entries {
        map.insert(render_outline(&entry.outline), entry.word.as_str());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(&map)?)
        .with_context(|| path.display().to_string())?;
    Ok(())
}

/// Build the dictionary, write it, and say what it costs to type next to Plover and Lapwing.
pub fn build(setup: &Setup, briefs: &Briefs, common: usize, out: &Path) -> Result<()> {
    let generated = generate(setup, briefs, common)?;
    write_json(&generated, out)?;

    let mut by_origin: BTreeMap<String, usize> = BTreeMap::new();
    for entry in &generated.entries {
        *by_origin.entry(format!("{:?}", entry.origin)).or_default() += 1;
    }
    println!(
        "wrote {} entries to {}",
        generated.entries.len(),
        out.display()
    );
    for (origin, n) in &by_origin {
        println!("  {origin:<12} {n:>7}");
    }
    println!(
        "  words the rules could not place: {}",
        generated.unresolved.len()
    );

    // Each word's cheapest outline here, and what Plover and Lapwing spend on the same word
    let mut ours: HashMap<&str, (usize, u32)> = HashMap::new();
    for entry in &generated.entries {
        let cost = (entry.outline.len(), keys(&entry.outline));
        let best = ours.entry(entry.word.as_str()).or_insert(cost);
        *best = (*best).min(cost);
    }
    let cost_of = |outlines: Option<&Vec<String>>| {
        let outline = shortest(outlines?)?;
        Some((outline.len(), keys(&outline)))
    };
    let mut total = [0f64; 7];
    let mut worse: Vec<(f64, &str, u32, u32)> = Vec::new();
    for (word, &(strokes, key_count)) in &ours {
        let (Some(plover), Some(lapwing)) = (
            cost_of(setup.gold.plover.get(*word)),
            cost_of(setup.gold.lapwing.get(*word)),
        ) else {
            continue;
        };
        let weight = setup.count.get(*word).copied().unwrap_or(0) as f64;
        total[0] += weight;
        total[1] += weight * strokes as f64;
        total[2] += weight * f64::from(key_count);
        total[3] += weight * plover.0 as f64;
        total[4] += weight * f64::from(plover.1);
        total[5] += weight * lapwing.0 as f64;
        total[6] += weight * f64::from(lapwing.1);
        if key_count > plover.1 {
            worse.push((
                weight * f64::from(key_count - plover.1),
                word,
                key_count,
                plover.1,
            ));
        }
    }
    println!("\nwhat a word costs on average, weighted by how often it is said");
    println!("(words all three write; strokes and keys pressed per word)");
    println!("            strokes     keys");
    for (name, strokes, key_count) in [
        ("this", total[1], total[2]),
        ("plover", total[3], total[4]),
        ("lapwing", total[5], total[6]),
    ] {
        println!(
            "  {name:<9} {:>7.3} {:>8.3}",
            strokes / total[0],
            key_count / total[0]
        );
    }
    worse.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!("\nthe words that cost the most extra keys over plover");
    for (_, word, ours_keys, plover_keys) in worse.iter().take(40) {
        let outline = generated
            .entries
            .iter()
            .filter(|e| e.word == *word)
            .map(|e| render_outline(&e.outline))
            .collect::<Vec<_>>()
            .join(" ");
        let plover = setup
            .gold
            .plover
            .get(*word)
            .and_then(|o| o.first())
            .map_or("", String::as_str);
        println!(
            "  {word:<12} {ours_keys} vs {plover_keys} keys   ours {outline:<24} plover {plover}"
        );
    }
    Ok(())
}
