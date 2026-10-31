//! The phonetic rules: from sounds to strokes, read from a TOML file.

use crate::stroke::Stroke;
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// The rules that ship with the program.
const BUILT_IN: &str = include_str!("../rules/plover.toml");
/// Spelling-mode's letter table (see `docs/spelling-mode.md`): always this file, independent of
/// whichever rules file `Rules::load` was given, the same way `rules/suffixes.toml` and
/// `rules/briefs.toml` are always their own built-in file regardless of the theory in use —
/// every letter gets one unambiguous chord no matter which phonetic theory is loaded.
const BUILT_IN_SPELLING: &str = include_str!("../rules/spelling.toml");
/// `rules/suffixes.toml`'s own table, loaded the same unconditional way as `BUILT_IN_SPELLING`
/// above: see [`Rules::suffixes`].
const BUILT_IN_SUFFIXES: &str = include_str!("../rules/suffixes.toml");

/// The right-hand `R`, the one key a vowel (`ER`, `AR`) and a cluster (`-FRB`) may share.
const R_KEY: u32 = 1 << 14;

#[derive(Deserialize, Default)]
struct Raw {
    #[serde(default)]
    onset: HashMap<String, String>,
    #[serde(default)]
    coda: HashMap<String, String>,
    #[serde(default)]
    vowel: HashMap<String, String>,
    #[serde(default)]
    vowel_coda: HashMap<String, String>,
    #[serde(default)]
    onset_cluster: HashMap<String, String>,
    #[serde(default)]
    coda_cluster: HashMap<String, String>,
    #[serde(default)]
    syllables: RawSyllables,
    #[serde(default)]
    spelled: Vec<Spelled>,
    #[serde(default)]
    reduce: RawReduce,
    #[serde(default)]
    theory: RawTheory,
    #[serde(default)]
    brevity: Vec<RawBrevity>,
    #[serde(default)]
    prefix: Vec<Prefix>,
}

/// A prefix written with fixed keys that join the first stroke of the rest of the word: `ex-` is
/// `KP`, so `expect` is `KPEBGT`.
#[derive(Deserialize, Debug, Clone)]
pub struct Prefix {
    /// The word starts with these letters
    pub spelling: String,
    /// And its sounds start with one of these (with stress digits)
    pub sounds: Vec<String>,
    /// The keys that replace those sounds
    pub keys: String,
}

#[derive(Deserialize)]
struct RawBrevity {
    step: Step,
    /// Words up to this frequency rank take this step
    rank: usize,
    /// Whether the stressed vowel is kept (one-stroke steps)
    #[serde(default = "yes")]
    vowel: bool,
    /// Only words with at least this many syllables
    #[serde(default = "two")]
    min_syllables: usize,
    /// Whether the folded consonants may land out of steno order (`any`) or not (`strict`)
    #[serde(default)]
    order: Option<String>,
}

const fn yes() -> bool {
    true
}

const fn two() -> usize {
    2
}

/// A way to write a word of several syllables shorter than a stroke per syllable. Each is a
/// transformation of the full outline a learner can do by hand.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    /// One stroke: the consonants that start the word, then every consonant after its first vowel
    /// folded onto the right hand.
    Skeleton,
    /// One stroke: the consonants that start the word and those of the stressed and last
    /// syllables.
    Shell,
    /// One stroke: the consonants that start the word and those of the last syllable.
    Ends,
    /// An unstressed first syllable that is only a vowel is dropped: `about` PWOUT.
    Lead,
    /// The consonants of an unstressed first syllable go on the left hand of the next stroke:
    /// `believe` PWHRAOEF.
    Lean,
    /// The outline with reduced syllables folded out of steno order where they would not fit in
    /// order, as Plover does: `student` STAOUPBTD, `goodness` TKPWAOPBSD.
    Fold,
}

impl std::fmt::Display for Step {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Skeleton => "skeleton",
            Self::Shell => "shell",
            Self::Ends => "ends",
            Self::Lead => "lead",
            Self::Lean => "lean",
            Self::Fold => "fold",
        })
    }
}

/// One shortening step and how common a word has to be to take it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shortening {
    pub step: Step,
    pub rank: usize,
    pub vowel: bool,
    pub min_syllables: usize,
    pub any: bool,
}

#[derive(Deserialize, Default)]
struct RawReduce {
    /// Unstressed vowels that are not written: the syllable's consonants join the stroke before
    #[serde(default)]
    vowels: Vec<String>,
    /// Whether the joined consonants may land out of steno order (`any`) or only in order
    /// (`strict`), writing a continuation stroke otherwise
    #[serde(default)]
    order: Option<String>,
    /// Variants of those vowels that are written after all: a final `a` (`china` KHAOEU/TPHA)
    #[serde(default)]
    except: Vec<String>,
    /// Variants only reduced when a single consonant starts the syllable: `-ly` folds as -L in
    /// `lately`, but `assembly` ends in a stroke of its own, PWHREU
    #[serde(default)]
    lone: Vec<String>,
    /// Consonants that may land out of steno order when a syllable of only them is folded:
    /// Plover's -RT for `-ter` (`better` PWERT) and -LT for `-tly` (`lately` HRAEULT)
    #[serde(default)]
    loose: Vec<String>,
}

#[derive(Deserialize, Default)]
struct RawTheory {
    /// Strokes after the first of a word carry a join key
    #[serde(default)]
    join_key: bool,
}

#[derive(Deserialize, Default)]
struct RawSyllables {
    #[serde(default)]
    onsets: Vec<String>,
    #[serde(default)]
    lax: Vec<String>,
}

/// A sound that is written differently depending on how the word is spelled. It becomes a
/// variant of the sound (`UW.oo`), which has its own entry in the tables.
#[derive(Deserialize, Debug, Clone)]
pub struct Spelled {
    pub sound: String,
    pub label: String,
    /// The word starts with this, and the sound is its first
    pub starts: Option<String>,
    /// The word ends with this, and the sound is its last
    pub ends: Option<String>,
    /// The word has this in it
    pub contains: Option<String>,
    /// The word starts with this, and the sound is its first vowel and unstressed: `be-` in
    /// `believe` but not in `bee`
    pub prefix: Option<String>,
    /// The sound before this one is this (`R` also matches a vowel with an R in it, `ER`)
    pub after: Option<String>,
    /// The sound after this one is this, and the last: the `Z` of `babies`
    pub before: Option<String>,
    /// The word has at least this many syllables
    pub min_syllables: Option<usize>,
    /// The word has none of these in it
    #[serde(default)]
    pub unless: Vec<String>,
    /// Only used to tell apart words that sound the same: the word is written this way when
    /// another word would otherwise have the same outline
    #[serde(default)]
    pub clash: bool,
}

impl Spelled {
    fn applies(
        &self,
        word: &str,
        sounds: &[String],
        at: usize,
        syllables: usize,
        first_vowel: usize,
    ) -> bool {
        let count = sounds.len();
        let before = at.checked_sub(1).map(|i| plain(&sounds[i]));
        self.min_syllables.is_none_or(|n| syllables >= n)
            && self
                .after
                .as_ref()
                .is_none_or(|s| before.is_some_and(|b| b == s || (s == "R" && b == "ER")))
            && self
                .before
                .as_ref()
                .is_none_or(|s| at + 2 == count && plain(&sounds[at + 1]) == s)
            && !self.unless.iter().any(|s| word.contains(s.as_str()))
            && self
                .starts
                .as_ref()
                .is_none_or(|s| at == 0 && word.starts_with(s.as_str()))
            && self.prefix.as_ref().is_none_or(|s| {
                at == first_vowel && sounds[at].ends_with('0') && word.starts_with(s.as_str())
            })
            && self
                .ends
                .as_ref()
                .is_none_or(|s| at + 1 == count && word.ends_with(s.as_str()))
            && self
                .contains
                .as_ref()
                .is_none_or(|s| word.contains(s.as_str()))
    }
}

/// One syllable's sounds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Syllable {
    pub onset: Vec<String>,
    /// With its stress digit: `AH0`
    pub vowel: String,
    pub coda: Vec<String>,
}

#[derive(Debug, Default)]
pub struct Rules {
    onset: HashMap<String, Stroke>,
    coda: HashMap<String, Stroke>,
    vowel: HashMap<String, Stroke>,
    vowel_coda: HashMap<String, Stroke>,
    onset_cluster: HashMap<String, Stroke>,
    coda_cluster: HashMap<String, Stroke>,
    onsets: HashSet<String>,
    lax: HashSet<String>,
    spelled: Vec<Spelled>,
    reduce: HashSet<String>,
    keep: HashSet<String>,
    lone: HashSet<String>,
    loose: HashSet<String>,
    /// Whether consonants joining the stroke before may land out of steno order
    fold_any: bool,
    join_key: bool,
    brevity: Vec<Shortening>,
    prefixes: Vec<(Prefix, Stroke)>,
    // Read the keys of a stroke the rules did not write, back into the sounds that would have
    // written them: the tables above, inverted once here instead of on every call.
    onset_rev: Vec<(Stroke, Vec<String>)>,
    coda_rev: Vec<(Stroke, Vec<String>)>,
    vowel_rev: HashMap<Stroke, String>,
    vowel_coda_rev: Vec<(Stroke, String, String)>,
    /// Spelling-mode's chord -> letter table (see `docs/spelling-mode.md`), keyed by each
    /// letter's bare chord (no join or spell bit).
    spelling: HashMap<Stroke, String>,
    /// `rules/suffixes.toml`'s own chord -> text table, keyed exactly as that file writes each
    /// chord (join bit included, e.g. `+-D`) — consulted by [`Self::resolve`] for a continuation
    /// stroke that misses the dictionary entirely, so a suffix reads as its own real spelling
    /// ("ed") rather than a generic phonetic guess at its bare consonants ("d") whenever it rides
    /// along after some other, unrelated stroke with no dictionary entry of its own.
    suffixes: HashMap<Stroke, String>,
    /// `rules/suffixes.toml`'s two-stroke entries (`-ism`, `-ary`/`-ery`/`-ory`, `-ical`, `-logy`,
    /// `-cy`/`-sy`, `-ify`/`-fy`) - each one's own spelling and its two-stroke chord, exactly as
    /// written there. Consulted by [`Self::two_stroke_suffix`] for the backup-generation pass in
    /// `generate.rs`, by spelling rather than re-derived sound rules (see that method's own doc).
    two_stroke_suffixes: Vec<(String, Vec<Stroke>)>,
}

/// A table inverted: each distinct right-hand side keeps every left-hand side that produced it,
/// split into its sound tokens (`"P Y"` is `["P", "Y"]`). Sorted by its tokens, so that when two
/// entries share the exact same keys (`Z` and the glide cluster `Z Y` are both `STKPW`, since
/// the glide is silent on the keys) the tie always breaks the same way between runs.
fn invert(table: &HashMap<String, Stroke>) -> Vec<(Stroke, Vec<String>)> {
    let mut rev: Vec<(Stroke, Vec<String>)> = table
        .iter()
        .map(|(sounds, &stroke)| {
            (
                stroke,
                sounds.split_whitespace().map(str::to_owned).collect(),
            )
        })
        .collect();
    rev.sort_by(|a, b| a.1.cmp(&b.1));
    rev
}

fn strokes(table: HashMap<String, String>, what: &str) -> Result<HashMap<String, Stroke>> {
    table
        .into_iter()
        .map(|(sound, text)| {
            let stroke = Stroke::parse(&text)
                .with_context(|| format!("[{what}] {sound} = {text:?} is not a stroke"))?;
            Ok((sound, stroke))
        })
        .collect()
}

/// `rules/spelling.toml`'s chord-notation keys, parsed into the bare chords (no join or spell
/// bit) spelling mode looks a stroke's keys up by.
fn spelling_table(raw: &str) -> Result<HashMap<Stroke, String>> {
    let table: HashMap<String, String> = toml::from_str(raw)?;
    table
        .into_iter()
        .map(|(chord, letter)| {
            let stroke = Stroke::parse(&chord)
                .with_context(|| format!("[spelling] {chord:?} is not a stroke"))?;
            Ok((stroke, letter))
        })
        .collect()
}

/// `rules/suffixes.toml`'s single-stroke entries only, keyed by their own chord exactly as
/// written there — its handful of two-stroke suffixes (`-ism`, `-ary`/`-ery`/`-ory`, `-ical`,
/// `-logy`, `-ity`/`-ety`, `-cy`/`-sy`, `-ify`/`-fy`) don't parse as a single [`Stroke`] at all
/// (their own chord has a `/` in it) and are skipped rather than erroring the whole table: a
/// continuation stroke [`Rules::resolve`] reads is always exactly one stroke at a time, so a
/// suffix that takes two has nothing here to match against yet (see `docs/suffixes.md`).
fn single_stroke_suffixes(raw: &str) -> Result<HashMap<Stroke, String>> {
    let table: HashMap<String, String> = toml::from_str(raw)?;
    Ok(table
        .into_iter()
        .filter_map(|(chord, text)| Stroke::parse(&chord).map(|stroke| (stroke, text)))
        .collect())
}

/// `rules/suffixes.toml`'s two-stroke entries only - the ones [`single_stroke_suffixes`] skips
/// because their own chord has a `/` in it. Each chord is parsed whole, in order, keeping its own
/// join bits exactly as written (both strokes already carry `+`).
fn two_stroke_suffixes(raw: &str) -> Result<Vec<(String, Vec<Stroke>)>> {
    let table: HashMap<String, String> = toml::from_str(raw)?;
    table
        .into_iter()
        .filter(|(chord, _)| chord.contains('/'))
        .map(|(chord, text)| {
            let strokes: Option<Vec<Stroke>> = chord.split('/').map(Stroke::parse).collect();
            let strokes =
                strokes.with_context(|| format!("[suffixes] {chord:?} is not a valid two-stroke chord"))?;
            Ok((text, strokes))
        })
        .collect()
}

/// A sound without its stress digit or spelling variant: `UW.oo1` is `UW`.
fn plain(sound: &str) -> &str {
    sound
        .trim_end_matches(|c: char| c.is_ascii_digit())
        .split('.')
        .next()
        .unwrap_or(sound)
}

/// A syllable written as a continuation stroke: if that stroke has no vowel keys, the syllable
/// has no vowel either when more consonants fold onto it (`confidence` KOPB/-FD/-PBS).
fn vowelless(mut syllable: Syllable, written: Stroke) -> Syllable {
    if written.0 & (Stroke::VOWELS | Stroke::STAR) == 0 {
        syllable.vowel = String::new();
    }
    // Written on the right hand, its consonants stay there
    if written.0 & Stroke::LEFT == 0 {
        let mut coda = std::mem::take(&mut syllable.onset);
        coda.append(&mut syllable.coda);
        syllable.coda = coda;
    }
    syllable
}

/// How a vowel label sorts when two share a physical stroke: lowest wins the stroke's reverse
/// reading. A plain sound normally beats a spelling variant, except `UH` ("book", "put"), which
/// is rarer in practice than the `UW.oo` ("too", "-oo" endings like "kangaroo") and `OW.oa`
/// ("boat") variants it happens to share bare `AO` with — ranking it first like any other plain
/// sound would make the fallback's best guess for that stroke the least likely reading.
fn vowel_rank(s: &str) -> (bool, &str) {
    (!s.contains('.') && s != "UH", s)
}

/// A sound without its spelling variant but with its stress: `UW.oo1` is `UW1`.
fn unlabelled(sound: &str) -> String {
    let stress: String = sound.chars().filter(char::is_ascii_digit).collect();
    format!("{}{stress}", plain(sound))
}

impl Rules {
    pub fn built_in() -> Result<Self> {
        Self::parse(BUILT_IN)
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).with_context(|| path.display().to_string())?;
        Self::parse(&text)
    }

    fn parse(text: &str) -> Result<Self> {
        let raw: Raw = toml::from_str(text)?;
        if raw.onset.is_empty() || raw.vowel.is_empty() {
            bail!("the rules need at least [onset] and [vowel]");
        }
        let onset = strokes(raw.onset, "onset")?;
        let coda = strokes(raw.coda, "coda")?;
        let vowel = strokes(raw.vowel, "vowel")?;
        let vowel_coda = strokes(raw.vowel_coda, "vowel_coda")?;
        let onset_cluster = strokes(raw.onset_cluster, "onset_cluster")?;
        let coda_cluster = strokes(raw.coda_cluster, "coda_cluster")?;

        let mut onset_rev = invert(&onset);
        onset_rev.extend(invert(&onset_cluster));
        let mut coda_rev = invert(&coda);
        coda_rev.extend(invert(&coda_cluster));
        // When two labels share the same vowel keys, deterministically pick which one wins the
        // reverse reading: see `vowel_rank`.
        let mut vowel_pairs: Vec<(Stroke, String)> =
            vowel.iter().map(|(s, &k)| (k, s.clone())).collect();
        vowel_pairs.sort_by(|a, b| vowel_rank(&a.1).cmp(&vowel_rank(&b.1)));
        let vowel_rev: HashMap<Stroke, String> = vowel_pairs.into_iter().collect();
        let mut vowel_coda_rev: Vec<(Stroke, String, String)> = vowel_coda
            .iter()
            .filter_map(|(sounds, &stroke)| {
                let (vowel, coda) = sounds.split_once(' ')?;
                Some((stroke, vowel.to_owned(), coda.to_owned()))
            })
            .collect();
        vowel_coda_rev.sort_by(|a, b| (&a.1, &a.2).cmp(&(&b.1, &b.2)));
        let spelling = spelling_table(BUILT_IN_SPELLING)?;
        let suffixes = single_stroke_suffixes(BUILT_IN_SUFFIXES)?;
        let two_stroke_suffixes = two_stroke_suffixes(BUILT_IN_SUFFIXES)?;

        Ok(Self {
            onset,
            coda,
            vowel,
            vowel_coda,
            onset_cluster,
            coda_cluster,
            onset_rev,
            coda_rev,
            vowel_rev,
            vowel_coda_rev,
            spelling,
            suffixes,
            two_stroke_suffixes,
            onsets: raw.syllables.onsets.into_iter().collect(),
            lax: raw.syllables.lax.into_iter().collect(),
            spelled: raw.spelled,
            reduce: raw.reduce.vowels.into_iter().collect(),
            keep: raw.reduce.except.into_iter().collect(),
            lone: raw.reduce.lone.into_iter().collect(),
            loose: raw.reduce.loose.into_iter().collect(),
            fold_any: match raw.reduce.order.as_deref() {
                None | Some("any") => true,
                Some("strict") => false,
                Some(other) => bail!("[reduce] order = {other:?}: use \"any\" or \"strict\""),
            },
            prefixes: raw
                .prefix
                .into_iter()
                .map(|p| {
                    let keys = Stroke::parse(&p.keys)
                        .with_context(|| format!("[[prefix]] {} keys {:?}", p.spelling, p.keys))?;
                    Ok((p, keys))
                })
                .collect::<Result<_>>()?,
            join_key: raw.theory.join_key,
            brevity: raw
                .brevity
                .into_iter()
                .map(|b| {
                    Ok(Shortening {
                        step: b.step,
                        rank: b.rank,
                        vowel: b.vowel,
                        min_syllables: b.min_syllables,
                        any: match b.order.as_deref() {
                            None | Some("strict") => false,
                            Some("any") => true,
                            Some(other) => {
                                bail!("[[brevity]] order = {other:?}: use \"any\" or \"strict\"")
                            }
                        },
                    })
                })
                .collect::<Result<_>>()?,
        })
    }

    /// The keys for a run of consonants, one sound after another (or a cluster rule for a run of
    /// them). The keys have to come in steno order, with no overlap: otherwise it can't be
    /// written in one stroke by these rules and this gives `None`.
    ///
    /// With `fold`, the keys may come in any order as long as none is used twice: that is how a
    /// reduced syllable's consonants are added to the stroke before it.
    fn consonants(
        sounds: &[String],
        single: &HashMap<String, Stroke>,
        clusters: &HashMap<String, Stroke>,
        fold: bool,
    ) -> Option<Stroke> {
        let mut bits = 0u32;
        let mut at = 0;
        while at < sounds.len() {
            let (stroke, used) = (2..=sounds.len() - at)
                .rev()
                .find_map(|n| {
                    let run = &sounds[at..at + n];
                    let labelled = run.join(" ");
                    let bare = run.iter().map(|s| plain(s)).collect::<Vec<_>>().join(" ");
                    clusters
                        .get(&labelled)
                        .or_else(|| clusters.get(&bare))
                        .map(|&s| (s, n))
                })
                .or_else(|| {
                    single
                        .get(&sounds[at])
                        .or_else(|| single.get(plain(&sounds[at])))
                        .map(|&s| (s, 1))
                })?;
            let highest = Stroke(bits).highest();
            // Only a star may go anywhere: it belongs to the vowels
            let keys = Stroke(stroke.0 & !Stroke::STAR);
            if fold {
                if bits & keys.0 != 0 {
                    return None;
                }
            } else if let (Some(highest), Some(lowest)) = (highest, keys.lowest())
                && lowest <= highest
            {
                return None;
            }
            bits |= stroke.0;
            at += used;
        }
        Some(Stroke(bits))
    }

    /// The keys that write one syllable, if one stroke can.
    pub fn syllable_stroke(&self, syllable: &Syllable) -> Option<Stroke> {
        self.stroke_for(syllable, false)
    }

    fn stroke_for(&self, syllable: &Syllable, fold: bool) -> Option<Stroke> {
        let onset = Self::consonants(&syllable.onset, &self.onset, &self.onset_cluster, false)?;
        let base = syllable
            .vowel
            .trim_end_matches(|c: char| c.is_ascii_digit());
        // A continuation stroke that became the stem of the next fold has no vowel
        let plain_vowel = if syllable.vowel.is_empty() {
            Some(Stroke(0))
        } else {
            self.vowel
                .get(&syllable.vowel)
                .or_else(|| self.vowel.get(base))
                .or_else(|| self.vowel.get(plain(base)))
                .copied()
        };
        // The vowel and the consonant after it together (`AR`, `OPBG`) when the rules have that,
        // and the rest of the consonants fit after it; otherwise the vowel on its own
        let joined = syllable.coda.first().and_then(|first| {
            self.vowel_coda
                .get(&format!("{base} {first}"))
                .or_else(|| {
                    self.vowel_coda
                        .get(&format!("{} {}", plain(base), plain(first)))
                })
                .map(|&v| (v, &syllable.coda[1..]))
        });
        let rest = &syllable.coda[..];
        let with = |vowel: Stroke, coda: &[String]| -> Option<Stroke> {
            // A vowel with an `R` of its own (`ER`, `AR`) and a cluster that has the `R` in it
            // too, as in `serve` -FRB, `march` -FRPB or `harsh` -RB, share the key; any other
            // key used twice does not fit, and nor does a second `R` sound (`harder`)
            let shared = if coda.iter().any(|s| plain(s) == "R") {
                0
            } else {
                R_KEY
            };
            let coda = Self::consonants(coda, &self.coda, &self.coda_cluster, fold)?;
            if vowel.0 & coda.0 & Stroke::RIGHT & !shared != 0 {
                return None;
            }
            if vowel.0 & coda.0 & Stroke::STAR != 0 {
                return None;
            }
            Some(Stroke(onset.0 | vowel.0 | coda.0))
        };
        joined
            .and_then(|(vowel, coda)| with(vowel, coda))
            .or_else(|| with(plain_vowel?, rest))
    }

    /// Give the sounds of a word their spelling variants: how the word is spelled can decide
    /// which keys a sound goes on.
    pub fn label(&self, word: &str, sounds: &[String]) -> Vec<String> {
        self.label_with(word, sounds, false)
    }

    /// [`Self::label`], also using the rules that only tell apart words that sound the same.
    fn label_with(&self, word: &str, sounds: &[String], clash: bool) -> Vec<String> {
        let is_vowel = |s: &String| s.ends_with(|c: char| c.is_ascii_digit());
        let syllables = sounds.iter().filter(|s| is_vowel(s)).count();
        let first_vowel = sounds.iter().position(is_vowel).unwrap_or(usize::MAX);
        sounds
            .iter()
            .enumerate()
            .map(|(at, sound)| {
                let stress: String = sound.chars().filter(char::is_ascii_digit).collect();
                let rule = self.spelled.iter().find(|r| {
                    (clash || !r.clash)
                        && r.sound == plain(sound)
                        && r.applies(word, sounds, at, syllables, first_vowel)
                });
                rule.map_or_else(|| sound.clone(), |r| format!("{}{stress}", r.label))
            })
            .collect()
    }

    /// Split a word's sounds into syllables. Between vowels, the longest run of consonants that
    /// can start a syllable goes to the next one, except that a stressed short vowel keeps a single
    /// consonant after it.
    pub fn syllables(&self, sounds: &[String]) -> Option<Vec<Syllable>> {
        let is_vowel = |s: &str| s.ends_with(|c: char| c.is_ascii_digit());
        let vowels: Vec<usize> = (0..sounds.len())
            .filter(|&i| is_vowel(&sounds[i]))
            .collect();
        let (&first, &last) = (vowels.first()?, vowels.last()?);
        let mut result: Vec<Syllable> = Vec::new();
        let mut onset: Vec<String> = sounds[..first].to_vec();
        for (n, &v) in vowels.iter().enumerate() {
            let mut syllable = Syllable {
                onset: std::mem::take(&mut onset),
                vowel: sounds[v].clone(),
                coda: Vec::new(),
            };
            if v == last {
                syllable.coda = sounds[v + 1..].to_vec();
            } else {
                let cluster = &sounds[v + 1..vowels[n + 1]];
                let next = sounds[vowels[n + 1]].trim_end_matches(|c: char| c.is_ascii_digit());
                // The `-ly` ending takes its L and a stop before it (`sim-ply`, `as-sem-bly`),
                // never an S: `ous-ly`, not `ou-sly`
                let take = if next == "IY.ly" {
                    let ends_in_l = cluster.last().is_some_and(|s| plain(s) == "L");
                    let stressed = syllable.vowel.ends_with(['1', '2']);
                    let keeps = stressed && self.lax.contains(plain(&syllable.vowel));
                    // `ug-ly`: a stressed short vowel keeps its consonant here too
                    let two = cluster.len() >= 2 && !(keeps && cluster.len() == 2) && {
                        let pair = &cluster[cluster.len() - 2..];
                        plain(&pair[0]) != "S"
                            && self.onsets.contains(&format!("{} L", plain(&pair[0])))
                    };
                    match (ends_in_l, two) {
                        (true, true) => 2,
                        (true, false) => 1,
                        _ => 0,
                    }
                } else {
                    self.onset_length(&syllable.vowel, cluster)
                };
                let split = cluster.len() - take;
                syllable.coda = cluster[..split].to_vec();
                onset = cluster[split..].to_vec();
            }
            result.push(syllable);
        }
        Some(result)
    }

    /// How many of the consonants between two vowels start the second syllable.
    fn onset_length(&self, vowel: &str, cluster: &[String]) -> usize {
        // A stressed short vowel keeps a consonant after it: `bet-ter`, `ev-ry`, not `e-vry`
        let stressed = vowel.ends_with(['1', '2']);
        let keep = usize::from(stressed && self.lax.contains(plain(vowel)));
        (1..=cluster.len().saturating_sub(keep))
            .rev()
            .find(|&n| {
                let run: Vec<&str> = cluster[cluster.len() - n..]
                    .iter()
                    .map(|s| plain(s))
                    .collect();
                self.onsets.contains(&run.join(" "))
            })
            .unwrap_or(0)
    }

    /// The shortening steps, in the order they are tried.
    pub fn brevity(&self) -> &[Shortening] {
        &self.brevity
    }

    /// The shorter outlines a shortening step gives a word of several syllables.
    pub fn shorten(&self, step: Shortening, word: &str, sounds: &[String]) -> Vec<Vec<Stroke>> {
        let syllables = sounds
            .iter()
            .filter(|s| s.ends_with(|c: char| c.is_ascii_digit()))
            .count();
        if syllables < step.min_syllables {
            return Vec::new();
        }
        match step.step {
            Step::Lead | Step::Lean => self.leading(step.step, word, sounds).into_iter().collect(),
            Step::Fold => self
                .outline_folded(word, sounds, false, true)
                .into_iter()
                .collect(),
            Step::Skeleton | Step::Shell | Step::Ends => self
                .skeletons_folded(word, sounds, step.any)
                .into_iter()
                .filter(|(kind, vowel, _)| *kind == step.step && *vowel == step.vowel)
                .map(|(_, _, stroke)| vec![stroke])
                .collect(),
        }
    }

    /// A word of several syllables as one stroke: the consonants that start it, then consonants
    /// after its first vowel folded onto the right hand, with the stressed vowel or without. Which
    /// consonants depends on the step: all of them, those of the stressed and last syllables, or
    /// those of the last syllable only.
    #[cfg(test)]
    pub fn skeletons(&self, word: &str, sounds: &[String]) -> Vec<(Step, bool, Stroke)> {
        self.skeletons_folded(word, sounds, false)
    }

    /// [`Self::skeletons`], with the consonants allowed out of steno order (`any`) or not.
    fn skeletons_folded(
        &self,
        word: &str,
        sounds: &[String],
        any: bool,
    ) -> Vec<(Step, bool, Stroke)> {
        let labelled = self.label(word, sounds);
        let (prefix, skip) = self.prefix(word, &labelled).unzip();
        let Some(syllables) = self.syllables(&labelled[skip.unwrap_or(0)..]) else {
            return Vec::new();
        };
        if syllables.len() + usize::from(prefix.is_some()) < 2 {
            return Vec::new();
        }
        // The consonants of a syllable, in order, with the `R` a reduced `ER` hides and without
        // the glide `Y` of `opinion`, `brilliant`
        let consonants_of = |syllable: &Syllable, with_onset: bool| -> Vec<String> {
            let mut sounds: Vec<String> = Vec::new();
            if with_onset {
                sounds.extend(syllable.onset.iter().cloned());
            }
            if plain(&syllable.vowel) == "ER" {
                sounds.push("R".into());
            }
            sounds.extend(syllable.coda.iter().cloned());
            sounds.retain(|s| plain(s) != "Y");
            sounds
        };
        // An unstressed first syllable is not a syllable here: its consonants are the left hand
        // (`in-` TPH, `be-` PW) and the next syllable's onset joins them when it comes after in
        // order (`believe` PWHR); with no consonants it is dropped (`opinion` P-PB)
        let in_order = |a: Stroke, b: Stroke| match (a.highest(), b.lowest()) {
            (Some(high), Some(low)) => high <= low,
            _ => true,
        };
        let left =
            |sounds: &[String]| Self::consonants(sounds, &self.onset, &self.onset_cluster, false);
        let mut start = 0;
        let mut onset = Stroke(0);
        // Whether the first kept syllable's own onset goes on the right hand with the rest
        let mut onset_right = false;
        if syllables.len() >= 2 && syllables[0].vowel.ends_with('0') {
            let lead = &syllables[0];
            let coda: Vec<String> = lead
                .coda
                .iter()
                .filter(|s| plain(s) != "Y")
                .cloned()
                .collect();
            let mut keys = left(&lead.onset).unwrap_or_default();
            if let Some(c) = left(&coda)
                && in_order(keys, c)
            {
                keys.0 |= c.0;
            }
            start = 1;
            match left(&syllables[1].onset) {
                Some(next) if in_order(keys, next) => onset = Stroke(keys.0 | next.0),
                _ if keys.is_empty() => start = 0,
                _ => {
                    onset = keys;
                    onset_right = true;
                }
            }
        }
        if start == 0 {
            let Some(own) = left(&syllables[0].onset) else {
                return Vec::new();
            };
            onset = own;
        }
        let onset = Stroke(onset.0 | prefix.map_or(0, |p| p.0));
        let syllables = &syllables[start..];
        let last = syllables.len() - 1;
        let stress = syllables
            .iter()
            .position(|s| s.vowel.ends_with('1'))
            .unwrap_or(0);
        // From the most to the fewest consonants kept: all of them, then those of the stressed
        // and last syllables, then those of the last syllable only
        let mut choices: Vec<(Step, Vec<String>)> = Vec::new();
        let first_coda = consonants_of(&syllables[0], onset_right);
        let mut all = first_coda.clone();
        for syllable in &syllables[1..] {
            all.extend(consonants_of(syllable, true));
        }
        choices.push((Step::Skeleton, all));
        let mut some = if stress == 0 {
            first_coda.clone()
        } else {
            Vec::new()
        };
        if stress > 0 {
            some.extend(consonants_of(&syllables[stress], true));
        }
        if last > stress {
            some.extend(consonants_of(&syllables[last], true));
        }
        choices.push((Step::Shell, some));
        if last > 0 {
            choices.push((Step::Ends, consonants_of(&syllables[last], true)));
        }

        // The vowel of the syllable with the main stress
        let key = syllables[stress].vowel.as_str();
        let vowel = self
            .vowel
            .get(key)
            .or_else(|| {
                self.vowel
                    .get(key.trim_end_matches(|c: char| c.is_ascii_digit()))
            })
            .or_else(|| self.vowel.get(plain(key)))
            .copied();

        let mut found: Vec<(Step, bool, Stroke)> = Vec::new();
        for (step, rest) in choices {
            // The consonants in steno order, as they are read; a form with no left hand and no
            // vowel is not offered
            let Some(coda) = Self::consonants(&rest, &self.coda, &self.coda_cluster, any) else {
                continue;
            };
            if coda.is_empty() {
                continue;
            }
            if onset.is_empty() {
                continue;
            }
            let bare = Stroke(onset.0 | coda.0);
            found.push((step, false, bare));
            if let Some(vowel) = vowel
                && vowel.0 & coda.0 & Stroke::STAR == 0
            {
                found.push((step, true, Stroke(bare.0 | vowel.0)));
            }
        }
        found
    }

    /// Whether strokes after the first of a word carry the join key.
    pub const fn join_key(&self) -> bool {
        self.join_key
    }

    /// Whether a syllable is one whose vowel is not written.
    fn is_reduced(&self, syllable: &Syllable) -> bool {
        let label = syllable
            .vowel
            .trim_end_matches(|c: char| c.is_ascii_digit());
        syllable.vowel.ends_with('0')
            && !self.keep.contains(label)
            && (syllable.onset.len() == 1 || !self.lone.contains(label))
            && (self.reduce.contains(label) || self.reduce.contains(plain(label)))
    }

    /// A reduced syllable that did not fit on the stroke before it: a continuation stroke of just
    /// its consonants (`-G` for `-ing`, `-D` for `-ed`, `-PB` for `-en`). A reduced `ER` keeps its
    /// vowel keys (`ER`), and a syllable with no consonants keeps its vowel too.
    fn continuation(&self, syllable: &Syllable) -> Option<Stroke> {
        let is_er = plain(&syllable.vowel) == "ER";
        let consonants = syllable.onset.len() + syllable.coda.len();
        if consonants == 0 && !is_er {
            return self.syllable_stroke(syllable);
        }
        // A suffix is written on the right hand with no vowel (`-ness` -PBS, `-ment` -PLT, `-er`
        // -R); when its consonants don't fit there in order, the first go on the left (`-nial`
        // TPH-L)
        let mut all = syllable.onset.clone();
        if is_er {
            all.push("R".into());
        }
        all.extend(syllable.coda.iter().cloned());
        if let Some(coda) = Self::consonants(&all, &self.coda, &self.coda_cluster, false) {
            return Some(coda);
        }
        let onset = Self::consonants(&syllable.onset, &self.onset, &self.onset_cluster, false)?;
        let coda = Self::consonants(&syllable.coda, &self.coda, &self.coda_cluster, false)?;
        Some(Stroke(onset.0 | coda.0))
    }

    /// Whether a word's own last syllable is a reduced syllable that could be struck as a
    /// standalone continuation stroke on its own — the same split [`Self::strokes_for`] already
    /// falls back to when folding it onto one stroke doesn't fit (`ended` EPBD/+-D, `breaking`
    /// PWRAEUBG/+-G), computed here unconditionally instead of only when that fold fails. `None`
    /// for a one-syllable word (nothing to split), when the last syllable isn't reduced, or when
    /// it has no bare continuation stroke of its own.
    ///
    /// Returns the index into `sounds` where the stem ends (the last syllable's own onset, if it
    /// has one, counts as part of the stem: a standalone suffix never has an onset of its own,
    /// the same convention `rules/suffixes.toml` already uses, so those sounds belong to
    /// whichever word they'd actually be pronounced with) and the bare (no join key) stroke the
    /// trailing syllable's vowel and coda alone would be struck as.
    pub fn suffix_split(&self, word: &str, sounds: &[String]) -> Option<(usize, Stroke)> {
        let labelled = self.label(word, sounds);
        let syllables = self.syllables(&labelled)?;
        if syllables.len() < 2 {
            return None;
        }
        let last = syllables.last()?;
        if !self.is_reduced(last) {
            return None;
        }
        let suffix = Syllable {
            onset: Vec::new(),
            vowel: last.vowel.clone(),
            coda: last.coda.clone(),
        };
        let stroke = self.continuation(&suffix)?;
        Some((sounds.len() - 1 - last.coda.len(), stroke))
    }

    /// Whether `word` ends in one of `rules/suffixes.toml`'s two-stroke suffixes (`-ism`,
    /// `-ary`/`-ery`/`-ory`, `-ical`, `-logy`, `-cy`/`-sy`, `-ify`/`-fy`) - found by spelling
    /// (`word` ends with the suffix's own letters), unlike [`Self::suffix_split`]'s sound-driven
    /// single-stroke version. That difference is deliberate, not a shortcut: a two-syllable
    /// suffix's second syllable sometimes has its onset consonant migrate backward onto the
    /// *first* stroke rather than staying on the second the way plain syllable rules would place
    /// it (`-ism`'s own `+EUZ/+-PL` ends its first stroke in the `z` that ordinary onset
    /// assignment would otherwise give the second syllable) - `rules/suffixes.toml`'s own chord is
    /// already correct for this, hand-verified against real words, so it is reused as-is rather
    /// than re-derived and risking getting that migration wrong. The longest matching suffix wins
    /// when spelling matches more than one (so a word ending in both a shorter and a longer
    /// registered suffix is read by whichever one actually describes it). `None` if nothing
    /// matches, or a match would consume the whole word (leaving no stem).
    #[must_use]
    pub fn two_stroke_suffix(&self, word: &str) -> Option<(&str, &[Stroke])> {
        self.two_stroke_suffixes
            .iter()
            .filter(|(text, _)| word.len() > text.len() && word.ends_with(text.as_str()))
            .max_by_key(|(text, _)| text.len())
            .map(|(text, strokes)| (text.as_str(), strokes.as_slice()))
    }

    /// The keys one or more leading consonant sounds take through the onset tables, for a prefix
    /// found by an exact match against another already-placed word's own trailing pronunciation
    /// (`never`'s leading "N" in front of `ever`'s own exact sounds) rather than through syllable
    /// analysis the way [`Self::suffix_split`] finds a trailing one. The caller already knows
    /// where the split falls; this only answers what the leading sounds themselves would be
    /// struck as. `None` for no leading sounds at all, or when they don't fit together in one
    /// stroke's own steno order.
    pub fn prefix_keys(&self, word: &str, sounds: &[String], split: usize) -> Option<Stroke> {
        if split == 0 {
            return None;
        }
        let labelled = self.label(word, sounds);
        Self::consonants(&labelled[..split], &self.onset, &self.onset_cluster, false)
    }

    /// The prefix a word starts with, if the rules write one with fixed keys: the keys and how
    /// many sounds they stand for.
    fn prefix(&self, word: &str, sounds: &[String]) -> Option<(Stroke, usize)> {
        self.prefixes.iter().find_map(|(prefix, keys)| {
            if !word.starts_with(prefix.spelling.as_str()) {
                return None;
            }
            prefix.sounds.iter().find_map(|run| {
                let run: Vec<&str> = run.split_whitespace().collect();
                // With a vowel left after it: `disk` is not `dis-` and a `K`
                let matches = run.len() < sounds.len()
                    && run.iter().zip(sounds).all(|(a, b)| *a == unlabelled(b))
                    && sounds[run.len()..]
                        .iter()
                        .any(|s| s.ends_with(|c: char| c.is_ascii_digit()));
                matches.then_some((*keys, run.len()))
            })
        })
    }

    /// The strokes for a word. A reduced syllable's consonants join the stroke before it when
    /// they fit; every other syllable is a stroke of its own. `None` if some syllable can't be
    /// written in a stroke by these rules.
    pub fn outline(&self, word: &str, sounds: &[String]) -> Option<Vec<Stroke>> {
        self.outline_with(word, sounds, false)
    }

    /// How the spelling tells this word from others that sound the same: the outline it takes
    /// when it has to, if that differs from its usual one.
    pub fn variant(&self, word: &str, sounds: &[String]) -> Option<Vec<Stroke>> {
        let alternative = self.outline_with(word, sounds, true)?;
        (Some(&alternative) != self.outline(word, sounds).as_ref()).then_some(alternative)
    }

    fn outline_with(&self, word: &str, sounds: &[String], clash: bool) -> Option<Vec<Stroke>> {
        self.outline_folded(word, sounds, clash, self.fold_any)
    }

    /// The outline with the folded consonants allowed out of steno order (`any`) or not.
    fn outline_folded(
        &self,
        word: &str,
        sounds: &[String],
        clash: bool,
        any: bool,
    ) -> Option<Vec<Stroke>> {
        let labelled = self.label_with(word, sounds, clash);
        let (prefix, skip) = self.prefix(word, &labelled).unzip();
        let syllables = self.syllables(&labelled[skip.unwrap_or(0)..])?;
        let mut strokes = self.strokes_for(&syllables, any)?;
        if let Some(prefix) = prefix {
            // The prefix keys join the first stroke's own left-hand keys, sharing any they both
            // use: `KP` + `PEBGT` is `KPEBGT`, `STK` + `TAPBS` is `STKAPBS`
            strokes.first_mut()?.0 |= prefix.0;
        }
        Some(strokes)
    }

    /// The strokes for a run of syllables, folding reduced ones onto the stroke before.
    fn strokes_for(&self, syllables: &[Syllable], any: bool) -> Option<Vec<Stroke>> {
        let mut strokes: Vec<Stroke> = Vec::new();
        let mut previous: Option<Syllable> = None;
        for syllable in syllables.iter().cloned() {
            if let Some(before) = &previous
                && self.is_reduced(&syllable)
            {
                let mut merged = before.clone();
                let mut added: Vec<String> = syllable.onset.clone();
                if plain(&syllable.vowel) == "ER" {
                    added.push("R".into());
                }
                added.extend(syllable.coda.iter().cloned());
                // Some consonants may land out of order (Plover's -RT for `-ter`, -LT for `-tly`)
                let loose = added.iter().all(|s| self.loose.contains(plain(s)));
                merged.coda.extend(added);
                if let Some(stroke) = self.stroke_for(&merged, any || loose) {
                    *strokes.last_mut()? = stroke;
                    previous = Some(merged);
                    continue;
                }
                // The syllable's first consonants belong to the stem and the rest is a suffix
                // stroke: `ended` EPBD/-D, `breaking` PWRAEUBG/-G, `artist` ART/-FT
                let mut stem = before.clone();
                stem.coda.extend(syllable.onset.iter().cloned());
                let suffix = Syllable {
                    onset: Vec::new(),
                    vowel: syllable.vowel.clone(),
                    coda: syllable.coda.clone(),
                };
                if !syllable.onset.is_empty()
                    && let Some(stroke) = self.stroke_for(&stem, any)
                    && let Some(rest) = self.continuation(&suffix)
                {
                    *strokes.last_mut()? = stroke;
                    strokes.push(rest);
                    previous = Some(vowelless(suffix, rest));
                    continue;
                }
                let rest = self.continuation(&syllable)?;
                strokes.push(rest);
                previous = Some(vowelless(syllable, rest));
                continue;
            }
            strokes.push(self.syllable_stroke(&syllable)?);
            previous = Some(syllable);
        }
        Some(strokes)
    }

    /// The outlines the `lead` and `lean` steps give a word: an unstressed first syllable is not
    /// written as a syllable. `lead` drops one that is only a vowel (`about` PWOUT, `again`
    /// TKPWEPB); `lean` puts the consonants of one onto the left hand of the next stroke when
    /// they come before its own keys (`believe` PWHRAOEF, `deliver` TKHREUFR).
    fn leading(&self, step: Step, word: &str, sounds: &[String]) -> Option<Vec<Stroke>> {
        let labelled = self.label(word, sounds);
        if self.prefix(word, &labelled).is_some() {
            return None;
        }
        let syllables = self.syllables(&labelled)?;
        let first = syllables.first()?;
        if syllables.len() < 2 || !first.vowel.ends_with('0') {
            return None;
        }
        let mut consonants = first.onset.clone();
        consonants.extend(first.coda.iter().cloned());
        let mut rest = self.strokes_for(&syllables[1..], self.fold_any)?;
        match step {
            Step::Lead if consonants.is_empty() => Some(rest),
            Step::Lean if !consonants.is_empty() => {
                let keys = Self::consonants(&consonants, &self.onset, &self.onset_cluster, false)?;
                let next = rest.first_mut()?;
                let own = Stroke(next.0 & Stroke::LEFT);
                let in_order = match (keys.highest(), own.lowest()) {
                    (Some(high), Some(low)) => high <= low,
                    _ => true,
                };
                if !in_order {
                    return None;
                }
                next.0 |= keys.0;
                Some(rest)
            }
            _ => None,
        }
    }

    /// Mark the strokes after the first of a word with the join key, if the theory has one.
    pub fn joined(&self, outline: &mut [Stroke]) {
        if self.join_key {
            for stroke in outline.iter_mut().skip(1) {
                stroke.0 |= Stroke::JOIN;
            }
        }
    }

    /// Read a stroke's keys back into sounds, the same tables read the other way: a miss in
    /// the dictionary is not the end, since the rules can still say what it would have meant.
    /// `None` only when no combination of onset, vowel and coda these rules define fits the
    /// keys down: a true miss, not a guess.
    pub fn resolve_stroke(&self, stroke: Stroke) -> Option<Resolved> {
        let join = stroke.0 & Stroke::JOIN != 0;
        let bits = stroke.0 & !Stroke::JOIN;
        let onset = Self::decompose(bits & Stroke::LEFT, &self.onset_rev)?;
        let vowel_bits = bits & Stroke::VOWELS;
        let right_all = bits & (Stroke::RIGHT | Stroke::STAR);
        let (vowel, coda, marker) = self.decode_right(vowel_bits, right_all)?;
        if onset.is_empty() && vowel.is_none() && coda.is_empty() {
            return None;
        }
        Some(Resolved {
            onset,
            vowel,
            coda,
            marker,
            join,
        })
    }

    /// The vowel and coda for a stroke's vowel-area and right-hand keys. No vowel is a bare
    /// continuation or skeleton stroke of consonants only, valid either way. A star left over
    /// once nothing else claims it is read as the generator's own disambiguation flag (as the
    /// star ladder uses it), not a sound, and the `bool` says so.
    fn decode_right(
        &self,
        vowel_bits: u32,
        right_all: u32,
    ) -> Option<(Option<String>, Vec<String>, bool)> {
        // Try the star as a sound first (`*T` for TH, `*PBG` for NG K); only drop it, and flag
        // it as a leftover marker, if nothing fits with it in.
        for (right, marker) in [(right_all, false), (right_all & !Stroke::STAR, true)] {
            if marker && right == right_all {
                continue;
            }
            for (combo, vowel, coda) in &self.vowel_coda_rev {
                let combo_right = combo.0 & Stroke::RIGHT;
                if combo.0 & Stroke::VOWELS == vowel_bits
                    && right & combo_right == combo_right
                    && let Some(mut tail) = Self::decompose(right & !combo_right, &self.coda_rev)
                {
                    let mut coda = vec![coda.clone()];
                    coda.append(&mut tail);
                    return Some((Some(vowel.clone()), coda, marker));
                }
            }
            if vowel_bits == 0 {
                if let Some(coda) = Self::decompose(right, &self.coda_rev) {
                    return Some((None, coda, marker));
                }
            } else if let Some(label) = self.vowel_rev.get(&Stroke(vowel_bits))
                && let Some(coda) = Self::decompose(right, &self.coda_rev)
            {
                return Some((Some(label.clone()), coda, marker));
            }
        }
        None
    }

    /// A run of keys as a sequence of sounds: at the lowest key still down, the entry that
    /// starts there, covers the most keys, (tied on that) is the fewest sounds, and (tied on
    /// that) is a plain sound rather than a spelling variant of one — a plain label and a dotted
    /// variant can cover the exact same keys (coda `D`'s own bits are also `T`'s "-ed" spelling,
    /// *walked* sounding like "walkt"), and without this a tie fell to whichever happened to
    /// sort last, which is not a decision, just an accident of alphabetical order — consumed,
    /// repeated until none are left. `None` if some key is left with nothing that starts there:
    /// these keys spell no sound this theory defines.
    fn decompose(bits: u32, catalog: &[(Stroke, Vec<String>)]) -> Option<Vec<String>> {
        let mut bits = bits;
        let mut out = Vec::new();
        while bits != 0 {
            let lowest = bits.trailing_zeros();
            let best = catalog
                .iter()
                .filter(|(s, _)| s.0 != 0 && s.0.trailing_zeros() == lowest && bits & s.0 == s.0)
                .max_by_key(|(s, tokens)| {
                    let plain = !tokens.iter().any(|t| t.contains('.'));
                    (s.0.count_ones(), std::cmp::Reverse(tokens.len()), plain)
                })?;
            out.extend(best.1.iter().cloned());
            bits &= !best.0.0;
        }
        Some(out)
    }

    /// Whether `bits` holds only part of `target`'s own keys: some of them down, but not all.
    /// Zero overlap is fine (the candidate isn't trying to be this sound at all, which is just an
    /// unrelated memorized key, not a fragment of anything); full overlap is fine (the sound is
    /// there, whole, however many other keys ride along with it); only a partial hit means a key
    /// combination this one sound needs *together* shows up with a piece of it missing.
    const fn fragments(bits: u32, target: Stroke) -> bool {
        target.0.count_ones() > 1 && bits & target.0 != 0 && bits & target.0 != target.0
    }

    /// The keys a sound token is written with, checking the plain table first and the cluster
    /// table (`[onset_cluster]`/`[coda_cluster]`, built from more than one sound together) next.
    fn token_bits(token: &str, table: &HashMap<String, Stroke>, cluster: &HashMap<String, Stroke>) -> Option<Stroke> {
        table.get(token).or_else(|| cluster.get(token)).copied()
    }

    /// Bits legitimately spoken for by some syllable *other than* `exclude`'s own onset or
    /// coda — a one-stroke brief for a multi-syllable word routinely holds more than one
    /// syllable's sounds at once, and a bit that's already a different syllable's own complete
    /// sound isn't a fragment of anything, no matter which unrelated cluster it also happens to
    /// overlap. Used to clear a stray bit before checking it against the *first* syllable's onset
    /// or the *last* syllable's coda, so a real, complete middle (or, symmetrically, first/last)
    /// cluster can never be mistaken for a partial attempt at some other syllable's own sound —
    /// see [`Self::matches_real_consonants`]'s own doc comment for the `district`/`describe`/
    /// `captain` cases this resolves.
    fn explained_elsewhere(&self, syllables: &[Syllable], exclude: usize) -> u32 {
        syllables
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != exclude)
            .flat_map(|(_, syllable)| {
                syllable
                    .onset
                    .iter()
                    .filter_map(|t| Self::token_bits(t, &self.onset, &self.onset_cluster))
                    .chain(syllable.coda.iter().filter_map(|t| Self::token_bits(t, &self.coda, &self.coda_cluster)))
            })
            .fold(0, |acc, stroke| acc | stroke.0)
    }

    /// Whether this outline's first onset and last coda are each a whole key or cluster toward
    /// the word's own real first and last sound, never half of one. Plover and Lapwing's own
    /// shortest brief for a word is sometimes their own theory's memorized one-off trick that
    /// leaves only part of a key combination the rules only ever use together for *that* sound
    /// (`good`'s Plover brief `KPWAOD` keeps the `P W` of `TKPW`'s own `G` but drops the `T`;
    /// `work`'s `WORB` keeps only the `B` of `-BG`'s own `K`). That fragment still decodes as
    /// *some* sound, just never the one that was meant, which reads as arbitrary rather than
    /// learnable even to someone who already knows the brief. A whole key used as a pure
    /// memorized mnemonic completely unrelated to the word's own sound (`it` on the onset `T`,
    /// which "it" doesn't start with) isn't this — it shares no keys with "it" having no onset at
    /// all, so there's nothing to fragment, and it's exactly as learnable as any other memorized
    /// brief, nothing new to work out. Only a sound's *own* keys, partly present, are rejected.
    ///
    /// This only ever checks the *first* syllable's onset and the *last* syllable's coda — a
    /// one-stroke brief for a longer word routinely condenses every syllable in between onto that
    /// same stroke too, and [`Self::explained_elsewhere`] is what keeps those middle (or, the
    /// mirror case, first/last) syllables' own complete sounds from being mistaken for a fragment
    /// of the one actually being checked, in both directions this used to get wrong:
    /// `district`/`describe` reduce their own real first sound (a "d") to nothing, so their
    /// one-stroke brief's left-hand keys are entirely the *second* syllable's own "tr"/"skr"
    /// cluster — correctly whole, but sharing a key with "d"'s own onset cluster, which used to
    /// get this flagged as a fragment of "d" and "repaired" into dropping a real "t"/"k" out of
    /// "tr"/"skr" (`rules/briefs.toml` pinned both to their correct, literal outline to work
    /// around it). `captain` is the mirror case on the coda side: its own first syllable's real,
    /// legitimate coda "p" shares a key with the word's entirely-dropped final syllable's own "n"
    /// cluster (`-PB`), which used to flag the brief `KAPT` as holding only a fragment of "n" and
    /// offer `KAT` — `cat`'s own natural outline — as a "repair", displacing `cat` onto a starred
    /// variant for no reason related to `cat` at all.
    pub fn matches_real_consonants(&self, outline: &[Stroke], word: &str, sounds: &[String]) -> bool {
        let Some(syllables) = self.syllables(&self.label(word, sounds)) else {
            return true;
        };
        let (Some(first), Some(last), Some(&first_stroke), Some(&last_stroke)) = (
            syllables.first(),
            syllables.last(),
            outline.first(),
            outline.last(),
        ) else {
            return true;
        };
        let last_index = syllables.len() - 1;
        let onset_bits = first_stroke.0 & Stroke::LEFT & !self.explained_elsewhere(&syllables, 0);
        let coda_bits = last_stroke.0 & (Stroke::RIGHT | Stroke::STAR) & !self.explained_elsewhere(&syllables, last_index);
        let no_fragment = |tokens: &[String], bits: u32, table: &HashMap<String, Stroke>, cluster: &HashMap<String, Stroke>| {
            tokens
                .iter()
                .filter_map(|t| Self::token_bits(t, table, cluster))
                .all(|target| !Self::fragments(bits, target))
        };
        no_fragment(&first.onset, onset_bits, &self.onset, &self.onset_cluster)
            && no_fragment(&last.coda, coda_bits, &self.coda, &self.coda_cluster)
    }

    /// The bits of `bits` that are a fragment of one of `tokens`' own multi-key sounds — the part
    /// [`Self::matches_real_consonants`] rejects a candidate for holding.
    fn fragment_bits(tokens: &[String], bits: u32, table: &HashMap<String, Stroke>, cluster: &HashMap<String, Stroke>) -> u32 {
        tokens
            .iter()
            .filter_map(|t| Self::token_bits(t, table, cluster))
            .filter(|target| Self::fragments(bits, *target))
            .fold(0, |acc, target| acc | (bits & target.0))
    }

    /// A shorter, equally legitimate alternative to completing a fragment: drop it instead. A
    /// brief is always free to leave out a whole sound it isn't attempting to represent (`it` on
    /// the bare onset `T`, nothing of "it"'s own first sound at all, is exactly as fine as ever);
    /// what [`Self::matches_real_consonants`] rejects is holding only *part* of a sound's own key
    /// combination. Completing the fragment (the fallback already in place) and removing it
    /// outright are both ways to stop holding only part of it — removing is usually fewer keys,
    /// so this is offered as a second candidate alongside the word's full, correct outline, and
    /// the normal cheapest-and-correct selection picks whichever one is both. `None` if there is
    /// no fragment to drop, or dropping it would leave a stroke with no keys down at all.
    pub fn drop_fragments(&self, outline: &[Stroke], word: &str, sounds: &[String]) -> Option<Vec<Stroke>> {
        let syllables = self.syllables(&self.label(word, sounds))?;
        let (first, last, &first_stroke, &last_stroke) = (
            syllables.first()?,
            syllables.last()?,
            outline.first()?,
            outline.last()?,
        );
        let last_index = syllables.len() - 1;
        let onset_bits = first_stroke.0 & Stroke::LEFT & !self.explained_elsewhere(&syllables, 0);
        let coda_bits = last_stroke.0 & (Stroke::RIGHT | Stroke::STAR) & !self.explained_elsewhere(&syllables, last_index);
        let onset_fragment = Self::fragment_bits(&first.onset, onset_bits, &self.onset, &self.onset_cluster);
        let coda_fragment = Self::fragment_bits(&last.coda, coda_bits, &self.coda, &self.coda_cluster);
        if onset_fragment == 0 && coda_fragment == 0 {
            return None;
        }
        let mut out = outline.to_vec();
        let last_index = out.len() - 1;
        out[0] = Stroke(out[0].0 & !onset_fragment);
        out[last_index] = Stroke(out[last_index].0 & !coda_fragment);
        (out[0].0 != 0 && out[last_index].0 != 0).then_some(out)
    }

    /// A spelling-mode run (`Stroke::SPELL` on its first stroke) read back letter by letter: no
    /// vowel-guessing ambiguity the phonetic fallback has, since each letter was struck
    /// explicitly. `None` if any stroke's bare chord (join/spell bits aside) isn't in
    /// `rules/spelling.toml`. See `docs/spelling-mode.md`.
    fn resolve_spelling(&self, outline: &[Stroke]) -> Option<String> {
        let mut text = String::new();
        for &stroke in outline {
            let bits = Stroke(stroke.0 & !(Stroke::JOIN | Stroke::SPELL));
            text.push_str(self.spelling.get(&bits)?);
        }
        Some(text)
    }

    /// The reverse of [`Self::resolve_spelling`]: a word written out as its own spelling-mode
    /// outline, one letter-chord stroke per character, the first marked `Stroke::SPELL` and every
    /// one after it `Stroke::JOIN` like any other continuation. `None` for a word with any
    /// character `rules/spelling.toml` has no chord for (anything outside plain a-z) — the same
    /// condition that makes a word untypeable by spelling mode at all.
    pub fn spell_word(&self, word: &str) -> Option<Vec<Stroke>> {
        word.chars()
            .enumerate()
            .map(|(i, ch)| {
                let letter = ch.to_lowercase().next()?.to_string();
                let (&bits, _) = self.spelling.iter().find(|(_, l)| **l == letter)?;
                let marker = if i == 0 { Stroke::SPELL } else { Stroke::JOIN };
                Some(Stroke(bits.0 | marker))
            })
            .collect()
    }

    /// The best reading the rules can give a stroke outline: each stroke's sounds, read back
    /// and spelled plainly (not real orthography, just enough to read), continuation strokes
    /// run on and others started with a space. `None` only once some stroke in it cannot be
    /// read at all.
    ///
    /// A run that starts with the spelling-mode marker (`Stroke::SPELL`) is read entirely
    /// differently: see [`Self::resolve_spelling`].
    pub fn resolve(&self, outline: &[Stroke], stems: Option<&HashMap<Stroke, String>>) -> Option<String> {
        if outline.first().is_some_and(|s| s.0 & Stroke::SPELL != 0) {
            return self.resolve_spelling(outline);
        }
        let mut text = String::new();
        // If this run's first stroke is itself an already-placed single-stroke word, use that
        // word's own real text as the stem instead of re-deriving it from raw phonemes — a
        // continuation after a known word (`SKP/+-R`, "and" plus "-er") must read as that word's
        // own text ("ander"), never as its outline's own bare sounds mistaken for a true miss
        // (`SKP` alone reads as "sp", garbling "and" entirely). Only for a genuine multi-stroke
        // run: a single-stroke outline that matches a placed word would already have been a
        // dictionary hit before `resolve` is ever reached, so `outline.len() > 1` here always
        // means a real continuation, never the word's own first and only stroke.
        let skip = if outline.len() > 1
            && let Some(&first) = outline.first()
            && let Some(stem) = stems.and_then(|stems| stems.get(&first))
        {
            // The stem's own trailing silent `e` (if any) needs to see the very next piece's
            // first letter before it can be written at all (`have` + `ing` -> `having`, not
            // `haveing`) — so that piece is computed here, before the stem is pushed, rather than
            // by the general loop below.
            if let Some(&next) = outline.get(1) {
                let (next_text, needs_space) = self.piece_text(next, true)?;
                let starts_with_vowel = next_text.starts_with(['a', 'e', 'i', 'o', 'u']);
                text.push_str(drop_silent_e(stem, starts_with_vowel));
                if needs_space {
                    text.push(' ');
                }
                text.push_str(&next_text);
                2
            } else {
                text.push_str(stem);
                1
            }
        } else {
            0
        };
        for (i, &stroke) in outline.iter().enumerate().skip(skip) {
            let (piece, needs_space) = self.piece_text(stroke, i > 0)?;
            if needs_space && !text.is_empty() {
                text.push(' ');
            }
            text.push_str(&piece);
        }
        Some(text)
    }

    /// A single stroke's own text within a continuation run: a `rules/suffixes.toml` match if
    /// `is_continuation` and the stroke is exactly one of those chords ("ed", not a generic
    /// phonetic guess at bare consonants, "d") — the same reading it would have as the
    /// dictionary's own standalone entry for this suffix, now unconditional instead of only when
    /// some specific composed outline happens to already be a literal dictionary entry — the
    /// plain phonetic read-back otherwise. The second item says whether a space belongs before
    /// this piece when it is not the very first thing assembled: never, for a suffix match or any
    /// other joined stroke.
    fn piece_text(&self, stroke: Stroke, is_continuation: bool) -> Option<(String, bool)> {
        if is_continuation && stroke.0 & Stroke::JOIN != 0 && let Some(suffix) = self.suffixes.get(&stroke) {
            return Some((suffix.clone(), false));
        }
        let resolved = self.resolve_stroke(stroke)?;
        let mut text = String::new();
        for sound in resolved
            .onset
            .iter()
            .chain(resolved.vowel.iter())
            .chain(resolved.coda.iter())
        {
            text.push_str(default_spelling(sound));
        }
        if resolved.marker {
            text.push('*');
        }
        Some((text, !resolved.join))
    }

    /// The source of a keymux WASM plugin that reads a stroke back exactly the way
    /// [`Self::resolve`] does, generated from this theory's own tables instead of hand-copied:
    /// the two can never drift apart, because there is only one implementation of the rule.
    /// The ABI boilerplate (`alloc`/`fallback`) is the one part every plugin needs regardless
    /// of theory, so it is a fixed template, not generated from data.
    #[must_use]
    #[allow(clippy::too_many_lines)] // one template, emitting one generated file: splitting
    // the format! string across helper functions would hurt readability, not help it
    /// `stems` is every single-stroke word in the generated dictionary, by its own bare stroke
    /// (see [`crate::generate::single_stroke_stems`]) — embedded so the deployed plugin's own
    /// fallback can read a known word's real text instead of its raw phonemes when it's the first
    /// stroke of an otherwise-unmatched continuation run (`SKP/+-R`, "and" plus "-er", reading as
    /// "ander" instead of "sper"). Regenerate and redeploy whenever the dictionary itself changes,
    /// not only when a rule table does — this table is a snapshot of which word is on which
    /// single stroke right now, not a fixed theory table like `SUFFIXES`/`SPELLING`.
    pub fn generate_plugin(&self, stems: &HashMap<Stroke, String>) -> String {
        use std::fmt::Write;

        fn tokens_literal(tokens: &[String]) -> String {
            let inner: Vec<String> = tokens.iter().map(|t| format!("{t:?}")).collect();
            format!("&[{}]", inner.join(", "))
        }

        let mut onset_rev = String::new();
        for (stroke, tokens) in &self.onset_rev {
            let _ = writeln!(onset_rev, "    ({}, {}),", stroke.0, tokens_literal(tokens));
        }
        let mut coda_rev = String::new();
        for (stroke, tokens) in &self.coda_rev {
            let _ = writeln!(coda_rev, "    ({}, {}),", stroke.0, tokens_literal(tokens));
        }
        let mut vowel_rev: Vec<(&Stroke, &String)> = self.vowel_rev.iter().collect();
        vowel_rev.sort_by_key(|(s, _)| s.0);
        let mut vowel_rev_src = String::new();
        for (stroke, label) in vowel_rev {
            let _ = writeln!(vowel_rev_src, "    ({}, {label:?}),", stroke.0);
        }
        let mut vowel_coda_rev = String::new();
        for (combo, vowel, coda) in &self.vowel_coda_rev {
            let _ = writeln!(
                vowel_coda_rev,
                "    ({}, {vowel:?}, {coda:?}),",
                combo.0
            );
        }
        let mut spelling: Vec<(&Stroke, &String)> = self.spelling.iter().collect();
        spelling.sort_by_key(|(s, _)| s.0);
        let mut spelling_src = String::new();
        for (stroke, letter) in spelling {
            let _ = writeln!(spelling_src, "    ({}, {letter:?}),", stroke.0);
        }
        let mut suffixes: Vec<(&Stroke, &String)> = self.suffixes.iter().collect();
        suffixes.sort_by_key(|(s, _)| s.0);
        let mut suffixes_src = String::new();
        for (stroke, text) in suffixes {
            let _ = writeln!(suffixes_src, "    ({}, {text:?}),", stroke.0);
        }
        let mut stems: Vec<(&Stroke, &String)> = stems.iter().collect();
        stems.sort_by_key(|(s, _)| s.0);
        let mut stems_src = String::new();
        for (stroke, word) in stems {
            let _ = writeln!(stems_src, "    ({}, {word:?}),", stroke.0);
        }

        format!(
            r##"//! A phonetic best guess for a stroke that missed every dictionary, generated by
//! `lyk-steno-theory` (`cargo run -- plugin`) from the same tables `src/rules.rs` uses to write
//! and read back every other word: this file can never drift from the theory it reads, because
//! there is no second, hand-maintained copy of the rule for it to drift from. Do not edit this
//! file by hand; it is overwritten the next time it is generated.
//!
//! It reads a stroke's own keys back into a plain, readable spelling: not real English
//! spelling, since the stroke alone can't say how a word is actually spelled, just close
//! enough to be read. A stroke with no vowel key down is a bare consonant skeleton (the start
//! of a word a join-key continuation finishes, or all there really is) and reads with no vowel
//! sound invented for it.

const LEFT: u32 = {left};
const VOWELS: u32 = {vowels};
const STAR: u32 = {star};
const RIGHT: u32 = {right};
const JOIN: u32 = {join};
const SPELL: u32 = {spell};

const KEYS: &[u8; 23] = b"#STKPWHRAO*EUFRPBLGTSDZ";
const RIGHT_START: usize = 13;

const ONSET_REV: &[(u32, &[&str])] = &[
{onset_rev}];
const CODA_REV: &[(u32, &[&str])] = &[
{coda_rev}];
const VOWEL_REV: &[(u32, &str)] = &[
{vowel_rev}];
const VOWEL_CODA_REV: &[(u32, &str, &str)] = &[
{vowel_coda_rev}];
const SPELLING: &[(u32, &str)] = &[
{spelling}];
const SUFFIXES: &[(u32, &str)] = &[
{suffixes}];
const STEMS: &[(u32, &str)] = &[
{stems}];

/// A sound without its stress digit or spelling variant: `UW.oo1` is `UW`.
fn plain(sound: &str) -> &str {{
    sound
        .trim_end_matches(|c: char| c.is_ascii_digit())
        .split('.')
        .next()
        .unwrap_or(sound)
}}

/// English's own rule for a stem ending in a silent `e`: drop it before a suffix that starts with
/// a vowel letter. Mirrors `rules.rs`'s `drop_silent_e`.
fn drop_silent_e(stem: &str, next_starts_with_vowel: bool) -> &str {{
    let bytes = stem.as_bytes();
    if next_starts_with_vowel
        && bytes.len() > 1
        && bytes[bytes.len() - 1] == b'e'
        && !matches!(bytes[bytes.len() - 2], b'a' | b'e' | b'i' | b'o' | b'u')
    {{
        &stem[..stem.len() - 1]
    }} else {{
        stem
    }}
}}

/// A plain, readable spelling for a sound: not real orthography, just enough that a resolved
/// reading can be read at all. Mirrors `rules.rs`'s `default_spelling`.
fn default_spelling(sound: &str) -> &str {{
    match plain(sound) {{
        "P" => "p",
        "B" => "b",
        "T" => "t",
        "D" => "d",
        "K" => "k",
        "G" => "g",
        "M" => "m",
        "N" => "n",
        "NG" => "ng",
        "F" => "f",
        "V" => "v",
        "TH" => "th",
        "DH" => "dh",
        "S" => "s",
        "Z" => "z",
        "SH" => "sh",
        "ZH" => "zh",
        "CH" => "ch",
        "JH" => "j",
        "HH" => "h",
        "L" => "l",
        "R" => "r",
        "W" => "w",
        "Y" => "y",
        "IY" => "ee",
        "IH" => "ih",
        "EY" => "ay",
        "EH" => "eh",
        "AE" => "a",
        "AA" => "ah",
        "AH" | "UH" => "uh",
        "AO" => "aw",
        "OW" => "oh",
        "UW" => "oo",
        "AY" => "eye",
        "AW" => "ow",
        "OY" => "oy",
        "ER" => "er",
        other => other,
    }}
}}

/// A run of keys as a sequence of sounds: at the lowest key still down, the entry that starts
/// there, covers the most keys, (tied on that) is the fewest sounds, and (tied on that) is a
/// plain sound rather than a spelling variant of one, consumed, repeated until none are left.
/// `None` if some key is left with nothing that starts there.
fn decompose(bits: u32, catalog: &[(u32, &[&str])]) -> Option<Vec<String>> {{
    let mut bits = bits;
    let mut out = Vec::new();
    while bits != 0 {{
        let lowest = bits.trailing_zeros();
        let best = catalog
            .iter()
            .filter(|(s, _)| *s != 0 && s.trailing_zeros() == lowest && bits & s == *s)
            .max_by_key(|(s, tokens)| {{
                let plain = !tokens.iter().any(|t| t.contains('.'));
                (s.count_ones(), std::cmp::Reverse(tokens.len()), plain)
            }})?;
        out.extend(best.1.iter().map(ToString::to_string));
        bits &= !best.0;
    }}
    Some(out)
}}

/// The vowel and coda for a stroke's vowel-area and right-hand keys.
fn decode_right(vowel_bits: u32, right_all: u32) -> Option<(Option<String>, Vec<String>, bool)> {{
    for (right, marker) in [(right_all, false), (right_all & !STAR, true)] {{
        if marker && right == right_all {{
            continue;
        }}
        for (combo, vowel, coda) in VOWEL_CODA_REV {{
            let combo_right = combo & RIGHT;
            if combo & VOWELS == vowel_bits && right & combo_right == combo_right {{
                if let Some(mut tail) = decompose(right & !combo_right, CODA_REV) {{
                    let mut coda_v = vec![(*coda).to_string()];
                    coda_v.append(&mut tail);
                    return Some((Some((*vowel).to_string()), coda_v, marker));
                }}
            }}
        }}
        if vowel_bits == 0 {{
            if let Some(coda) = decompose(right, CODA_REV) {{
                return Some((None, coda, marker));
            }}
        }} else {{
            let label = VOWEL_REV.iter().find(|(b, _)| *b == vowel_bits).map(|(_, l)| *l);
            if let Some(label) = label {{
                if let Some(coda) = decompose(right, CODA_REV) {{
                    return Some((Some(label.to_string()), coda, marker));
                }}
            }}
        }}
    }}
    None
}}

/// A stroke's onset, vowel, coda, leftover-star marker and join flag, or `None` for a stroke
/// with no reading at all (nothing in it any onset, vowel or coda table recognizes).
fn resolve_stroke(bits_with_join: u32) -> Option<(Vec<String>, Option<String>, Vec<String>, bool, bool)> {{
    let join = bits_with_join & JOIN != 0;
    let bits = bits_with_join & !JOIN;
    let onset = decompose(bits & LEFT, ONSET_REV)?;
    let vowel_bits = bits & VOWELS;
    let right_all = bits & (RIGHT | STAR);
    let (vowel, coda, marker) = decode_right(vowel_bits, right_all)?;
    if onset.is_empty() && vowel.is_none() && coda.is_empty() {{
        return None;
    }}
    Some((onset, vowel, coda, marker, join))
}}

/// Parse one stroke's notation (`KAT`, `-T`, `+AO`, `~S`) into its bits.
fn parse_stroke(text: &str) -> Option<u32> {{
    let (mut bits, text) = match text.strip_prefix('+') {{
        Some(rest) => (JOIN, rest),
        None => match text.strip_prefix('~') {{
            Some(rest) => (SPELL, rest),
            None => (0, text),
        }},
    }};
    let mut cursor = 0usize;
    for c in text.bytes() {{
        if c == b'-' {{
            cursor = cursor.max(RIGHT_START);
            continue;
        }}
        let at = KEYS[cursor..].iter().position(|&k| k == c)? + cursor;
        bits |= 1 << at;
        cursor = at + 1;
    }}
    (bits & !(JOIN | SPELL) != 0).then_some(bits)
}}

/// A spelling-mode run (`SPELL` on its first stroke) read back letter by letter instead of
/// sounded out: see `docs/spelling-mode.md` and `Rules::resolve_spelling`, which this mirrors.
fn resolve_spelling(outline: &str) -> Option<String> {{
    let mut text = String::new();
    for part in outline.split('/') {{
        let bits = parse_stroke(part)? & !(JOIN | SPELL);
        let &(_, letter) = SPELLING.iter().find(|&&(b, _)| b == bits)?;
        text.push_str(letter);
    }}
    Some(text)
}}

/// A single stroke's own text within a continuation run: a `rules/suffixes.toml` match if
/// `is_continuation` and the stroke is exactly one of those chords, the plain phonetic read-back
/// otherwise. The second item says whether a space belongs before this piece when it is not the
/// very first thing assembled: never, for a suffix match or any other joined stroke. Mirrors
/// `rules.rs`'s `Rules::piece_text`.
fn piece_text(bits: u32, is_continuation: bool) -> Option<(String, bool)> {{
    if is_continuation && bits & JOIN != 0 {{
        if let Some(&(_, suffix)) = SUFFIXES.iter().find(|&&(b, _)| b == bits) {{
            return Some((suffix.to_string(), false));
        }}
    }}
    let (onset, vowel, coda, marker, join) = resolve_stroke(bits)?;
    let mut text = String::new();
    for sound in onset.iter().chain(vowel.iter()).chain(coda.iter()) {{
        text.push_str(default_spelling(sound));
    }}
    if marker {{
        text.push('*');
    }}
    Some((text, !join))
}}

/// This plugin's actual job: a stroke outline, in Plover notation (one stroke, or several
/// separated by `/`), read back the way the theory itself would read it.
fn resolve(outline: &str) -> Option<String> {{
    let parts: Vec<&str> = outline.split('/').collect();
    let first_bits = parse_stroke(parts.first()?)?;
    if first_bits & SPELL != 0 {{
        return resolve_spelling(outline);
    }}
    let mut text = String::new();
    // If this run's first stroke is itself an already-placed single-stroke word, use that
    // word's own real text as the stem instead of re-deriving it from raw phonemes: mirrors
    // `Rules::resolve`.
    let mut skip = 0;
    if parts.len() > 1 {{
        if let Some(&(_, stem)) = STEMS.iter().find(|&&(b, _)| b == first_bits) {{
            // The stem's own trailing silent `e` (if any) needs to see the very next piece's
            // first letter before it can be written at all (`have` + `ing` -> `having`, not
            // `haveing`): mirrors `Rules::resolve`.
            if let Some(part) = parts.get(1) {{
                let next_bits = parse_stroke(part)?;
                let (next_text, needs_space) = piece_text(next_bits, true)?;
                let starts_with_vowel = next_text.starts_with(['a', 'e', 'i', 'o', 'u']);
                text.push_str(drop_silent_e(stem, starts_with_vowel));
                if needs_space {{
                    text.push(' ');
                }}
                text.push_str(&next_text);
                skip = 2;
            }} else {{
                text.push_str(stem);
                skip = 1;
            }}
        }}
    }}
    for (i, part) in parts.iter().enumerate().skip(skip) {{
        let bits = parse_stroke(part)?;
        let (piece, needs_space) = piece_text(bits, i > 0)?;
        if needs_space && !text.is_empty() {{
            text.push(' ');
        }}
        text.push_str(&piece);
    }}
    Some(text)
}}

// --- The interface keymux's plugin loader calls. See docs/plugins/README.md. ---

#[no_mangle]
pub extern "C" fn alloc(len: i32) -> i32 {{
    let mut buf: Vec<u8> = vec![0u8; len.max(0) as usize];
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr as i32
}}

#[no_mangle]
pub extern "C" fn fallback(ptr: i32, len: i32) -> i64 {{
    let bytes = unsafe {{ std::slice::from_raw_parts(ptr as *const u8, len.max(0) as usize) }};
    let Ok(outline) = std::str::from_utf8(bytes) else {{ return -1; }};
    let Some(text) = resolve(outline) else {{ return -1; }};
    let mut out = text.into_bytes();
    let out_len = out.len();
    let out_ptr = out.as_mut_ptr();
    std::mem::forget(out);
    (i64::from(out_ptr as i32) << 32) | i64::from(out_len as i32)
}}

#[no_mangle]
pub extern "C" fn joins(bits: i64) -> i32 {{
    i32::from(bits as u32 & JOIN != 0)
}}
"##,
            left = Stroke::LEFT,
            vowels = Stroke::VOWELS,
            star = Stroke::STAR,
            right = Stroke::RIGHT,
            join = Stroke::JOIN,
            spell = Stroke::SPELL,
            onset_rev = onset_rev,
            coda_rev = coda_rev,
            vowel_rev = vowel_rev_src,
            vowel_coda_rev = vowel_coda_rev,
            spelling = spelling_src,
            suffixes = suffixes_src,
            stems = stems_src,
        )
    }
}

/// What a stroke's keys spell out, read back through the tables that write them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub onset: Vec<String>,
    pub vowel: Option<String>,
    pub coda: Vec<String>,
    /// A star down that no coda or vowel combination claims: a disambiguation flag, not a sound
    pub marker: bool,
    pub join: bool,
}

/// English's own rule for a stem ending in a silent `e`: drop it before a suffix that starts with
/// a vowel letter (`have` + `ing` -> `having`, not `haveing`) so a continuation after a known stem
/// (see [`Rules::resolve`]) reads as real spelling, not naive concatenation. Skipped when the `e`
/// isn't silent - already preceded by another vowel (`agree` + `ing` stays `agreeing`; `argue` +
/// `ing` still drops only the one trailing `e`, `arguing`). Known, unhandled exceptions: English
/// keeps the `e` in a few cases to stay visually distinct from another word (`dye` + `ing` =
/// `dyeing`, not `dying`; `singe` + `ing` = `singeing`, not `singing`) - not special-cased here.
fn drop_silent_e(stem: &str, next_starts_with_vowel: bool) -> &str {
    let bytes = stem.as_bytes();
    if next_starts_with_vowel
        && bytes.len() > 1
        && bytes[bytes.len() - 1] == b'e'
        && !matches!(bytes[bytes.len() - 2], b'a' | b'e' | b'i' | b'o' | b'u')
    {
        &stem[..stem.len() - 1]
    } else {
        stem
    }
}

/// A plain, readable spelling for a sound: not real orthography (that needs the word's own
/// spelling, which a stroke alone does not carry), just enough that a resolved reading can be
/// read at all.
fn default_spelling(sound: &str) -> &str {
    match plain(sound) {
        "P" => "p",
        "B" => "b",
        "T" => "t",
        "D" => "d",
        "K" => "k",
        "G" => "g",
        "M" => "m",
        "N" => "n",
        "NG" => "ng",
        "F" => "f",
        "V" => "v",
        "TH" => "th",
        "DH" => "dh",
        "S" => "s",
        "Z" => "z",
        "SH" => "sh",
        "ZH" => "zh",
        "CH" => "ch",
        "JH" => "j",
        "HH" => "h",
        "L" => "l",
        "R" => "r",
        "W" => "w",
        "Y" => "y",
        "IY" => "ee",
        "IH" => "ih",
        "EY" => "ay",
        "EH" => "eh",
        "AE" => "a",
        "AA" => "ah",
        "AH" | "UH" => "uh",
        "AO" => "aw",
        "OW" => "oh",
        "UW" => "oo",
        "AY" => "eye",
        "AW" => "ow",
        "OY" => "oy",
        "ER" => "er",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stroke::{parse_outline, render_outline};

    fn sounds(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    fn write(word: &str, text: &str) -> String {
        Rules::built_in()
            .unwrap()
            .outline(word, &sounds(text))
            .map_or_else(|| "-".to_owned(), |o| render_outline(&o))
    }

    #[test]
    fn a_syllable_is_its_onset_vowel_and_coda() {
        assert_eq!(write("cat", "K AE1 T"), "KAT");
        assert_eq!(write("stop", "S T AA1 P"), "STOP");
        assert_eq!(write("dog", "D AO1 G"), "TKOG");
    }

    #[test]
    fn spelling_picks_between_ways_to_write_a_sound() {
        // The same vowel is two keys written `oo` and three written `u`
        assert_eq!(write("moon", "M UW1 N"), "PHAOPB");
        assert_eq!(write("tune", "T UW1 N"), "TAOUPB");
        assert_eq!(write("when", "W EH1 N"), "WHEPB");
    }

    #[test]
    fn an_unstressed_syllable_joins_the_stroke_before_it() {
        assert_eq!(write("living", "L IH1 V IH0 NG"), "HREUFG");
        assert_eq!(write("special", "S P EH1 SH AH0 L"), "SPERBL");
    }

    #[test]
    fn a_suffix_that_does_not_fit_is_a_stroke_of_its_consonants() {
        // The stem keeps its consonant and the ending is a bare continuation stroke
        assert_eq!(write("ended", "EH1 N D AH0 D"), "EPBD/-D");
        assert_eq!(write("breaking", "B R EY1 K IH0 NG"), "PWRAEUBG/-G");
        assert_eq!(write("hidden", "HH IH1 D AH0 N"), "HEUD/-PB");
        // A reduced `er` that does not fold is -R alone
        assert_eq!(write("harder", "HH AA1 R D ER0"), "HARD/-R");
    }

    #[test]
    fn two_stroke_suffixes_are_found_by_spelling_not_sound() {
        let rules = Rules::built_in().unwrap();
        // "ism" is the longer, correct match over any shorter coincidental suffix spelling.
        let (text, strokes) = rules.two_stroke_suffix("cubism").unwrap();
        assert_eq!(text, "ism");
        assert_eq!(render_outline(strokes), "+EUZ/+-PL");
        // "ify" the same way.
        let (text, strokes) = rules.two_stroke_suffix("quantify").unwrap();
        assert_eq!(text, "ify");
        assert_eq!(render_outline(strokes), "+U/+TPAOEU");
        // A word that IS the suffix, with nothing left for a stem, is not a match.
        assert!(rules.two_stroke_suffix("ism").is_none());
        // A word that merely contains the letters, not as a true suffix match at all, still
        // matches by spelling - this function only ever answers "could this be composed this
        // way," `generate.rs`'s own stem lookup (does a placed word exist with this exact
        // remaining spelling) is what actually gates whether a backup entry gets built.
        assert!(rules.two_stroke_suffix("prism").is_some());
        // No suffix spelling present at all.
        assert!(rules.two_stroke_suffix("hello").is_none());
    }

    #[test]
    fn consonants_only_join_the_stroke_before_in_steno_order() {
        assert_eq!(write("goodness", "G UH1 D N AH0 S"), "TKPWAOD/-PBS");
        // Except a lone L or R, which may land before the stem's last key as in Plover
        assert_eq!(write("lately", "L EY1 T L IY0"), "HRAEULT");
        assert_eq!(write("color", "K AH1 L ER0"), "KORL");
    }

    #[test]
    fn a_prefix_has_keys_of_its_own() {
        assert_eq!(write("expect", "IH0 K S P EH1 K T"), "KPEBGT");
        assert_eq!(write("exist", "IH0 G Z IH1 S T"), "KPEUFT");
        assert_eq!(write("distance", "D IH1 S T AH0 N S"), "STKUPBS");
        // Not a prefix when nothing with a vowel follows
        assert_eq!(write("disk", "D IH1 S K"), "TKEUFBG");
    }

    #[test]
    fn a_reduced_first_syllable_takes_its_spelled_vowel() {
        assert_eq!(write("believe", "B IH0 L IY1 V"), "PWE/HRAOEF");
        assert_eq!(write("about", "AH0 B AW1 T"), "A/PWOUT");
        // Not when the vowel is stressed
        assert_eq!(write("bee", "B IY1"), "PWAOE");
    }

    #[test]
    fn a_sound_after_an_r_vowel_shares_the_r_key() {
        assert_eq!(write("serve", "S ER1 V"), "SEFRB");
        assert_eq!(write("march", "M AA1 R CH"), "PHAFRPB");
        assert_eq!(write("harsh", "HH AA1 R SH"), "HARB");
        // Only after the R
        assert_eq!(write("survive", "S ER0 V AY1 V"), "SUR/SRAOEUF");
    }

    #[test]
    fn a_stressed_short_vowel_keeps_a_consonant() {
        assert_eq!(write("every", "EH1 V R IY0"), "EF/REU");
        assert_eq!(write("better", "B EH1 T ER0"), "PWERT");
    }

    #[test]
    fn a_miss_is_read_back_into_sounds() {
        let rules = Rules::built_in().unwrap();
        let read = |text: &str| rules.resolve(&parse_outline(text).unwrap(), None);
        // A plain stroke that spells no word the rules know, read back the same way the rules
        // would have written it: onset, vowel, coda
        assert_eq!(read("STKPW"), Some("z".to_owned()));
        // Two labels can share the exact same keys (a plain sound and a spelling variant, or a
        // sound and its silent-glide cluster); the read-back always picks the same one
        for _ in 0..5 {
            assert_eq!(read("A/+A"), Some("aa".to_owned()));
        }
    }

    #[test]
    fn a_suffix_stroke_reads_as_its_own_spelling_after_any_miss_not_just_a_known_one() {
        let rules = Rules::built_in().unwrap();
        let read = |text: &str| rules.resolve(&parse_outline(text).unwrap(), None);
        // `STKPW` ("z") has no dictionary entry at all, let alone one composed with any of
        // these suffixes - the continuation stroke must still read as the suffix's own real
        // spelling, not a generic phonetic guess at its bare consonants (`-D` alone would
        // otherwise read as plain "d", not "ed").
        assert_eq!(read("STKPW/+-D"), Some("zed".to_owned()));
        assert_eq!(read("STKPW/+-G"), Some("zing".to_owned()));
        assert_eq!(read("STKPW/+-S"), Some("zs".to_owned()));
        assert_eq!(read("STKPW/+-L"), Some("zly".to_owned()));
        assert_eq!(read("STKPW/+-PBS"), Some("zness".to_owned()));
        assert_eq!(read("STKPW/+*T"), Some("zth".to_owned()));
        // A suffix struck as the *first* stroke (no join bit set, since nothing precedes it) is
        // not this case at all - it reads as the generic phonetic guess at its bare consonants,
        // same as any other bare consonant-only stroke would (and, in a real dictionary, this
        // exact outline is already a literal entry for the suffix itself - see
        // `rules/suffixes.toml` - so `resolve` is never even reached for it there).
        assert_eq!(read("-D"), Some("d".to_owned()));
    }

    #[test]
    fn a_run_whose_first_stroke_is_a_known_word_reads_as_that_word_plus_the_suffix() {
        let rules = Rules::built_in().unwrap();
        let skp = parse_outline("SKP").unwrap()[0];
        let pwhr = parse_outline("PWHR").unwrap()[0];
        let mut stems = HashMap::new();
        stems.insert(skp, "and".to_owned());
        stems.insert(pwhr, "build".to_owned());
        let read = |text: &str| rules.resolve(&parse_outline(text).unwrap(), Some(&stems));
        // `SKP` alone has no phonetic reading that spells "and" at all (it's a memorized brief,
        // not derived from "and"'s own sounds) - without the stem lookup this used to read back
        // as whatever `SKP` means phonetically, garbling the word entirely.
        assert_eq!(read("SKP/+-R"), Some("ander".to_owned()));
        assert_eq!(read("PWHR/+-D"), Some("builded".to_owned()));
        // With no stems table at all (the plain phonetic fallback, e.g. a stroke nothing has
        // ever placed), the first stroke is read phonetically same as always.
        assert_eq!(rules.resolve(&parse_outline("SKP/+-R").unwrap(), None), Some("sper".to_owned()));
        // A single stroke that happens to be a known word's own outline is never routed through
        // the stem table - a real dictionary would have already answered it directly, so this
        // case should never actually reach `resolve` with `outline.len() == 1`.
        assert_eq!(read("SKP"), Some("sp".to_owned()));
    }

    #[test]
    fn a_stems_own_silent_e_drops_before_a_vowel_initial_continuation() {
        let rules = Rules::built_in().unwrap();
        let sr = parse_outline("SR").unwrap()[0];
        let mut stems = HashMap::new();
        let read = |rules: &Rules, stems: &HashMap<Stroke, String>, text: &str| {
            rules.resolve(&parse_outline(text).unwrap(), Some(stems))
        };
        // "have" + the standalone "-ing" suffix used to concatenate literally ("haveing"); English
        // drops a stem's own trailing silent `e` before a vowel-initial suffix.
        stems.insert(sr, "have".to_owned());
        assert_eq!(read(&rules, &stems, "SR/+-G"), Some("having".to_owned()));
        // A consonant-initial suffix never drops it ("entirely", not "entirly" - not a real stem
        // here, but the same mechanism, exercised through a made-up one).
        stems.insert(sr, "entire".to_owned());
        assert_eq!(read(&rules, &stems, "SR/+-L"), Some("entirely".to_owned()));
        // An `e` that wasn't silent to begin with (preceded by another vowel) never drops either.
        stems.insert(sr, "agree".to_owned());
        assert_eq!(read(&rules, &stems, "SR/+-G"), Some("agreeing".to_owned()));
    }

    #[test]
    fn a_word_with_no_pronunciation_can_be_spelled_letter_by_letter() {
        let rules = Rules::built_in().unwrap();
        let read = |text: &str| rules.resolve(&parse_outline(text).unwrap(), None);
        // "stenography" has no CMUdict entry at all, so the phonetic rules have nothing to
        // generate from; spelling mode (the `~` marker on the first stroke, `+` continuing every
        // one after it, same as any other multi-stroke word) reads it back letter-exact instead.
        assert_eq!(
            read("~S/+T/+E/+TPH/+O/+TKPW/+R/+A/+P/+H/+KWH"),
            Some("stenography".to_owned())
        );
        // A chord with no entry in rules/spelling.toml is a true miss, same as any other.
        assert_eq!(read("~*"), None);
    }

    #[test]
    fn spell_word_is_resolve_spellings_exact_inverse() {
        let rules = Rules::built_in().unwrap();
        // The encode direction: given just the word, not a stroke sequence someone already wrote
        // out by hand, produce the same outline `docs/spelling-mode.md` hand-writes for
        // "stenography" - a word with no pronunciation at all for the phonetic rules to run from.
        let outline = rules.spell_word("stenography").unwrap();
        assert_eq!(render_outline(&outline), "~S/+T/+E/+TPH/+O/+TKPW/+R/+A/+P/+H/+KWH");
        // Round-trips back through the decode direction exactly.
        assert_eq!(rules.resolve(&outline, None), Some("stenography".to_owned()));
        // A character spelling mode has no chord for (anything but a-z) is a true miss, same as
        // a stroke chord with no entry in the table.
        assert_eq!(rules.spell_word("o'clock"), None);
    }

    #[test]
    fn only_a_truly_empty_stroke_is_a_true_miss() {
        let rules = Rules::built_in().unwrap();
        // Every key this theory uses means something alone or in some combination; what has no
        // reading at all is a stroke with nothing left once the star and join bits are set
        // aside: no onset, no vowel, no coda.
        assert_eq!(rules.resolve_stroke(Stroke(Stroke::STAR)), None);
        assert_eq!(rules.resolve_stroke(Stroke(Stroke::JOIN)), None);
        assert_eq!(
            rules.resolve_stroke(Stroke(Stroke::STAR | Stroke::JOIN)),
            None
        );
    }

    #[test]
    fn an_earlier_syllables_own_sound_is_never_mistaken_for_a_fragment_of_a_later_one() {
        let rules = Rules::built_in().unwrap();
        // "captain"'s own first syllable really does end in a "p" (K AE1 | P T AH0 N); that "p"
        // happens to share a key with the entirely-dropped final syllable's "n" cluster (`-PB`),
        // which used to make `KAPT` look like it held only a fragment of "n" - wrongly, since the
        // brief isn't attempting "n" at all. See `rules/briefs.toml`'s `district`/`describe` note
        // for the mirror case on the onset side.
        let captain = sounds("K AE1 P T AH0 N");
        let kapt = parse_outline("KAPT").unwrap();
        assert!(rules.matches_real_consonants(&kapt, "captain", &captain));
        assert_eq!(rules.drop_fragments(&kapt, "captain", &captain), None);
        // `district`'s real first sound (a "d") reduces to nothing, so its one-stroke brief's
        // left-hand keys are entirely the *second* syllable's own "tr" cluster - correctly whole,
        // but sharing a key with "d"'s own onset cluster (`TK`).
        let district = sounds("D IH1 S T R IH0 K T");
        let tr_t = parse_outline("TR-T").unwrap();
        assert!(rules.matches_real_consonants(&tr_t, "district", &district));
        assert_eq!(rules.drop_fragments(&tr_t, "district", &district), None);
    }

    #[test]
    fn spelling_tells_words_that_sound_the_same_apart() {
        let rules = Rules::built_in().unwrap();
        let sound = sounds("S EY1 L");
        // The same outline either way; only the silent-e word moves aside when they clash
        assert_eq!(write("sail", "S EY1 L"), write("sale", "S EY1 L"));
        assert_eq!(rules.variant("sail", &sound), None);
        let sale = rules.variant("sale", &sound).unwrap();
        assert_eq!(render_outline(&sale), "SAEL");
    }

    #[test]
    fn a_long_word_can_be_one_skeleton_stroke() {
        let rules = Rules::built_in().unwrap();
        let forms = |word: &str, text: &str| -> Vec<String> {
            rules
                .skeletons(word, &sounds(text))
                .into_iter()
                .map(|(step, vowel, stroke)| {
                    format!("{step}{} {stroke}", if vowel { "+v" } else { "" })
                })
                .collect()
        };
        // Every consonant after the first vowel in steno order, bare and with the stressed vowel
        let window = forms("window", "W IH1 N D OW0");
        assert_eq!(window[..2], ["skeleton W-PBD", "skeleton+v WEUPBD"]);
        assert!(window.contains(&"ends W-D".to_owned()), "{window:?}");
        // The L of `-ly` would land before the T, so only the last syllable's form is in order
        assert_eq!(
            forms("lately", "L EY1 T L IY0"),
            ["ends HR-L", "ends+v HRAEUL"]
        );
        // Out of order (the D of `-dish` would land after the SH), neither the full skeleton
        // nor the stressed-and-last form is offered; the last syllable's form is Plover's brief
        assert_eq!(
            forms("condition", "K AH0 N D IH1 SH AH0 N"),
            ["ends K-PB", "ends+v KEUPB"]
        );
        // A dropped `a-` leaves the next onset as the left hand; a form never lacks one
        let america = forms("america", "AH0 M EH1 R IH0 K AH0");
        assert!(america.iter().all(|f| f.contains(" PH")), "{america:?}");
        // An unstressed `in-` is the left hand, with the rest on the right
        let information = forms("information", "IH2 N F ER0 M EY1 SH AH0 N");
        assert!(
            information.iter().all(|f| f.contains(" TPH")),
            "{information:?}"
        );
    }
}
