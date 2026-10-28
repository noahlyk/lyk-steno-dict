//! Judging the rules against Plover and Lapwing, and learning which rules are missing.

use crate::data::{Pronunciations, load_cmudict, load_frequency};
use crate::gold::Gold;
use crate::rules::Rules;
use crate::stroke::{Stroke, render_outline};
use anyhow::Result;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

pub struct Setup {
    pub rules: Rules,
    pub pron: Pronunciations,
    pub gold: Gold,
    /// 0 is the most common word
    pub rank: HashMap<String, usize>,
    /// How often each word is said
    pub count: HashMap<String, u64>,
}

impl Setup {
    pub fn load(data: &Path, rules: Option<&Path>) -> Result<Self> {
        let rules = match rules {
            Some(path) => Rules::load(path)?,
            None => Rules::built_in()?,
        };
        let frequency = load_frequency(&data.join("frequency/en_full.txt"))?;
        let mut rank = HashMap::new();
        let mut count = HashMap::new();
        for (n, (word, times)) in frequency.into_iter().enumerate() {
            count.entry(word.clone()).or_insert(times);
            rank.entry(word).or_insert(n);
        }
        Ok(Self {
            rules,
            pron: load_cmudict(&data.join("pronunciation/cmudict.dict"))?,
            gold: Gold::load(data)?,
            rank,
            count,
        })
    }

    /// Every outline the rules give a word, one per pronunciation that can be written, and the
    /// form its spelling gives it when it clashes with a word that sounds the same.
    pub fn candidates(&self, word: &str) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        for sounds in self.pron.get(word).into_iter().flatten() {
            for outline in [
                self.rules.outline(word, sounds),
                self.rules.variant(word, sounds),
            ]
            .into_iter()
            .flatten()
            {
                let text = render_outline(&outline);
                if !found.contains(&text) {
                    found.push(text);
                }
            }
        }
        found
    }

    fn syllable_count(&self, word: &str) -> usize {
        self.pron
            .get(word)
            .and_then(|all| all.first())
            .map_or(0, |sounds| {
                sounds
                    .iter()
                    .filter(|s| s.ends_with(|c: char| c.is_ascii_digit()))
                    .count()
            })
    }

    /// Words either dictionary writes that have a pronunciation, most common first.
    fn words(&self) -> Vec<&str> {
        let mut words: Vec<&str> = self
            .gold
            .words()
            .into_iter()
            .filter(|w| self.pron.contains_key(*w))
            .collect();
        words.sort_by_key(|w| (self.rank.get(*w).copied().unwrap_or(usize::MAX), *w));
        words
    }
}

#[derive(Default, Clone, Copy)]
struct Tally {
    words: usize,
    written: usize,
    plover: usize,
    lapwing: usize,
    either: usize,
}

/// How many of the words the rules write the way Plover and Lapwing do, by syllables.
pub fn score(setup: &Setup, common: usize) -> Result<()> {
    let mut by_size: BTreeMap<usize, Tally> = BTreeMap::new();
    let mut total = Tally::default();
    for word in setup.words() {
        if setup.rank.get(word).copied().unwrap_or(usize::MAX) >= common {
            continue;
        }
        let candidates = setup.candidates(word);
        let (mut plover, mut lapwing) = (false, false);
        for outline in &candidates {
            let (p, l) = setup.gold.writes(word, outline);
            plover |= p;
            lapwing |= l;
        }
        let size = setup.syllable_count(word).min(4);
        for tally in [by_size.entry(size).or_default(), &mut total] {
            tally.words += 1;
            tally.written += usize::from(!candidates.is_empty());
            tally.plover += usize::from(plover);
            tally.lapwing += usize::from(lapwing);
            tally.either += usize::from(plover || lapwing);
        }
    }
    println!(
        "the {common} most common words that Plover or Lapwing write and CMUdict pronounces\n"
    );
    println!("syllables   words  written   plover  lapwing   either");
    let line = |label: String, t: Tally| {
        let pct = |n: usize| format!("{:>5.1}%", 100.0 * n as f64 / t.words.max(1) as f64);
        println!(
            "{label:<9} {:>7} {:>8} {:>8} {:>8} {:>8}",
            t.words,
            pct(t.written),
            pct(t.plover),
            pct(t.lapwing),
            pct(t.either)
        );
    };
    for (size, tally) in by_size {
        let label = if size == 4 {
            "4+".to_owned()
        } else {
            size.to_string()
        };
        line(label, tally);
    }
    line("all".into(), total);
    Ok(())
}

/// How the rules write a word, syllable by syllable, next to what Plover and Lapwing write.
pub fn explain(setup: &Setup, word: &str) -> Result<()> {
    let word = word.to_lowercase();
    let Some(variants) = setup.pron.get(&word) else {
        println!("{word}: no pronunciation in CMUdict");
        return Ok(());
    };
    for sounds in variants {
        println!("{word}  /{}/", sounds.join(" "));
        match setup.rules.syllables(&setup.rules.label(&word, sounds)) {
            None => println!("  no vowel"),
            Some(syllables) => {
                for syllable in &syllables {
                    let stroke = setup.rules.syllable_stroke(syllable);
                    println!(
                        "  [{}] {} [{}]  ->  {}",
                        syllable.onset.join(" "),
                        syllable.vowel,
                        syllable.coda.join(" "),
                        stroke.map_or("cannot be written in one stroke".to_owned(), |s| s
                            .to_string())
                    );
                }
            }
        }
    }
    let (plover, lapwing) = setup.gold.shortest(&word);
    println!("ours    {}", setup.candidates(&word).join("  "));
    println!(
        "plover  {}",
        setup
            .gold
            .plover
            .get(&word)
            .map_or(String::new(), |v| v.join("  "))
    );
    let _ = (plover, lapwing);
    println!(
        "lapwing {}",
        setup
            .gold
            .lapwing
            .get(&word)
            .map_or(String::new(), |v| v.join("  "))
    );
    Ok(())
}

/// The most common words the rules get wrong, with what Plover and Lapwing write.
pub fn mismatches(
    setup: &Setup,
    syllables: usize,
    limit: usize,
    missing: bool,
    after: usize,
) -> Result<()> {
    let mut shown = 0;
    for word in setup.words() {
        if setup.syllable_count(word) != syllables
            || setup.rank.get(word).copied().unwrap_or(usize::MAX) < after
        {
            continue;
        }
        let candidates = setup.candidates(word);
        let right = candidates.iter().any(|o| {
            let (p, l) = setup.gold.writes(word, o);
            p || l
        });
        if right || (missing && !candidates.is_empty()) {
            continue;
        }
        let (plover, lapwing) = setup.gold.shortest(word);
        let sounds = setup.pron[word]
            .first()
            .map(|s| s.join(" "))
            .unwrap_or_default();
        println!(
            "{word:<12} ours {:<18} plover {:<16} lapwing {:<16} {sounds}",
            candidates.first().map_or("-", String::as_str),
            plover.unwrap_or("-"),
            lapwing.unwrap_or("-"),
        );
        shown += 1;
        if shown == limit {
            break;
        }
    }
    Ok(())
}

/// For one-syllable words that Plover and Lapwing write the same way in one stroke, which keys
/// each group of sounds went on. This is where the rules come from.
pub fn learn(setup: &Setup, min: usize, limit: usize) -> Result<()> {
    type Counts = HashMap<String, HashMap<String, usize>>;
    let (mut onsets, mut rimes): (Counts, Counts) = (HashMap::new(), HashMap::new());
    for word in setup.words() {
        let (Some(plover), Some(lapwing)) = setup.gold.shortest(word) else {
            continue;
        };
        if plover != lapwing || plover.contains('/') {
            continue;
        }
        let Some(sounds) = setup.pron[word].first() else {
            continue;
        };
        let Some(syllables) = setup.rules.syllables(sounds) else {
            continue;
        };
        let [syllable] = syllables.as_slice() else {
            continue;
        };
        let Some(stroke) = Stroke::parse(plover) else {
            continue;
        };
        let left = Stroke(stroke.0 & Stroke::LEFT);
        let rest = Stroke(stroke.0 & !Stroke::LEFT);
        let base = syllable
            .vowel
            .trim_end_matches(|c: char| c.is_ascii_digit());
        let onset = syllable.onset.join(" ");
        let rime = std::iter::once(base)
            .chain(syllable.coda.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ");
        let shown = |s: Stroke| {
            if s.is_empty() {
                "(none)".to_owned()
            } else {
                s.to_string()
            }
        };
        *onsets
            .entry(onset)
            .or_default()
            .entry(shown(left))
            .or_default() += 1;
        *rimes
            .entry(rime)
            .or_default()
            .entry(shown(rest))
            .or_default() += 1;
    }
    for (title, counts) in [
        ("start of a syllable", onsets),
        ("vowel and the end of a syllable", rimes),
    ] {
        println!("== {title}: sounds -> keys (how many words)\n");
        let mut rows: Vec<(usize, String, Vec<(String, usize)>)> = counts
            .into_iter()
            .map(|(sounds, keys)| {
                let mut keys: Vec<_> = keys.into_iter().collect();
                keys.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                (keys.iter().map(|k| k.1).sum(), sounds, keys)
            })
            .filter(|row| row.0 >= min)
            .collect();
        rows.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        for (total, sounds, keys) in rows.into_iter().take(limit) {
            let top: Vec<String> = keys
                .iter()
                .take(4)
                .map(|(k, n)| format!("{k} ({n})"))
                .collect();
            println!("{sounds:<14} {total:>5}  {}", top.join("  "));
        }
        println!();
    }
    Ok(())
}
