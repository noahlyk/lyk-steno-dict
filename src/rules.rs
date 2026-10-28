//! The phonetic rules: from sounds to strokes, read from a TOML file.

use crate::stroke::Stroke;
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// The rules that ship with the program.
const BUILT_IN: &str = include_str!("../rules/plover.toml");

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
    /// Only the stressed syllable keeps its vowel. Every other syllable is its consonants, folded
    /// onto the stroke before when they fit, otherwise a vowelless stroke of its own.
    Squeeze,
    /// One stroke: the consonants that start the word, then every consonant after its first vowel
    /// folded onto the right hand.
    Skeleton,
    /// One stroke: the consonants that start the word and those of the stressed and last
    /// syllables.
    Shell,
    /// One stroke: the consonants that start the word and those of the last syllable.
    Ends,
}

impl std::fmt::Display for Step {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Squeeze => "squeeze",
            Self::Skeleton => "skeleton",
            Self::Shell => "shell",
            Self::Ends => "ends",
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
    /// Whether consonants joining the stroke before may land out of steno order
    fold_any: bool,
    join_key: bool,
    brevity: Vec<Shortening>,
    prefixes: Vec<(Prefix, Stroke)>,
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

/// A sound without its stress digit or spelling variant: `UW.oo1` is `UW`.
fn plain(sound: &str) -> &str {
    sound
        .trim_end_matches(|c: char| c.is_ascii_digit())
        .split('.')
        .next()
        .unwrap_or(sound)
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
        Ok(Self {
            onset: strokes(raw.onset, "onset")?,
            coda: strokes(raw.coda, "coda")?,
            vowel: strokes(raw.vowel, "vowel")?,
            vowel_coda: strokes(raw.vowel_coda, "vowel_coda")?,
            onset_cluster: strokes(raw.onset_cluster, "onset_cluster")?,
            coda_cluster: strokes(raw.coda_cluster, "coda_cluster")?,
            onsets: raw.syllables.onsets.into_iter().collect(),
            lax: raw.syllables.lax.into_iter().collect(),
            spelled: raw.spelled,
            reduce: raw.reduce.vowels.into_iter().collect(),
            keep: raw.reduce.except.into_iter().collect(),
            lone: raw.reduce.lone.into_iter().collect(),
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
                .map(|b| Shortening {
                    step: b.step,
                    rank: b.rank,
                    vowel: b.vowel,
                    min_syllables: b.min_syllables,
                })
                .collect(),
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
        let plain_vowel = self
            .vowel
            .get(&syllable.vowel)
            .or_else(|| self.vowel.get(base))
            .or_else(|| self.vowel.get(plain(base)))
            .copied();
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
                    let two = cluster.len() >= 2 && {
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

    /// The one-stroke forms a shortening step gives a word of several syllables.
    pub fn shorten(&self, step: Shortening, word: &str, sounds: &[String]) -> Vec<Stroke> {
        let syllables = sounds
            .iter()
            .filter(|s| s.ends_with(|c: char| c.is_ascii_digit()))
            .count();
        if syllables < step.min_syllables {
            return Vec::new();
        }
        self.skeletons(word, sounds)
            .into_iter()
            .filter(|(kind, vowel, _)| *kind == step.step && *vowel == step.vowel)
            .map(|(_, _, stroke)| stroke)
            .collect()
    }

    /// A word of several syllables as one stroke: the consonants that start it, then consonants
    /// after its first vowel folded onto the right hand, with the stressed vowel or without. Which
    /// consonants depends on the step: all of them, those of the stressed and last syllables, or
    /// those of the last syllable only.
    pub fn skeletons(&self, word: &str, sounds: &[String]) -> Vec<(Step, bool, Stroke)> {
        let labelled = self.label(word, sounds);
        let (prefix, skip) = self.prefix(word, &labelled).unzip();
        let Some(syllables) = self.syllables(&labelled[skip.unwrap_or(0)..]) else {
            return Vec::new();
        };
        if syllables.len() + usize::from(prefix.is_some()) < 2 {
            return Vec::new();
        }
        let Some(onset) =
            Self::consonants(&syllables[0].onset, &self.onset, &self.onset_cluster, false)
        else {
            return Vec::new();
        };
        let onset = Stroke(onset.0 | prefix.map_or(0, |p| p.0));
        // The consonants of a syllable, in order, with the `R` a reduced `ER` hides
        let consonants_of = |syllable: &Syllable, with_onset: bool| -> Vec<String> {
            let mut sounds: Vec<String> = Vec::new();
            if with_onset {
                sounds.extend(syllable.onset.iter().cloned());
            }
            if plain(&syllable.vowel) == "ER" {
                sounds.push("R".into());
            }
            sounds.extend(syllable.coda.iter().cloned());
            sounds
        };
        let last = syllables.len() - 1;
        let stress = syllables
            .iter()
            .position(|s| s.vowel.ends_with('1'))
            .unwrap_or(0);
        // From the most to the fewest consonants kept: all of them, then those of the stressed
        // and last syllables, then those of the last syllable only
        let mut choices: Vec<(Step, Vec<String>)> = Vec::new();
        let first_coda = consonants_of(&syllables[0], false);
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
        choices.push((Step::Ends, consonants_of(&syllables[last], true)));

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
            let Some(coda) = Self::consonants(&rest, &self.coda, &self.coda_cluster, false) else {
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
        let consonants = syllable.onset.len() + syllable.coda.len();
        if plain(&syllable.vowel) == "ER" || consonants == 0 {
            return self.syllable_stroke(syllable);
        }
        // A suffix is written on the right hand (`-ness` -PBS, `-ment` -PLT); when its consonants
        // don't fit there in order, the first go on the left (`-nial` TPH-L)
        let mut all = syllable.onset.clone();
        all.extend(syllable.coda.iter().cloned());
        if let Some(coda) = Self::consonants(&all, &self.coda, &self.coda_cluster, false) {
            return Some(coda);
        }
        let onset = Self::consonants(&syllable.onset, &self.onset, &self.onset_cluster, false)?;
        let coda = Self::consonants(&syllable.coda, &self.coda, &self.coda_cluster, false)?;
        Some(Stroke(onset.0 | coda.0))
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
        let labelled = self.label_with(word, sounds, clash);
        let (prefix, skip) = self.prefix(word, &labelled).unzip();
        let mut syllables = self.syllables(&labelled[skip.unwrap_or(0)..])?;
        let mut strokes: Vec<Stroke> = Vec::new();
        let mut previous: Option<Syllable> = None;
        for syllable in syllables.drain(..) {
            if let Some(before) = &previous
                && self.is_reduced(&syllable)
            {
                let mut merged = before.clone();
                merged.coda.extend(syllable.onset.iter().cloned());
                if plain(&syllable.vowel) == "ER" {
                    merged.coda.push("R".into());
                }
                merged.coda.extend(syllable.coda.iter().cloned());
                if let Some(stroke) = self.stroke_for(&merged, self.fold_any) {
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
                    && let Some(stroke) = self.stroke_for(&stem, self.fold_any)
                    && let Some(rest) = self.continuation(&suffix)
                {
                    *strokes.last_mut()? = stroke;
                    strokes.push(rest);
                    previous = Some(suffix);
                    continue;
                }
                strokes.push(self.continuation(&syllable)?);
                previous = Some(syllable);
                continue;
            }
            strokes.push(self.syllable_stroke(&syllable)?);
            previous = Some(syllable);
        }
        if let Some(prefix) = prefix {
            // The prefix keys join the first stroke's own left-hand keys, sharing any they both
            // use: `KP` + `PEBGT` is `KPEBGT`, `STK` + `TAPBS` is `STKAPBS`
            strokes.first_mut()?.0 |= prefix.0;
        }
        Some(strokes)
    }

    /// Mark the strokes after the first of a word with the join key, if the theory has one.
    pub fn joined(&self, outline: &mut [Stroke]) {
        if self.join_key {
            for stroke in outline.iter_mut().skip(1) {
                stroke.0 |= Stroke::JOIN;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stroke::render_outline;

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
        // A reduced `er` keeps its vowel keys
        assert_eq!(write("harder", "HH AA1 R D ER0"), "HARD/ER");
    }

    #[test]
    fn consonants_only_join_the_stroke_before_in_steno_order() {
        assert_eq!(write("goodness", "G UH1 D N AH0 S"), "TKPWAOD/-PBS");
        assert_eq!(write("lately", "L EY1 T L IY0"), "HRAEUT/-L");
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
        // The folded R would land before the T, so `-er` is a stroke of its own
        assert_eq!(write("better", "B EH1 T ER0"), "PWET/ER");
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
        // With no left hand there is no short form
        assert!(forms("america", "AH0 M EH1 R IH0 K AH0").is_empty());
    }
}
