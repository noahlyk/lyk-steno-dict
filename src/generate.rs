//! Putting the dictionary together: briefs first, then the rules, with clashes settled in a fixed
//! order so the result is predictable.

use crate::analysis::Setup;
use crate::rules::Step;
use crate::stroke::{Stroke, parse_outline, render_outline};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

const BUILT_IN_BRIEFS: &str = include_str!("../rules/briefs.toml");
const BUILT_IN_SUFFIXES: &str = include_str!("../rules/suffixes.toml");

/// Every stroke that isn't a word at all: the commands, punctuation, and fingerspelling/OS-key
/// dictionaries deployed alongside the generated dictionary in a real keymux config. These load
/// at *higher* priority than the generated words there, so a word that happens to land on the
/// same stroke as one of these is never reachable — a real stroke, that was never a bug in the
/// deployed config, just unknown to the generator. Reserved the same way a brief's outline is
/// (see `generate`), so the collision can't resurface on the next regeneration.
const RESERVED: &[(&str, &str)] = &[
    ("a command", include_str!("../rules/reserved/commands.json")),
    ("a punctuation mark", include_str!("../rules/reserved/punctuation.json")),
    ("a fingerspelled letter or OS shortcut", include_str!("../rules/reserved/emilys-modifiers.json")),
];

/// A brief this short was hand-picked by a theory's authors, not derived from frequency: see
/// [`generate`].
const TINY_BRIEF_KEYS: u32 = 2;

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
    /// A standalone suffix: a continuation stroke that stands for it on its own, not folded onto
    /// any particular word (see `rules/suffixes.toml`)
    Suffix,
    Brief,
    Rules,
    /// A long word written shorter by one of the shortening steps
    Short(Step),
    /// The spelling told it from a word that sounds the same
    Spelling,
    /// Another pronunciation of a word
    Alternate,
    /// The ladder step that settled a clash: 1 is the star on the first stroke
    Ladder(u8),
}

pub struct Entry {
    pub word: String,
    pub outline: Vec<Stroke>,
    pub origin: Origin,
    /// The word that has the outline this one wanted
    pub lost_to: Option<String>,
}

pub struct Generated {
    pub entries: Vec<Entry>,
    pub unresolved: Vec<String>,
}

/// The shortest outline (fewest strokes, then fewest keys) from a list.
pub fn shortest(outlines: &[String]) -> Option<Vec<Stroke>> {
    outlines
        .iter()
        .filter_map(|o| parse_outline(o))
        .min_by_key(|o| (o.len(), keys(o)))
}

/// Among a word's candidate outlines, the one a brief should actually use: fewest strokes, but
/// preferring one whose first and last consonants read, under lyk's own rules, as the word's real
/// first and last sounds, before falling back to fewest keys. Plover and Lapwing's own shortest
/// outline for a word is sometimes their own theory's memorized one-off trick, not derived from
/// either theory's general rules — which can mean only part of a two-key consonant is actually
/// left in the stroke (`good`'s Plover brief `KPWAOD` drops the `T` of `TKPW`'s own `G`, `work`'s
/// `WORB` drops the `G` of `-BG`'s own `K`). Where that happens, two repairs are tried alongside
/// the original candidates, not just one: completing the fragment into the word's whole, correct
/// sound (what used to be the only fallback), and dropping the fragment outright, since a brief
/// is always free to leave out a sound it isn't attempting at all — only holding *part* of one is
/// disallowed. Dropping is usually the fewer keys of the two, so it's what wins when it's free;
/// both are only ever offered when the word's own gold dictionary pointed at the sound to begin
/// with, never invented from nothing.
fn best_brief(
    outlines: &[String],
    rules: &crate::rules::Rules,
    word: &str,
    sounds: &[String],
    natural: &HashMap<String, &str>,
) -> Option<Vec<Stroke>> {
    // Whether a candidate is one of the word's own gold outlines (never second-guessed by a
    // collision below — a hand-picked Plover brief like "and"'s `SKP` is allowed to sit on
    // whatever some other, unrelated word would have wanted by the plain rules; that trade is
    // this whole mechanism's normal, accepted cost, not a bug) or this function's own synthesized
    // drop repair (which is).
    let candidates: Vec<(bool, Vec<Stroke>)> = outlines
        .iter()
        .filter_map(|o| parse_outline(o))
        .flat_map(|o| {
            let repaired = rules.drop_fragments(&o, word, sounds).map(|d| (true, d));
            std::iter::once((false, o)).chain(repaired)
        })
        .collect();
    candidates
        .into_iter()
        .min_by_key(|(dropped, o)| {
            // A *dropped* sound that happens to land exactly on a different word's own plain
            // outline isn't a saving: it just hands that other word a ladder fight it would never
            // otherwise have had, almost always a shorter, commoner word than the one being
            // briefed (`fact` dropping its own `K` entirely used to take `fat`'s rightful,
            // un-briefed `TPAT`). Only the synthesized drop candidate is checked — a raw gold
            // outline colliding with another word is the ordinary, already-accepted case above,
            // not this one. Checked after the key-count/fragment comparisons that matter when
            // there's no such collision, so a candidate only pays this cost when it would
            // otherwise have won anyway.
            let collides = *dropped && natural.get(&render_outline(o)).is_some_and(|&owner| owner != word);
            (
                o.len(),
                !rules.matches_real_consonants(o, word, sounds),
                collides,
                keys(o),
            )
        })
        .map(|(_, o)| o)
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

    // Reserved first of all, ahead of even the suffixes: a stroke a real keymux config already
    // gives to a command, a punctuation mark, or fingerspelling is never available to a word,
    // whatever the rules would have generated for it.
    for (kind, json) in RESERVED {
        let strokes: HashMap<String, String> = serde_json::from_str(json)
            .with_context(|| format!("rules/reserved: {kind} dictionary"))?;
        for outline_text in strokes.keys() {
            let Some(outline) = parse_outline(outline_text) else {
                continue;
            };
            let text = render_outline(&outline);
            taken.entry(text).or_insert_with(|| format!("reserved ({kind})"));
        }
    }

    // Standalone suffixes, first of all: a word that would otherwise land on the same outline
    // as one of these loses to it, the same way a brief does, since a suffix is struck on its
    // own far more often than any one rare word would want that exact outline.
    let suffixes: BTreeMap<String, String> = toml::from_str(BUILT_IN_SUFFIXES)?;
    for (outline_text, word) in &suffixes {
        let outline = parse_outline(outline_text)
            .with_context(|| format!("{outline_text:?} in rules/suffixes.toml"))?;
        let text = render_outline(&outline);
        if let Some(holder) = taken.get(&text) {
            anyhow::bail!("suffix {word:?} wants the outline {holder:?} already has");
        }
        taken.insert(text, word.clone());
        entries.push(Entry {
            word: word.clone(),
            outline,
            origin: Origin::Suffix,
            lost_to: None,
        });
    }

    // The pronoun "I": the frequency list only ever has it lowercase, indistinguishable from
    // the letter name "i" ("spell that: i, c, e"), and the letter already won that one word's
    // only slot (see the single-letter carve-out below). A capitalized word needs an entry of
    // its own regardless, so it gets one directly, on Plover's own real convention for it.
    let outline = parse_outline("EU").context("\"EU\" for the pronoun \"I\"")?;
    let text = render_outline(&outline);
    if let Some(holder) = taken.get(&text) {
        anyhow::bail!("the pronoun \"I\" wants the outline {text:?} {holder:?} already has");
    }
    taken.insert(text, "I".to_string());
    entries.push(Entry {
        word: "I".to_string(),
        outline,
        origin: Origin::Brief,
        lost_to: None,
    });

    // Words, most common first
    let mut words: Vec<(&String, usize)> = setup
        .rank
        .iter()
        .filter(|(w, r)| {
            // A single letter is almost always noise in a frequency list (a stray scan
            // artifact, a placeholder), except the two that are real English words
            **r < common
                && setup.pron.contains_key(w.as_str())
                && (w.len() >= 2 || w.as_str() == "a" || w.as_str() == "i")
                && w.bytes().all(|b| b.is_ascii_lowercase() || b == b'\'')
        })
        .map(|(w, r)| (w, *r))
        .collect();
    words.sort_by_key(|(w, r)| (*r, w.as_str()));

    // Briefs: the common words Plover or Lapwing already write, plus any word either of them
    // writes in two keys or fewer, however rare: a brief that short was hand-picked, not derived
    // from frequency, so frequency is not what should gate keeping it. A word named explicitly in
    // `[word]` is a third way in, the same reasoning extended past "two keys or fewer" to "someone
    // looked at this specific word and decided its brief was worth keeping": the frequency window
    // is a default for words nobody has looked at individually, not a ceiling on them.
    let has_tiny_brief = |outlines: Option<&Vec<String>>| {
        outlines.and_then(|o| shortest(o)).is_some_and(|o| keys(&o) <= TINY_BRIEF_KEYS)
    };

    // A dry run of the brief loop below, exactly as the original algorithm picks it (no collision
    // awareness: an empty `natural` map never flags anything as colliding), to find which words
    // actually end up relying on their own plain outline because nothing better came of their own
    // brief attempt. Only those words are worth a different word paying extra keys to avoid
    // colliding with below — a word that lands a brief of its own regardless (`a`, pinned to
    // `AEU`) never needs its plain reading (`A`) at all, so it shouldn't block some other word
    // from cleanly dropping a sound down to that same text (see `fact`'s own drop candidate `TPAT`
    // falsely "colliding" with `a`'s unused plain reading before this check existed).
    let empty_natural: HashMap<String, &str> = HashMap::new();
    let mut baseline_taken = taken.clone();
    let mut baseline_pinned: HashSet<&str> = HashSet::new();
    for (word, rank) in &words {
        let plover = setup.gold.plover.get(word.as_str());
        let lapwing = setup.gold.lapwing.get(word.as_str());
        let named = briefs.word.contains_key(word.as_str());
        if *rank >= briefs.count && !has_tiny_brief(plover) && !has_tiny_brief(lapwing) && !named {
            continue;
        }
        let sounds = &setup.pron[word.as_str()][0];
        let source = briefs.word.get(word.as_str()).unwrap_or(&briefs.default);
        let outline = match source.as_str() {
            "plover" => plover.and_then(|o| best_brief(o, &setup.rules, word, sounds, &empty_natural)),
            "lapwing" => lapwing.and_then(|o| best_brief(o, &setup.rules, word, sounds, &empty_natural)),
            explicit => parse_outline(explicit),
        };
        let outline = outline
            .or_else(|| {
                setup
                    .gold
                    .plover
                    .get(word.as_str())
                    .and_then(|o| best_brief(o, &setup.rules, word, sounds, &empty_natural))
            })
            .or_else(|| {
                setup
                    .gold
                    .lapwing
                    .get(word.as_str())
                    .and_then(|o| best_brief(o, &setup.rules, word, sounds, &empty_natural))
            });
        let Some(outline) = outline else { continue };
        let text = render_outline(&outline);
        if baseline_taken.contains_key(&text) {
            continue;
        }
        baseline_taken.insert(text, (*word).clone());
        baseline_pinned.insert(word.as_str());
    }

    // Every common-enough word's own outline by the plain rules, so a brief below can tell a
    // genuine saving from one that only moves a collision onto a word a learner would also know
    // (see `best_brief`). Limited to words inside the same "commonly known" window a brief itself
    // uses (`briefs.count`) that the dry run above shows are NOT themselves landing a brief:
    // losing a plain outline to a brief is this whole mechanism's normal, accepted cost for a rare
    // word (that's what the ladder is for), so only a collision with a word that actually needs
    // this outline is worth a brief paying extra keys to avoid. The commonest word keeps a
    // contested outline, since `words` is already sorted by rank.
    let mut natural: HashMap<String, &str> = HashMap::new();
    for (word, rank) in &words {
        if *rank >= briefs.count || baseline_pinned.contains(word.as_str()) {
            continue;
        }
        if let Some(outline) = setup.pron[word.as_str()]
            .iter()
            .find_map(|sounds| setup.rules.outline(word, sounds))
        {
            natural.entry(render_outline(&outline)).or_insert(word.as_str());
        }
    }

    for (word, rank) in &words {
        let plover = setup.gold.plover.get(word.as_str());
        let lapwing = setup.gold.lapwing.get(word.as_str());
        let named = briefs.word.contains_key(word.as_str());
        if *rank >= briefs.count && !has_tiny_brief(plover) && !has_tiny_brief(lapwing) && !named {
            continue;
        }
        let sounds = &setup.pron[word.as_str()][0];
        let source = briefs.word.get(word.as_str()).unwrap_or(&briefs.default);
        let outline = match source.as_str() {
            "plover" => plover.and_then(|o| best_brief(o, &setup.rules, word, sounds, &natural)),
            "lapwing" => lapwing.and_then(|o| best_brief(o, &setup.rules, word, sounds, &natural)),
            explicit => {
                let outline = parse_outline(explicit)
                    .with_context(|| format!("{word:?}'s brief {explicit:?} in rules/briefs.toml"))?;
                let text = render_outline(&outline);
                // A literal outline in `[word]` was looked at and picked by hand, unlike a
                // `plover`/`lapwing` source's own collision-avoiding `best_brief` call above: if
                // it's already spoken for, that is always a stale or wrong entry worth fixing in
                // `rules/briefs.toml`, never something to fall back from silently. `up`'s own
                // `"-P"` sat this way for a long time, unnoticed, quietly losing every build to
                // `rules/reserved/punctuation.json`'s own `"-P"` and falling back to the
                // two-key `UP` the rules generate anyway, with nothing printed to say so.
                if let Some(holder) = taken.get(&text) {
                    anyhow::bail!(
                        "{word:?}'s brief ({explicit:?} in rules/briefs.toml) wants the outline \
                         {text:?}, which {holder:?} already has"
                    );
                }
                Some(outline)
            }
        };
        // A word only one of them writes still gets its brief
        let outline = outline
            .or_else(|| {
                setup
                    .gold
                    .plover
                    .get(word.as_str())
                    .and_then(|o| best_brief(o, &setup.rules, word, sounds, &natural))
            })
            .or_else(|| {
                setup
                    .gold
                    .lapwing
                    .get(word.as_str())
                    .and_then(|o| best_brief(o, &setup.rules, word, sounds, &natural))
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
            lost_to: None,
        });
    }

    let join = setup.rules.join_key();
    let mut unresolved = Vec::new();

    // Words that sound the same and would have the same outline: the spelling decides which one
    // moves to its own, before frequency decides anything
    let usual = |word: &str| {
        setup.pron[word]
            .iter()
            .find_map(|sounds| setup.rules.outline(word, sounds))
    };
    let mut groups: HashMap<String, Vec<&str>> = HashMap::new();
    for (word, _) in &words {
        if let Some(outline) = usual(word) {
            groups
                .entry(render_outline(&outline))
                .or_default()
                .push(word.as_str());
        }
    }
    // The outlines that one-syllable words have by the rules: the ones a learner would work out
    let mut simple: HashSet<&String> = HashSet::new();
    for (text, members) in &groups {
        let one_syllable = |w: &&str| {
            setup.pron[*w][0]
                .iter()
                .filter(|s| s.ends_with(|c: char| c.is_ascii_digit()))
                .count()
                == 1
        };
        if members.iter().any(one_syllable) {
            simple.insert(text);
        }
    }
    // The outlines some word has by the rules
    let wanted: HashSet<&String> = groups.keys().collect();
    let mut spelled: HashSet<String> = HashSet::new();
    // Words whose outline another word has too, or a brief has already: the commonest first, so
    // `heal` gets HAEL before `hale` asks for it
    let mut clashing: Vec<&str> = groups
        .iter()
        .filter(|(text, members)| members.len() > 1 || taken.contains_key(text.as_str()))
        .flat_map(|(_, members)| members.iter().copied())
        .filter(|w| !pinned.contains(*w))
        .collect();
    clashing.sort_by_key(|w| (setup.rank[*w], *w));
    {
        for word in &clashing {
            let Some(mut outline) = setup.pron[*word]
                .iter()
                .find_map(|sounds| setup.rules.variant(word, sounds))
            else {
                continue;
            };
            setup.rules.joined(&mut outline);
            let text = render_outline(&outline);
            if taken.contains_key(&text) || wanted.contains(&text) {
                continue;
            }
            taken.insert(text, (*word).to_owned());
            spelled.insert((*word).to_owned());
            entries.push(Entry {
                word: (*word).to_owned(),
                outline,
                origin: Origin::Spelling,
                lost_to: None,
            });
        }
    }
    for (word, _) in &words {
        if spelled.contains(word.as_str()) {
            continue;
        }
        // A word with a brief keeps it as its only entry: one outline, one way to learn it, and
        // whatever its rule outline or short forms would have been stays free for some other
        // word to claim instead. Words are handled once, in rank order, with `taken` checked at
        // the moment each one claims an outline, so never claiming these on a briefed word's
        // behalf is enough on its own for a later, less common word to pick them up as its own —
        // no separate pass is needed.
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
        let mut lost_to: Option<String> = None;
        // The commonest long words go first for the fewest keys: the shortening steps, in order.
        // Only where they save a stroke; the outline by the rules is added as well. Each short
        // form is a group of its plain and starred forms, of which the first free one is used.
        let mut options: Vec<(Origin, Vec<Vec<Stroke>>)> = Vec::new();
        let own = setup.rank[word.as_str()];
        for short in setup
            .rules
            .brevity()
            .iter()
            .filter(|s| own < s.rank && first.len() > 1)
        {
            for sounds in &setup.pron[word.as_str()] {
                for outline in setup.rules.shorten(*short, word, sounds) {
                    if outline.len() >= first.len() {
                        continue;
                    }
                    // Never an outline some word has by the rules: those stay theirs. When the
                    // form is taken, a star on it still beats a whole extra stroke.
                    let starred: Vec<Vec<Stroke>> = ladder(&outline, join)
                        .into_iter()
                        .filter(|(n, _)| *n <= 2)
                        .map(|(_, o)| o)
                        .collect();
                    let group: Vec<Vec<Stroke>> = std::iter::once(outline)
                        .chain(starred)
                        .filter(|outline| {
                            let text = render_outline(outline);
                            !simple.contains(&text)
                                && !wanted.contains(&text)
                                && !options.iter().any(|(_, g)| g.contains(outline))
                        })
                        .collect();
                    if !group.is_empty() {
                        options.push((Origin::Short(short.step), group));
                    }
                }
            }
        }
        options.push((Origin::Rules, vec![first.clone()]));
        options.extend(
            ladder(first, join)
                .into_iter()
                .map(|(n, o)| (Origin::Ladder(n), vec![o])),
        );
        for (origin, group) in options {
            // Every short form that is free is added: the dictionary has all of a word's forms,
            // and the cheapest counts. A rule or ladder outline settles the word.
            if placed && !matches!(origin, Origin::Short(_)) {
                break;
            }
            for mut outline in group {
                setup.rules.joined(&mut outline);
                let text = render_outline(&outline);
                if let Some(holder) = taken.get(&text) {
                    // Who has the outline the rules give this word (not one of its short forms)
                    if origin == Origin::Rules {
                        lost_to = Some(holder.clone());
                    }
                    continue;
                }
                taken.insert(text, (*word).clone());
                let lost_to = if matches!(origin, Origin::Ladder(_)) {
                    lost_to.clone()
                } else {
                    None
                };
                entries.push(Entry {
                    word: (*word).clone(),
                    outline,
                    origin,
                    lost_to,
                });
                if !matches!(origin, Origin::Short(_)) {
                    placed = true;
                }
                break;
            }
        }
        if !placed {
            unresolved.push((*word).clone());
        }
        // Other pronunciations are only added where they clash with nothing
        for outline in others {
            let mut outline = outline.clone();
            setup.rules.joined(&mut outline);
            let text = render_outline(&outline);
            if !taken.contains_key(&text) {
                taken.insert(text, (*word).clone());
                entries.push(Entry {
                    word: (*word).clone(),
                    outline,
                    origin: Origin::Alternate,
                    lost_to: None,
                });
            }
        }
    }
    Ok(Generated {
        entries,
        unresolved,
    })
}

/// Each word's cheapest outline (fewest strokes, then fewest keys) from a generated dictionary.
pub fn cheapest(generated: &Generated) -> HashMap<&str, &[Stroke]> {
    let mut best: HashMap<&str, &[Stroke]> = HashMap::new();
    for entry in &generated.entries {
        let cost = |o: &[Stroke]| (o.len(), keys(o));
        match best.get(entry.word.as_str()) {
            Some(current) if cost(current) <= cost(&entry.outline) => {}
            _ => {
                best.insert(&entry.word, &entry.outline);
            }
        }
    }
    best
}

/// Every outline in a generated dictionary, by its rendered text, to the word it writes.
pub fn index(generated: &Generated) -> HashMap<String, &str> {
    generated
        .entries
        .iter()
        .map(|e| (render_outline(&e.outline), e.word.as_str()))
        .collect()
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
pub fn build(
    setup: &Setup,
    briefs: &Briefs,
    common: usize,
    out: &Path,
    against: Option<&(String, HashMap<String, String>)>,
) -> Result<()> {
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
    let sample: Vec<String> = generated
        .unresolved
        .iter()
        .take(40)
        .map(|w| format!("{w} ({})", setup.pron[w.as_str()][0].join(" ")))
        .collect();
    println!("    the commonest: {}", sample.join(", "));

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
    // Another dictionary to compare with, by its own shortest outline for each word
    let other_cost = |word: &str| -> Option<(usize, u32)> {
        let outline = parse_outline(against?.1.get(word)?)?;
        Some((outline.len(), keys(&outline)))
    };
    let mut total = [0f64; 9];
    // The same, by how common the word is: where the keys go
    const BANDS: [(usize, &str); 5] = [
        (100, "top 100"),
        (1_000, "100-1k"),
        (5_000, "1k-5k"),
        (20_000, "5k-20k"),
        (usize::MAX, "20k+"),
    ];
    let mut bands = [[0f64; 5]; BANDS.len()];
    let mut worse: Vec<(f64, &str, u32, u32)> = Vec::new();
    for (word, &(strokes, key_count)) in &ours {
        let (Some(plover), Some(lapwing)) = (
            cost_of(setup.gold.plover.get(*word)),
            cost_of(setup.gold.lapwing.get(*word)),
        ) else {
            continue;
        };
        let other = other_cost(word);
        if against.is_some() && other.is_none() {
            continue;
        }
        let other = other.unwrap_or_default();
        let weight = setup.count.get(*word).copied().unwrap_or(0) as f64;
        total[0] += weight;
        total[1] += weight * strokes as f64;
        total[2] += weight * f64::from(key_count);
        total[3] += weight * plover.0 as f64;
        total[4] += weight * f64::from(plover.1);
        total[5] += weight * lapwing.0 as f64;
        total[6] += weight * f64::from(lapwing.1);
        total[7] += weight * other.0 as f64;
        total[8] += weight * f64::from(other.1);
        let rank = setup.rank.get(*word).copied().unwrap_or(usize::MAX);
        let band = BANDS.iter().position(|(to, _)| rank < *to).unwrap_or(0);
        bands[band][0] += weight;
        bands[band][1] += weight * f64::from(key_count);
        bands[band][2] += weight * f64::from(plover.1);
        bands[band][3] += weight * f64::from(lapwing.1);
        bands[band][4] += weight * f64::from(other.1);
        if key_count > plover.1 {
            worse.push((
                weight * f64::from(key_count - plover.1),
                word,
                key_count,
                plover.1,
            ));
        }
    }
    let other_name = against.map_or("", |(name, _)| name.as_str());
    println!("\nwhat a word costs on average, weighted by how often it is said");
    println!("(words all the dictionaries write; strokes and keys pressed per word)");
    println!("            strokes     keys");
    let mut rows = vec![
        ("this", total[1], total[2]),
        ("plover", total[3], total[4]),
        ("lapwing", total[5], total[6]),
    ];
    if against.is_some() {
        rows.push((other_name, total[7], total[8]));
    }
    for (name, strokes, key_count) in rows {
        println!(
            "  {name:<9} {:>7.3} {:>8.3}",
            strokes / total[0],
            key_count / total[0]
        );
    }
    println!("\nkeys per word by how common the word is, and the share of all words said");
    println!("            share     this   plover  lapwing  {other_name}");
    for ((_, name), band) in BANDS.iter().zip(&bands) {
        if band[0] == 0.0 {
            continue;
        }
        let other = if against.is_some() {
            format!("  {:>7.3}", band[4] / band[0])
        } else {
            String::new()
        };
        println!(
            "  {name:<9} {:>5.1}%  {:>7.3}  {:>7.3}  {:>7.3}{other}",
            100.0 * band[0] / total[0],
            band[1] / band[0],
            band[2] / band[0],
            band[3] / band[0]
        );
    }
    println!("\nthe commonest words that lost their outline to a word that sounds the same");
    let mut lost: Vec<&Entry> = generated
        .entries
        .iter()
        .filter(|e| e.lost_to.is_some() && matches!(e.origin, Origin::Ladder(_)))
        .collect();
    lost.sort_by_key(|e| setup.rank.get(&e.word).copied().unwrap_or(usize::MAX));
    for entry in lost.iter().take(30) {
        let holder = entry.lost_to.as_deref().unwrap_or("");
        let theirs = generated
            .entries
            .iter()
            .find(|e| e.word == holder)
            .map_or_else(String::new, |e| render_outline(&e.outline));
        println!(
            "  {:<12} {:<18} (the outline of {holder}: {theirs})",
            entry.word,
            render_outline(&entry.outline)
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
