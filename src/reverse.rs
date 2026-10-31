//! One reverse-lookup entry point, forward or backward.
//!
//! Mirrors keymux's own `src/steno/reverse.rs` (the engine's live lookup and its CLI share) so
//! the two answer the same way: a query with no lowercase letter that parses as a stroke outline
//! resolves forward (what does this stroke type); anything else is text to find the stroke(s)
//! for, a single word or a whole phrase, word by word, next to Plover's and Lapwing's. A word with
//! no pronunciation at all falls back to [`crate::rules::Rules::spell_word`]'s spelling-mode
//! outline rather than going unanswered - see `docs/spelling-mode.md`.

use crate::analysis::Setup;
use crate::stroke::{self, Stroke, render_outline};
use std::collections::HashMap;

/// What a query, as typed, means. See keymux's own `classify` for the identical rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Query {
    Stroke(Vec<Stroke>),
    Text(String),
}

#[must_use]
pub fn classify(input: &str) -> Query {
    let trimmed = input.trim();
    let looks_like_a_stroke =
        !trimmed.is_empty() && !trimmed.chars().any(|c| c.is_ascii_lowercase());
    if looks_like_a_stroke {
        if let Some(strokes) = stroke::parse_outline(trimmed) {
            return Query::Stroke(strokes);
        }
    }
    Query::Text(trimmed.to_string())
}

/// One word's reverse lookup: lyk's own best candidate next to Plover's and Lapwing's shortest,
/// any of the three `None` when that theory (or CMUdict) has nothing for the word.
#[derive(Debug, Clone)]
pub struct WordLookup {
    pub word: String,
    pub lyk: Option<String>,
    pub plover: Option<String>,
    pub lapwing: Option<String>,
}

/// A word's real, settled outline - the same answer `resolve`, `quote`, and the deployed
/// dictionary itself would give, not an independently re-derived approximation. Falls back to the
/// rules' own raw candidates for a word `dictionary` has nothing for at all (one ranked past
/// whatever `common` the dictionary was built with, or not a real English word - `ander` has no
/// dictionary entry of its own, since nobody writes "ander", but the rules can still read it back
/// once struck; see `Rules::resolve`'s own stem-recognition fallback for that case instead), then
/// to spelling mode for a word with no pronunciation on record for the rules to run from at all
/// (`stenography`; see `Rules::spell_word`).
#[must_use]
pub fn reverse_word(setup: &Setup, dictionary: &HashMap<&str, &[Stroke]>, word: &str) -> WordLookup {
    let lower = word.to_lowercase();
    let lyk = dictionary
        .get(lower.as_str())
        .map(|o| render_outline(o))
        .or_else(|| setup.candidates(&lower).into_iter().next())
        .or_else(|| setup.rules.spell_word(&lower).map(|o| render_outline(&o)));
    let (plover, lapwing) = setup.gold.shortest(&lower);
    WordLookup {
        word: word.to_string(),
        lyk,
        plover: plover.map(str::to_string),
        lapwing: lapwing.map(str::to_string),
    }
}

/// `phrase`'s reverse lookup, one entry per word, in the order the phrase gave them. `dictionary`
/// is built once by the caller (see [`reverse_word`]) and shared across every word in the phrase,
/// not rebuilt per word.
#[must_use]
pub fn reverse_phrase(setup: &Setup, dictionary: &HashMap<&str, &[Stroke]>, phrase: &str) -> Vec<WordLookup> {
    phrase.split_whitespace().map(|w| reverse_word(setup, dictionary, w)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stroke::parse_outline;

    #[test]
    fn an_all_caps_outline_classifies_as_a_stroke() {
        assert_eq!(classify("KAT/+-D"), Query::Stroke(parse_outline("KAT/+-D").unwrap()));
    }

    #[test]
    fn lowercase_text_classifies_as_text_even_if_it_would_parse() {
        assert_eq!(classify("kat"), Query::Text("kat".to_string()));
        assert_eq!(
            classify("a whole phrase"),
            Query::Text("a whole phrase".to_string())
        );
    }

    #[test]
    fn empty_input_is_text_not_a_stroke() {
        assert_eq!(classify(""), Query::Text(String::new()));
        assert_eq!(classify("   "), Query::Text(String::new()));
    }

    #[test]
    fn a_word_with_no_pronunciation_falls_back_to_spelling_mode() {
        let setup = Setup::load(std::path::Path::new("data"), None, Some(1_000)).unwrap();
        let found = reverse_word(&setup, &HashMap::new(), "stenography");
        assert_eq!(found.lyk.as_deref(), Some("~S/+T/+E/+TPH/+O/+TKPW/+R/+A/+P/+H/+KWH"));
    }
}
