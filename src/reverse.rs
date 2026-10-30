//! One reverse-lookup entry point, forward or backward.
//!
//! Mirrors keymux's own `src/steno/reverse.rs` (the engine's live lookup and its CLI share) so
//! the two answer the same way: a query with no lowercase letter that parses as a stroke outline
//! resolves forward (what does this stroke type); anything else is text to find the stroke(s)
//! for, a single word or a whole phrase, word by word, next to Plover's and Lapwing's. There is
//! no fingerspelling data in this repo (that lives in the deployed config's own "Emily's
//! modifiers" dictionary, not here), so a word with no pronunciation at all is simply unanswered
//! rather than spelled out, unlike keymux's own version.

use crate::analysis::Setup;
use crate::stroke::{self, Stroke};

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

#[must_use]
pub fn reverse_word(setup: &Setup, word: &str) -> WordLookup {
    let lower = word.to_lowercase();
    let lyk = setup.candidates(&lower).into_iter().next();
    let (plover, lapwing) = setup.gold.shortest(&lower);
    WordLookup {
        word: word.to_string(),
        lyk,
        plover: plover.map(str::to_string),
        lapwing: lapwing.map(str::to_string),
    }
}

/// `phrase`'s reverse lookup, one entry per word, in the order the phrase gave them.
#[must_use]
pub fn reverse_phrase(setup: &Setup, phrase: &str) -> Vec<WordLookup> {
    phrase.split_whitespace().map(|w| reverse_word(setup, w)).collect()
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
}
