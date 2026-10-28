//! The phonetic rules: from sounds to strokes, read from a TOML file.

use crate::stroke::Stroke;
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// The rules that ship with the program.
const BUILT_IN: &str = include_str!("../rules/plover.toml");

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
}

#[derive(Deserialize, Default)]
struct RawReduce {
    /// Unstressed vowels that are not written: the syllable's consonants join the stroke before
    #[serde(default)]
    vowels: Vec<String>,
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
    /// The word has at least this many syllables
    pub min_syllables: Option<usize>,
}

impl Spelled {
    fn applies(&self, word: &str, at: usize, count: usize, syllables: usize) -> bool {
        self.min_syllables.is_none_or(|n| syllables >= n)
            && self
                .starts
                .as_ref()
                .is_none_or(|s| at == 0 && word.starts_with(s.as_str()))
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
    join_key: bool,
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
            join_key: raw.theory.join_key,
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
                    let key = sounds[at..at + n].join(" ");
                    clusters.get(&key).map(|&s| (s, n))
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
        let mut coda: &[String] = &syllable.coda;
        let mut vowel = None;
        if let Some(first) = coda.first() {
            let joined = self.vowel_coda.get(&format!("{base} {first}")).or_else(|| {
                self.vowel_coda
                    .get(&format!("{} {}", plain(base), plain(first)))
            });
            if let Some(&joined) = joined {
                vowel = Some(joined);
                coda = &coda[1..];
            }
        }
        let vowel = vowel.or_else(|| {
            self.vowel
                .get(&syllable.vowel)
                .or_else(|| self.vowel.get(base))
                .or_else(|| self.vowel.get(plain(base)))
                .copied()
        })?;
        let coda = Self::consonants(coda, &self.coda, &self.coda_cluster, fold)?;
        // The consonants after a joined vowel and consonant have to come after it
        if let (Some(a), Some(b)) = (Stroke(vowel.0 & Stroke::RIGHT).highest(), coda.lowest())
            && b <= a
        {
            return None;
        }
        if vowel.0 & coda.0 & Stroke::STAR != 0 {
            return None;
        }
        Some(Stroke(onset.0 | vowel.0 | coda.0))
    }

    /// Give the sounds of a word their spelling variants: how the word is spelled can decide
    /// which keys a sound goes on.
    pub fn label(&self, word: &str, sounds: &[String]) -> Vec<String> {
        let syllables = sounds
            .iter()
            .filter(|s| s.ends_with(|c: char| c.is_ascii_digit()))
            .count();
        sounds
            .iter()
            .enumerate()
            .map(|(at, sound)| {
                let stress: String = sound.chars().filter(char::is_ascii_digit).collect();
                let rule = self.spelled.iter().find(|r| {
                    r.sound == plain(sound) && r.applies(word, at, sounds.len(), syllables)
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
                let take = self.onset_length(&syllable.vowel, cluster);
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
        let stressed = vowel.ends_with(['1', '2']);
        if cluster.len() == 1 && stressed && self.lax.contains(plain(vowel)) {
            return 0;
        }
        (1..=cluster.len())
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

    /// Whether a syllable is one whose vowel is not written.
    fn is_reduced(&self, syllable: &Syllable) -> bool {
        syllable.vowel.ends_with('0') && self.reduce.contains(plain(&syllable.vowel))
    }

    /// The strokes for a word. A reduced syllable's consonants join the stroke before it when
    /// they fit; every other syllable is a stroke of its own. `None` if some syllable can't be
    /// written in a stroke by these rules.
    pub fn outline(&self, word: &str, sounds: &[String]) -> Option<Vec<Stroke>> {
        let mut syllables = self.syllables(&self.label(word, sounds))?;
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
                if let Some(stroke) = self.stroke_for(&merged, true) {
                    *strokes.last_mut()? = stroke;
                    previous = Some(merged);
                    continue;
                }
            }
            strokes.push(self.syllable_stroke(&syllable)?);
            previous = Some(syllable);
        }
        if self.join_key {
            for stroke in strokes.iter_mut().skip(1) {
                stroke.0 |= Stroke::JOIN;
            }
        }
        Some(strokes)
    }
}
