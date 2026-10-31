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
    /// Composed from one or two already-placed words' own outlines: a stem plus a bare
    /// continuation stroke for a reduced trailing syllable, a leading consonant folded onto
    /// another word's first stroke, or two whole words back to back — struck on top of however
    /// each piece is actually briefed or folded, so a learner who knows the pieces can always
    /// reach the word that way too (see the backup-entry pass in [`generate`]).
    Backup,
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
    /// A backup entry (see [`Origin::Backup`]) whose composed outline another entry already had.
    pub backup_skipped: usize,
    /// A word whose own plain-rules outline costs less than its actual entry, and is sitting
    /// completely unclaimed: a regression guard, not something expected to ever hold more than a
    /// handful of words (see where it is built in [`generate`]).
    pub stranded: Vec<String>,
}

/// Among words that all share one exact pronunciation, the one whose own spelling also agrees
/// with `word`'s at the expected boundary — resolving an ambiguity the sounds alone can't (two
/// homophones, like `build`/`billed`, sharing the stem sounds `building` needs) the same way
/// spelling already breaks a tie between words that sound alike elsewhere in this theory (see
/// `docs/spelling.md`). Sound is still what found these candidates in the first place; spelling
/// only ever narrows a tie between them, never widens the search. `None` if nobody's spelling
/// agrees, or if more than one still does.
/// `sounds`, with each token's trailing stress digit removed — the same trimming
/// `Rules::syllables` already does to tell a vowel from a consonant, reused here so a word's own
/// primary-stress pronunciation can still match its secondary-stress form inside a compound
/// (`my`'s own `M AY1` against `myself`'s own `M AY2 ...`).
fn stress_stripped(sounds: &[String]) -> Vec<String> {
    sounds.iter().map(|s| s.trim_end_matches(|c: char| c.is_ascii_digit()).to_owned()).collect()
}

fn spelled_stem<'a>(word: &str, stems: &[&'a str], boundary: fn(&str, &str) -> bool) -> Option<&'a str> {
    match *stems {
        [stem] => Some(stem),
        _ => {
            let matches: Vec<&'a str> = stems.iter().copied().filter(|s| boundary(word, s)).collect();
            match matches[..] {
                [stem] => Some(stem),
                _ => None,
            }
        }
    }
}

/// One candidate composed outline from the exhaustive backup pass in [`generate`]: checked for a
/// real fragment (the same safety net every composition kind already used before this pass was
/// unified) and for a collision, then either added as a new [`Origin::Backup`] entry or counted
/// in `backup_skipped`. `added_this_round` is what the fixed-point loop in `generate` watches to
/// know when nothing further is left to find.
#[allow(clippy::too_many_arguments)]
fn try_add_backup(
    setup: &Setup,
    entries: &mut Vec<Entry>,
    taken: &mut HashMap<String, String>,
    backup_skipped: &mut usize,
    added_this_round: &mut usize,
    word: &str,
    sounds: &[String],
    composed: Vec<Stroke>,
) {
    if !setup.rules.matches_real_consonants(&composed, word, sounds) {
        return;
    }
    let text = render_outline(&composed);
    if taken.contains_key(&text) {
        *backup_skipped += 1;
        return;
    }
    taken.insert(text, word.to_owned());
    entries.push(Entry {
        word: word.to_owned(),
        outline: composed,
        origin: Origin::Backup,
        lost_to: None,
    });
    *added_this_round += 1;
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
    taken: &HashMap<String, String>,
) -> Option<Vec<Stroke>> {
    // Whether a candidate is one of the word's own gold outlines (never second-guessed by a
    // collision below — a hand-picked Plover brief like "and"'s `SKP` is allowed to sit on
    // whatever some other, unrelated word would have wanted by the plain rules; that trade is
    // this whole mechanism's normal, accepted cost, not a bug) or this function's own synthesized
    // drop repair (which is).
    let mut candidates: Vec<(bool, Vec<Stroke>)> = outlines
        .iter()
        .filter_map(|o| parse_outline(o))
        .flat_map(|o| {
            let repaired = rules.drop_fragments(&o, word, sounds).map(|d| (true, d));
            std::iter::once((false, o)).chain(repaired)
        })
        .collect();
    // A *dropped* sound that happens to land exactly on a different word's own plain outline
    // isn't a saving: it just hands that other word a ladder fight it would never otherwise have
    // had, almost always a shorter, commoner word than the one being briefed (`fact` dropping its
    // own `K` entirely used to take `fat`'s rightful, un-briefed `TPAT`). Only the synthesized
    // drop candidate is checked — a raw gold outline colliding with another word is the ordinary,
    // already-accepted case below, not this one. Checked after the key-count/fragment comparisons
    // that matter when there's no such collision, so a candidate only pays this cost when it
    // would otherwise have won anyway.
    candidates.sort_by_key(|(dropped, o)| {
        let collides = *dropped && natural.get(&render_outline(o)).is_some_and(|&owner| owner != word);
        // Two of Plover's own gold outlines sometimes tie on every measure above (`best`'s own
        // `PW*ES` and `PWEFT` both cost 5 keys) — a star used only as a tie-break coin flip in
        // their own source, not for anything this word actually needs disambiguated. Preferring
        // the star-free one when every other measure is equal costs nothing and never overrides
        // a real win on length, fragment-safety, collision avoidance or key count above it.
        let starred = o.iter().any(|s| s.0 & Stroke::STAR != 0);
        (o.len(), !rules.matches_real_consonants(o, word, sounds), collides, keys(o), starred)
    });
    // The best candidate by that order is usually free, but not always: the commonest word to
    // ever want an outline keeps it (that's the whole point of settling words commonest-first),
    // so a rarer word's own top pick can already be spoken for. Rather than giving up outright —
    // which used to mean falling all the way back to the word's full, multi-stroke phonetic
    // outline even when its second- or third-choice brief candidate was sitting right there,
    // untaken, and still far shorter — this tries every candidate in the same fragment-safe order
    // and takes the first one nobody else already has.
    candidates
        .into_iter()
        .map(|(_, o)| o)
        .find(|o| !taken.contains_key(&render_outline(o)))
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

/// Every outline a real keymux config already gives to a command, a punctuation mark, or
/// fingerspelling - never available to a word, whatever the rules would have generated for it.
/// Pulled out of [`generate`] so a test can confirm the invariant it exists to guarantee (no
/// generated word ever lands on one of these) without re-deriving this list by hand.
pub(crate) fn reserved_outlines() -> Result<HashMap<String, String>> {
    let mut taken = HashMap::new();
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
    Ok(taken)
}

pub fn generate(setup: &Setup, briefs: &Briefs, common: usize) -> Result<Generated> {
    let mut taken: HashMap<String, String> = reserved_outlines()?;
    let mut entries: Vec<Entry> = Vec::new();
    let mut pinned: HashSet<String> = HashSet::new();

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
            "plover" => {
                plover.and_then(|o| best_brief(o, &setup.rules, word, sounds, &empty_natural, &baseline_taken))
            }
            "lapwing" => {
                lapwing.and_then(|o| best_brief(o, &setup.rules, word, sounds, &empty_natural, &baseline_taken))
            }
            explicit => parse_outline(explicit),
        };
        let outline = outline
            .or_else(|| {
                setup.gold.plover.get(word.as_str()).and_then(|o| {
                    best_brief(o, &setup.rules, word, sounds, &empty_natural, &baseline_taken)
                })
            })
            .or_else(|| {
                setup.gold.lapwing.get(word.as_str()).and_then(|o| {
                    best_brief(o, &setup.rules, word, sounds, &empty_natural, &baseline_taken)
                })
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
            "plover" => plover.and_then(|o| best_brief(o, &setup.rules, word, sounds, &natural, &taken)),
            "lapwing" => lapwing.and_then(|o| best_brief(o, &setup.rules, word, sounds, &natural, &taken)),
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
                    .and_then(|o| best_brief(o, &setup.rules, word, sounds, &natural, &taken))
            })
            .or_else(|| {
                setup
                    .gold
                    .lapwing
                    .get(word.as_str())
                    .and_then(|o| best_brief(o, &setup.rules, word, sounds, &natural, &taken))
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

    // Backup entries: exhaustive, run to a fixed point. Every round indexes everything placed so
    // far (the primaries from the loop above in round 0, plus every earlier round's own backup
    // entries from round 1 on) and tries every split point of every word's own sound sequence
    // against three composition kinds — a trailing reduced-syllable suffix continuation (one or
    // two strokes), a leading consonant cluster folded onto a placed word's first stroke, and two
    // whole placed words back to back — adding every safe, non-colliding composed outline it
    // finds, not just the first. A word can and does end up with more than one backup entry;
    // that is the explicit goal here, not something to dedupe away.
    //
    // Chaining falls out for free from re-indexing from scratch every round: once `building` has
    // a backup entry (round 0: `build`'s own outline plus the "-ing" stroke), round 1's rebuilt
    // index includes it as an eligible stem the same as any primary entry, so `buildings`' own
    // search finds `building` and continues it with "-s" — `build`/`+-G`/`+-S`, reached without
    // `building` ever needing its own primary outline to be that shape.
    //
    // A standalone suffix's own entry (`rules/suffixes.toml`) is a continuation-stroke shape, not
    // a real word's own outline — using one as a stem here would carry its join key onto
    // whatever gets composed from it (`ed`'s own entry is the bare continuation `+-D`, the "-ed"
    // stroke struck on its own), so those are excluded from standing in as a stem entirely, every
    // round. Collisions are skipped and counted, never a hard failure, since this is a
    // best-effort pass over the whole dictionary, not an explicit human-authored override.
    //
    // Capped at a small, fixed number of rounds: growth should already have stopped well before
    // that in practice (each round can only add entries built from the *previous* round's new
    // stems, so the search space shrinks every round rather than compounding), and hitting the
    // cap is worth investigating as a sign something is matching too permissively, not a reason
    // to just raise it further.
    const MAX_BACKUP_ROUNDS: usize = 5;
    let mut backup_skipped = 0usize;
    for _round in 0..MAX_BACKUP_ROUNDS {
        let mut word_outline: HashMap<String, Vec<Stroke>> = HashMap::new();
        for entry in entries.iter().filter(|e| e.origin != Origin::Suffix) {
            let cost = (entry.outline.len(), keys(&entry.outline));
            match word_outline.get(&entry.word) {
                Some(current) if (current.len(), keys(current)) <= cost => {}
                _ => {
                    word_outline.insert(entry.word.clone(), entry.outline.clone());
                }
            }
        }
        let mut by_sounds: HashMap<&[String], Vec<&str>> = HashMap::new();
        // A stress-insensitive copy of the same index, for the compound direction (c) only: a
        // word keeps its own primary stress when struck alone (`my` is `M AY1`) but a compound's
        // own internal half often carries secondary stress instead (`myself` is `M AY2 S EH1 L
        // F`) - the same vowel, read the same way, just not the exact token an exact phoneme
        // match requires. Stripping the trailing stress digit the same way `syllables` already
        // does elsewhere lets `myself` find `my` as its own first half without also loosening the
        // suffix/prefix directions (a)/(b), which have never needed it.
        let mut by_sounds_loose: HashMap<Vec<String>, Vec<&str>> = HashMap::new();
        for word in word_outline.keys() {
            if let Some(sounds) = setup.pron.get(word.as_str()).and_then(|prons| prons.first()) {
                by_sounds.entry(sounds.as_slice()).or_default().push(word.as_str());
                by_sounds_loose.entry(stress_stripped(sounds)).or_default().push(word.as_str());
            }
        }

        let mut added_this_round = 0usize;

        for word in word_outline.keys().cloned().collect::<Vec<_>>() {
            let Some(sounds) = setup.pron.get(&word).and_then(|prons| prons.first()) else {
                continue;
            };

            // (a) stem + reduced trailing-syllable suffix continuation, one stroke (`build` +
            // "-ing" -> building; see `Rules::suffix_split`). More than one already-placed word
            // with the exact same pronunciation (`build`/`billed`): spelling breaks the tie
            // (`building` starts with `build`, not `billed`) if it can. The two-syllable suffixes
            // (`-ism`, `-ary`/`-ery`/`-ory`, `-ical`, `-logy`, `-cy`/`-sy`, `-ify`/`-fy`) are
            // handled separately, by (a2) below, not by extending this one: their own second
            // syllable's onset consonant sometimes migrates backward onto the *first* suffix
            // stroke rather than staying on the second the way plain syllabification would place
            // it (`-ism`'s own `+EUZ/+-PL` ends its first stroke in the "z" that `self.syllables`'
            // ordinary onset-assignment would otherwise give to the second syllable), so (a2) finds
            // its stem by spelling and reuses `rules/suffixes.toml`'s own, already hand-verified
            // chord as-is instead of re-deriving it through sound rules. See `docs/suffixes.md`.
            if let Some((split, bare)) = setup.rules.suffix_split(&word, sounds)
                && let Some(stems) = by_sounds.get(&sounds[..split])
                && let Some(stem) = spelled_stem(&word, stems, |w, s| w.starts_with(s))
                && stem != word
            {
                let mut composed = word_outline[stem].clone();
                composed.push(bare);
                setup.rules.joined(&mut composed);
                try_add_backup(
                    setup,
                    &mut entries,
                    &mut taken,
                    &mut backup_skipped,
                    &mut added_this_round,
                    &word,
                    sounds,
                    composed,
                );
            }

            // (a2) stem + a known two-stroke suffix (`-ism`, `-ary`/`-ery`/`-ory`, `-ical`,
            // `-logy`, `-cy`/`-sy`, `-ify`/`-fy`): the gap (a)'s own comment names above. Found by
            // spelling, not sound (see `Rules::two_stroke_suffix`'s own doc for why), so the stem
            // is looked up by its own spelling directly rather than through `by_sounds`.
            if let Some((suffix_text, suffix_strokes)) = setup.rules.two_stroke_suffix(&word) {
                let stem_spelling = &word[..word.len() - suffix_text.len()];
                if let Some(stem_outline) = word_outline.get(stem_spelling) {
                    let mut composed = stem_outline.clone();
                    composed.extend_from_slice(suffix_strokes);
                    setup.rules.joined(&mut composed);
                    try_add_backup(
                        setup,
                        &mut entries,
                        &mut taken,
                        &mut backup_skipped,
                        &mut added_this_round,
                        &word,
                        sounds,
                        composed,
                    );
                }
            }

            // (b) leading consonant cluster onto a placed word's first stroke, since lyk's join
            // key only ever continues a stroke forward, never backward: a continuation stroke can
            // follow a stem, but nothing can precede one (`ever` -> `never`, the extra sound "N").
            // Tries every leading length, not just the first that works. When the leading sounds'
            // own keys overlap a key the matched word's first stroke already uses, this direction
            // simply can't rescue the word (`unclear` needs `un-`'s own `TPH`, which shares an
            // `H` with `clear`'s own "cl" onset cluster `KHR`) — skipped, not forced; a real,
            // structural gap in the theory, not a bug in this pass.
            for split in 1..sounds.len().min(4) {
                let Some(stems) = by_sounds.get(&sounds[split..]) else {
                    continue;
                };
                let Some(stem) = spelled_stem(&word, stems, |w, s| w.ends_with(s)) else {
                    continue;
                };
                if stem == word {
                    continue;
                }
                let Some(leading) = setup.rules.prefix_keys(&word, sounds, split) else {
                    continue;
                };
                let Some(&target_first) = word_outline[stem].first() else {
                    continue;
                };
                if leading.0 & target_first.0 != 0 {
                    continue;
                }
                let mut composed = word_outline[stem].clone();
                composed[0] = Stroke(leading.0 | target_first.0);
                try_add_backup(
                    setup,
                    &mut entries,
                    &mut taken,
                    &mut backup_skipped,
                    &mut added_this_round,
                    &word,
                    sounds,
                    composed,
                );
            }

            // (c) two whole placed words back to back (`myself` -> `my` + `self`), composed as
            // the first word's own outline followed by the second's, every stroke of the second
            // marked as a join continuation exactly the way any other multi-stroke word already
            // is. Unlike (a), the second half doesn't have to be a *reduced* syllable at all —
            // it's a whole word's own outline, stressed vowel and all.
            //
            // Sound alone is too loose a net here: `myself` also sounds exactly like `mice` +
            // `elf` (a real, phonetically exact split at a different point), no relation to
            // either word's own sense and nothing a learner would ever guess — unlike
            // `doctor`/`docked`+`-er` or `crash`/`rash`, which are at least coincidences at the
            // word's own *real* boundary. Spelling is what tells these apart: only a split where
            // the word's own spelling is the literal concatenation of both halves' own spelling
            // is trusted (`myself` really is `"my"` + `"self"`, letter for letter; `"mice"` +
            // `"elf"` isn't `"myself"`'s spelling at all). Tries every split, adding every one
            // that passes — `outside` is genuinely both `"out"` + `"side"` and `"outs"` + `"ide"`,
            // and both are kept as separate entries.
            let loose = stress_stripped(sounds);
            for split in 1..loose.len() {
                let Some(firsts) = by_sounds_loose.get(&loose[..split]) else {
                    continue;
                };
                let Some(seconds) = by_sounds_loose.get(&loose[split..]) else {
                    continue;
                };
                let Some(first) = spelled_stem(&word, firsts, |w, s| w.starts_with(s)) else {
                    continue;
                };
                let Some(second) = spelled_stem(&word, seconds, |w, s| w.ends_with(s)) else {
                    continue;
                };
                if first == word || second == word || word != format!("{first}{second}") {
                    continue;
                }
                let mut composed = word_outline[first].clone();
                composed.extend(word_outline[second].iter().copied());
                setup.rules.joined(&mut composed);
                try_add_backup(
                    setup,
                    &mut entries,
                    &mut taken,
                    &mut backup_skipped,
                    &mut added_this_round,
                    &word,
                    sounds,
                    composed,
                );
            }
        }

        if added_this_round == 0 {
            break;
        }
    }

    // A word is stranded when its own plain-rules outline would cost less than whatever it
    // actually settled for, and that cheaper outline's text is sitting completely unclaimed - the
    // only way that happens today is an automatic brief that Plover or Lapwing mark with a star
    // or an extra key for their own, unrelated reasons (see the brief loop above, which now
    // checks for exactly this before pinning a word to one); kept here as a standing regression
    // guard for any other way this could happen, not just that one. Checked once, against the
    // fully-settled dictionary (after briefs, rules, the ladder, and every backup round), so it
    // only ever reports a genuine, still-available opportunity, never a transient one that got
    // claimed later by the word itself through some other path.
    let stranded: Vec<String> = {
        let best = cheapest(&entries);
        let mut found = Vec::new();
        for (word, outline) in &best {
            let Some(sounds) = setup.pron.get(*word).and_then(|p| p.first()) else { continue };
            let Some(mut natural_outline) = setup.rules.outline(word, sounds) else { continue };
            setup.rules.joined(&mut natural_outline);
            let text = render_outline(&natural_outline);
            let cur_cost = (outline.len(), keys(outline));
            let nat_cost = (natural_outline.len(), keys(&natural_outline));
            if nat_cost < cur_cost && !taken.contains_key(&text) {
                found.push((*word).to_owned());
            }
        }
        found.sort();
        found
    };

    Ok(Generated {
        entries,
        unresolved,
        backup_skipped,
        stranded,
    })
}

/// Each word's cheapest outline (fewest strokes, then fewest keys) among a list of entries.
///
/// A standalone suffix's own entry (`rules/suffixes.toml`) never counts, even when its own
/// display text happens to also be a real word (`less`, `age`, `some`, `ism`, and 17 others): its
/// outline is a continuation-stroke fragment that is never a word by itself when struck alone
/// (the join key it carries refuses that, by design — see [the join key](../docs/join-key.md)),
/// so it is never really "the outline" for that word even when it numerically looks cheapest.
pub fn cheapest(entries: &[Entry]) -> HashMap<&str, &[Stroke]> {
    let mut best: HashMap<&str, &[Stroke]> = HashMap::new();
    for entry in entries.iter().filter(|e| e.origin != Origin::Suffix) {
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

/// Every single-stroke word in the dictionary, by its own bare stroke — what
/// `Rules::resolve`/the deployed plugin check before reading an unmatched run's first stroke as
/// raw phonemes, so a continuation after a known word (`SKP/+-R`, "and" plus "-er") reads as that
/// word's own real text ("ander") instead of garbling the word itself ("sper", "and"'s own `SKP`
/// read as if it meant nothing). A standalone suffix's own entry is excluded the same way
/// [`cheapest`] excludes it everywhere else — its outline is a continuation-stroke fragment, never
/// a real word struck alone. Collision-free by construction: `taken` already guarantees one word
/// per outline text, so there is never more than one candidate for a given stroke here.
pub fn single_stroke_stems(generated: &Generated) -> HashMap<Stroke, String> {
    generated
        .entries
        .iter()
        .filter(|e| e.origin != Origin::Suffix)
        .filter_map(|e| match e.outline[..] {
            [stroke] => Some((stroke, e.word.clone())),
            _ => None,
        })
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
    println!(
        "  backup entries skipped as collisions: {}",
        generated.backup_skipped
    );
    println!(
        "  words stranded on a worse outline than their own, unclaimed plain reading: {}",
        generated.stranded.len()
    );
    if !generated.stranded.is_empty() {
        println!("    {}", generated.stranded.join(", "));
    }
    let sample: Vec<String> = generated
        .unresolved
        .iter()
        .take(40)
        .map(|w| format!("{w} ({})", setup.pron[w.as_str()][0].join(" ")))
        .collect();
    println!("    the commonest: {}", sample.join(", "));

    // Each word's cheapest outline here, and what Plover and Lapwing spend on the same word
    let ours: HashMap<&str, (usize, u32)> = cheapest(&generated.entries)
        .into_iter()
        .map(|(word, outline)| (word, (outline.len(), keys(outline))))
        .collect();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stroke::parse_outline;

    fn entry(word: &str, outline: &str, origin: Origin) -> Entry {
        Entry {
            word: word.to_owned(),
            outline: parse_outline(outline).unwrap(),
            origin,
            lost_to: None,
        }
    }

    #[test]
    fn a_standalone_suffixs_own_entry_never_wins_cheapest_even_when_shorter() {
        // "less" is both a real word (HRES, 4 keys) and the display text of the standalone
        // "-less" suffix's own continuation stroke (+-LS, 2 keys) - a stroke that is never a
        // word by itself when struck alone (see the join key), so it must never be picked as
        // "the" outline for the real word just because it numerically looks cheaper.
        let entries = vec![
            entry("less", "+-LS", Origin::Suffix),
            entry("less", "HRES", Origin::Rules),
        ];
        let best = cheapest(&entries);
        assert_eq!(best.get("less").copied(), Some(&parse_outline("HRES").unwrap()[..]));
    }
}
