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
        // When two labels share the same vowel keys (`AE` and the spelling variant `AA.a` are
        // both just `A`), the plain sound wins, deterministically, over a spelling variant
        let mut vowel_pairs: Vec<(Stroke, String)> =
            vowel.iter().map(|(s, &k)| (k, s.clone())).collect();
        vowel_pairs.sort_by(|a, b| {
            let plain_first = |s: &str| !s.contains('.');
            (plain_first(&a.1), &a.1).cmp(&(plain_first(&b.1), &b.1))
        });
        let vowel_rev: HashMap<Stroke, String> = vowel_pairs.into_iter().collect();
        let mut vowel_coda_rev: Vec<(Stroke, String, String)> = vowel_coda
            .iter()
            .filter_map(|(sounds, &stroke)| {
                let (vowel, coda) = sounds.split_once(' ')?;
                Some((stroke, vowel.to_owned(), coda.to_owned()))
            })
            .collect();
        vowel_coda_rev.sort_by(|a, b| (&a.1, &a.2).cmp(&(&b.1, &b.2)));

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
    /// starts there, covers the most keys, and (tied on that) is the fewest sounds, consumed,
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
                .max_by_key(|(s, tokens)| (s.0.count_ones(), std::cmp::Reverse(tokens.len())))?;
            out.extend(best.1.iter().cloned());
            bits &= !best.0.0;
        }
        Some(out)
    }

    /// The best reading the rules can give a stroke outline: each stroke's sounds, read back
    /// and spelled plainly (not real orthography, just enough to read), continuation strokes
    /// run on and others started with a space. `None` only once some stroke in it cannot be
    /// read at all.
    pub fn resolve(&self, outline: &[Stroke]) -> Option<String> {
        let mut text = String::new();
        for &stroke in outline {
            let resolved = self.resolve_stroke(stroke)?;
            if !resolved.join && !text.is_empty() {
                text.push(' ');
            }
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
        }
        Some(text)
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
        let read = |text: &str| rules.resolve(&parse_outline(text).unwrap());
        // A plain stroke that spells no word the rules know, read back the same way the rules
        // would have written it: onset, vowel, coda
        assert_eq!(read("STKPW"), Some("z".to_owned()));
        // Two labels can share the exact same keys (a plain sound and a spelling variant, or a
        // sound and its silent-glide cluster); the read-back always picks the same one
        for _ in 0..5 {
            assert_eq!(read("A/^A"), Some("aa".to_owned()));
        }
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
