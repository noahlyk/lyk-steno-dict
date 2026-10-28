//! What Plover and Lapwing write for each word: the answers the rules are scored against.

use crate::data::load_dictionary;
use crate::stroke::parse_outline;
use anyhow::Result;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

/// Every outline a dictionary has for each plain lowercase word, shortest first.
pub fn all_outlines(dict: &BTreeMap<String, String>) -> HashMap<String, Vec<String>> {
    let mut words: HashMap<String, Vec<String>> = HashMap::new();
    for (outline, word) in dict {
        let plain = !word.is_empty()
            && word.chars().all(|c| c.is_ascii_lowercase() || c == '\'')
            && word.chars().any(|c| c.is_ascii_lowercase());
        // Only outlines that are strokes (no numbers)
        if plain && parse_outline(outline).is_some() {
            words.entry(word.clone()).or_default().push(outline.clone());
        }
    }
    for outlines in words.values_mut() {
        outlines.sort_by_key(|o| (o.split('/').count(), o.len(), o.clone()));
    }
    words
}

pub struct Gold {
    pub plover: HashMap<String, Vec<String>>,
    pub lapwing: HashMap<String, Vec<String>>,
}

impl Gold {
    pub fn load(root: &Path) -> Result<Self> {
        let dicts = root.join("dictionaries");
        Ok(Self {
            plover: all_outlines(&load_dictionary(&dicts.join("plover-main.json"))?),
            lapwing: all_outlines(&load_dictionary(&dicts.join("lapwing-base.json"))?),
        })
    }

    /// The words either dictionary writes.
    pub fn words(&self) -> HashSet<&str> {
        self.plover
            .keys()
            .chain(self.lapwing.keys())
            .map(String::as_str)
            .collect()
    }

    /// Whether Plover and Lapwing write `word` as `outline`.
    pub fn writes(&self, word: &str, outline: &str) -> (bool, bool) {
        let has = |map: &HashMap<String, Vec<String>>| {
            map.get(word)
                .is_some_and(|all| all.iter().any(|o| o == outline))
        };
        (has(&self.plover), has(&self.lapwing))
    }

    pub fn shortest(&self, word: &str) -> (Option<&str>, Option<&str>) {
        (
            self.plover
                .get(word)
                .and_then(|v| v.first())
                .map(String::as_str),
            self.lapwing
                .get(word)
                .and_then(|v| v.first())
                .map(String::as_str),
        )
    }
}
