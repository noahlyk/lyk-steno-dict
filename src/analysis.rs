//! Judging the rules against Plover and Lapwing, and learning which rules are missing.

use crate::data::{Pronunciations, load_cmudict, load_frequency_bounded};
use crate::generate;
use crate::gold::Gold;
use crate::rules::Rules;
use crate::stroke::{Stroke, render_outline};
use anyhow::{Context, Result};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
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
    /// `common` bounds how much of the frequency list is worth ranking at all: `None` for every
    /// command that needs a real rank for any word regardless of commonness (`explain`,
    /// `learn`, `mismatches`), `Some(n)` for one that already only looks at the `n` most common
    /// words anyway (`build`, `score`, `quote`, `resolve`), padded past `n` so nothing a caller
    /// still reads near that edge comes up unranked that would not have been before.
    pub fn load(data: &Path, rules: Option<&Path>, common: Option<usize>) -> Result<Self> {
        let rules = match rules {
            Some(path) => Rules::load(path)?,
            None => Rules::built_in()?,
        };
        let freq_limit =
            common.map_or(usize::MAX, |c| c.saturating_add(c / 5).max(c.saturating_add(10_000)));
        let freq_path = data.join("frequency/en_full.txt");
        let pron_path = data.join("pronunciation/cmudict.dict");
        // Three independent files, none needed before another: load them on their own threads
        // instead of one after another.
        let (frequency, pron, gold) = std::thread::scope(|scope| {
            let frequency = scope.spawn(|| load_frequency_bounded(&freq_path, freq_limit));
            let pron = scope.spawn(|| load_cmudict(&pron_path));
            let gold = scope.spawn(|| Gold::load(data));
            (
                frequency.join().expect("frequency load panicked"),
                pron.join().expect("pronunciation load panicked"),
                gold.join().expect("gold dictionary load panicked"),
            )
        });
        let mut rank = HashMap::new();
        let mut count = HashMap::new();
        for (n, (word, times)) in frequency?.into_iter().enumerate() {
            count.entry(word.clone()).or_insert(times);
            rank.entry(word).or_insert(n);
        }
        Ok(Self {
            rules,
            pron: pron?,
            gold: gold?,
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
    /// Words with both a generated outline and a Plover one, counted toward the keys/stroke
    /// averages below (not every word has both, so this is its own count, not `words`)
    costed: usize,
    this_strokes: f64,
    this_keys: f64,
    plover_strokes: f64,
    plover_keys: f64,
    lapwing_strokes: f64,
    lapwing_keys: f64,
    /// Same as `this_strokes`/`this_keys`/`plover_strokes`/`plover_keys`/`lapwing_strokes`/
    /// `lapwing_keys`, weighted by how often each word is actually said (`Setup::count`): a word
    /// nobody says barely moves this, where it counts the same as "the" above. Closer to what
    /// typing each theory really feels like than the per-word average is.
    this_strokes_weighted: f64,
    this_keys_weighted: f64,
    plover_strokes_weighted: f64,
    plover_keys_weighted: f64,
    lapwing_strokes_weighted: f64,
    lapwing_keys_weighted: f64,
}

/// `score`'s and `check_readme`'s shared tally: by syllable count, then "all" last.
fn tallies(setup: &Setup, briefs: &generate::Briefs, common: usize) -> Result<Vec<(String, Tally)>> {
    // The real, brief-adjusted outline `build` would write for a word, cheapest first: matches
    // what ships, not just what the bare rules produce.
    let generated = generate::generate(setup, briefs, common)?;
    let mut ours: HashMap<&str, (usize, u32)> = HashMap::new();
    for entry in &generated.entries {
        let cost = (entry.outline.len(), generate::keys(&entry.outline));
        let best = ours.entry(entry.word.as_str()).or_insert(cost);
        *best = (*best).min(cost);
    }
    let plover_cost = |word: &str| -> Option<(usize, u32)> {
        let outline = generate::shortest(setup.gold.plover.get(word)?)?;
        Some((outline.len(), generate::keys(&outline)))
    };
    let lapwing_cost = |word: &str| -> Option<(usize, u32)> {
        let outline = generate::shortest(setup.gold.lapwing.get(word)?)?;
        Some((outline.len(), generate::keys(&outline)))
    };

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
        let cost = ours.get(word).zip(plover_cost(word));
        let lcost = ours.get(word).zip(lapwing_cost(word));
        let size = setup.syllable_count(word).min(4);
        for tally in [by_size.entry(size).or_default(), &mut total] {
            tally.words += 1;
            tally.written += usize::from(!candidates.is_empty());
            tally.plover += usize::from(plover);
            tally.lapwing += usize::from(lapwing);
            tally.either += usize::from(plover || lapwing);
            let weight = setup.count.get(word).copied().unwrap_or(0) as f64;
            if let Some((&(this_strokes, this_keys), (plover_strokes, plover_keys))) = cost {
                tally.costed += 1;
                tally.this_strokes += this_strokes as f64;
                tally.this_keys += f64::from(this_keys);
                tally.plover_strokes += plover_strokes as f64;
                tally.plover_keys += f64::from(plover_keys);
                tally.this_strokes_weighted += weight * this_strokes as f64;
                tally.this_keys_weighted += weight * f64::from(this_keys);
                tally.plover_strokes_weighted += weight * plover_strokes as f64;
                tally.plover_keys_weighted += weight * f64::from(plover_keys);
            }
            if let Some((&(_, _), (lapwing_strokes, lapwing_keys))) = lcost {
                tally.lapwing_strokes += lapwing_strokes as f64;
                tally.lapwing_keys += f64::from(lapwing_keys);
                tally.lapwing_strokes_weighted += weight * lapwing_strokes as f64;
                tally.lapwing_keys_weighted += weight * f64::from(lapwing_keys);
            }
        }
    }
    let mut rows: Vec<(String, Tally)> = by_size
        .into_iter()
        .map(|(size, tally)| {
            let label = if size == 4 {
                "4+".to_owned()
            } else {
                size.to_string()
            };
            (label, tally)
        })
        .collect();
    rows.push(("all".to_owned(), total));
    Ok(rows)
}

/// How many of the words the rules write the way Plover and Lapwing do, by syllables, and how
/// many keys each syllable count takes per stroke, lyk next to Plover and Lapwing.
pub fn score(setup: &Setup, briefs: &generate::Briefs, common: usize) -> Result<()> {
    let rows = tallies(setup, briefs, common)?;
    println!(
        "the {common} most common words that Plover or Lapwing write and CMUdict pronounces\n"
    );
    println!(
        "syllables   words  written   plover  lapwing   either  keys/stroke: this  plover  \
         lapwing  by usage: this  plover  lapwing"
    );
    for (label, t) in rows {
        let pct = |n: usize| format!("{:>5.1}%", 100.0 * n as f64 / t.words.max(1) as f64);
        let per_stroke = |keys: f64, strokes: f64| keys / strokes.max(1.0);
        println!(
            "{label:<9} {:>7} {:>8} {:>8} {:>8} {:>8}  {:>17.3} {:>7.3} {:>7.3}  {:>9.3} \
             {:>7.3} {:>7.3}",
            t.words,
            pct(t.written),
            pct(t.plover),
            pct(t.lapwing),
            pct(t.either),
            per_stroke(t.this_keys, t.this_strokes),
            per_stroke(t.plover_keys, t.plover_strokes),
            per_stroke(t.lapwing_keys, t.lapwing_strokes),
            per_stroke(t.this_keys_weighted, t.this_strokes_weighted),
            per_stroke(t.plover_keys_weighted, t.plover_strokes_weighted),
            per_stroke(t.lapwing_keys_weighted, t.lapwing_strokes_weighted),
        );
    }
    Ok(())
}

/// The README's basic syllable table, rendered fresh from `tallies` as the exact markdown it
/// should contain between the `<!-- lyk:score-table:start/end -->` markers.
fn score_markdown(setup: &Setup, briefs: &generate::Briefs, common: usize) -> Result<String> {
    let rows = tallies(setup, briefs, common)?;
    let pct = |n: usize, of: usize| format!("{:.0}%", 100.0 * n as f64 / of.max(1) as f64);
    let per_stroke = |keys: f64, strokes: f64| format!("{:.2}", keys / strokes.max(1.0));
    let mut out = String::new();
    out.push_str(
        "| Syllables | Same as Plover or Lapwing | Plover | Lapwing | Keys/Stroke (this) | \
         Keys/Stroke (Plover) | Keys/Stroke (Lapwing) |\n",
    );
    out.push_str("|---|---|---|---|---|---|---|\n");
    for (label, t) in &rows {
        // "0" is a handful of pronunciation-lookup artifacts (no real syllable), and "all" is
        // its own summary line; the README table is just the four real buckets.
        if label == "all" || label == "0" {
            continue;
        }
        writeln!(
            out,
            "| {label} | {} | {} | {} | {} | {} | {} |",
            pct(t.either, t.words),
            pct(t.plover, t.words),
            pct(t.lapwing, t.words),
            per_stroke(t.this_keys, t.this_strokes),
            per_stroke(t.plover_keys, t.plover_strokes),
            per_stroke(t.lapwing_keys, t.lapwing_strokes),
        )?;
    }
    Ok(out)
}

/// The README's usage-weighted table: the same keys/stroke figure as [`score_markdown`], but
/// weighted by how often each word is actually said, for all three theories side by side (not
/// just this one) — rendered fresh between the `<!-- lyk:usage-table:start/end -->` markers.
fn usage_markdown(setup: &Setup, briefs: &generate::Briefs, common: usize) -> Result<String> {
    let rows = tallies(setup, briefs, common)?;
    let per_stroke = |keys: f64, strokes: f64| format!("{:.2}", keys / strokes.max(1.0));
    let mut out = String::new();
    out.push_str(
        "| Syllables | Keys/Stroke (this, by usage) | Keys/Stroke (Plover, by usage) | \
         Keys/Stroke (Lapwing, by usage) |\n",
    );
    out.push_str("|---|---|---|---|\n");
    for (label, t) in &rows {
        if label == "all" || label == "0" {
            continue;
        }
        writeln!(
            out,
            "| {label} | {} | {} | {} |",
            per_stroke(t.this_keys_weighted, t.this_strokes_weighted),
            per_stroke(t.plover_keys_weighted, t.plover_strokes_weighted),
            per_stroke(t.lapwing_keys_weighted, t.lapwing_strokes_weighted),
        )?;
    }
    Ok(out)
}

const README_START: &str = "<!-- lyk:score-table:start -->";
const README_END: &str = "<!-- lyk:score-table:end -->";
const USAGE_START: &str = "<!-- lyk:usage-table:start -->";
const USAGE_END: &str = "<!-- lyk:usage-table:end -->";

/// The text strictly between a `start`/`end` marker pair in `text`.
fn between<'a>(text: &'a str, start: &str, end: &str) -> Result<&'a str> {
    let (_before, rest) = text
        .split_once(start)
        .with_context(|| format!("{start} not found"))?;
    let (current, _after) = rest
        .split_once(end)
        .with_context(|| format!("{end} not found"))?;
    Ok(current)
}

/// Whether the README's two score tables still match what `score` would compute today. Prints
/// the up-to-date tables and returns an error (so CI or a habit of running this before a commit
/// catches drift) when either doesn't; each table's own markers decide what gets compared, so
/// the surrounding prose is never touched by this check (or by anything that would write it).
pub fn check_readme(
    setup: &Setup,
    briefs: &generate::Briefs,
    common: usize,
    readme: &Path,
) -> Result<()> {
    let fresh_score = score_markdown(setup, briefs, common)?;
    let fresh_usage = usage_markdown(setup, briefs, common)?;
    let text = std::fs::read_to_string(readme).with_context(|| readme.display().to_string())?;
    let current_score = between(&text, README_START, README_END)
        .with_context(|| format!("in {}", readme.display()))?;
    let current_usage = between(&text, USAGE_START, USAGE_END)
        .with_context(|| format!("in {}", readme.display()))?;
    let score_ok = current_score.trim() == fresh_score.trim();
    let usage_ok = current_usage.trim() == fresh_usage.trim();
    if score_ok && usage_ok {
        println!("both score tables match: {}", readme.display());
        return Ok(());
    }
    if !score_ok {
        println!(
            "{} has a stale score table between {README_START} and {README_END}. it should \
             read:\n",
            readme.display()
        );
        println!("{fresh_score}");
    }
    if !usage_ok {
        println!(
            "{} has a stale usage table between {USAGE_START} and {USAGE_END}. it should \
             read:\n",
            readme.display()
        );
        println!("{fresh_usage}");
    }
    anyhow::bail!(
        "{} has a stale table — paste the table(s) above over what's there now",
        readme.display()
    );
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
    for step in setup.rules.brevity() {
        let forms: Vec<String> = variants
            .iter()
            .flat_map(|sounds| setup.rules.shorten(*step, &word, sounds))
            .map(|o| render_outline(&o))
            .collect();
        if !forms.is_empty() {
            let vowel = if step.vowel { "" } else { " (no vowel)" };
            println!("  {}{vowel}: {}", step.step, forms.join("  "));
        }
    }
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
