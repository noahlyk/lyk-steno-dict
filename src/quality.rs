//! Structural sanity checks on the real, deployed dictionary - not "is this word terse" (that's
//! `quote_corpus`'s job), but "does the dictionary even make sense": no two words fighting over
//! the same stroke, no word missing that obviously should not be, and the known cost of design
//! decisions already made (see `docs/decisions.md`) staying where it was measured, not quietly
//! growing. Each test here is a regression guard for a specific, previously-real bug this session
//! found the hard way, so a future change that reintroduces one of them fails immediately instead
//! of waiting for someone to notice live, the way "don't" and "quitters" both were noticed.

use crate::dictionary_fixture;
use crate::generate;
use crate::stroke::render_outline;
use std::collections::HashMap;

/// No two different words were ever meant to share a stroke - `generate()`'s own `taken` map
/// exists specifically to prevent it. This re-checks the invariant from the outside, on the real
/// built dictionary, rather than trusting that nothing inside `generate()` ever bypasses it.
#[test]
fn no_two_words_share_the_same_deployed_outline() {
    let lyk = dictionary_fixture::cheapest();
    let mut by_outline: HashMap<String, &str> = HashMap::new();
    let mut collisions = Vec::new();
    for (&word, outline) in &lyk {
        let text = render_outline(outline);
        if let Some(&other) = by_outline.get(&text)
            && other != word
        {
            collisions.push(format!("{text:?}: {other:?} and {word:?}"));
        }
        by_outline.insert(text, word);
    }
    collisions.sort();
    assert!(collisions.is_empty(), "words sharing a stroke: {collisions:#?}");
}

/// A word common enough to be in everyday use, with a real pronunciation on record, should never
/// be silently missing from the generated dictionary - that was exactly the "don't" bug (see
/// `docs/decisions.md`): a real contraction with a real `CMUdict` entry, absent from the deployed
/// dictionary entirely because of a corpus mismatch nothing surfaced until someone hit it live.
/// This would have caught it automatically. Mirrors `generate()`'s own word-list filter
/// (`src/generate.rs`'s `words` construction) so it only asks for a word the generator itself
/// should have tried to place, not one it deliberately skips (a stray single-letter frequency
/// artifact, for instance).
#[test]
fn every_sufficiently_common_pronounceable_word_has_a_real_outline() {
    const TOP_N: usize = 2_000;
    let setup = dictionary_fixture::setup();
    let lyk = dictionary_fixture::cheapest();
    let mut missing: Vec<&str> = setup
        .rank
        .iter()
        .filter(|&(w, &r)| {
            r < TOP_N
                && setup.pron.contains_key(w.as_str())
                && (w.len() >= 2 || w.as_str() == "a" || w.as_str() == "i")
                && w.bytes().all(|b| b.is_ascii_lowercase() || b == b'\'')
        })
        .map(|(w, _)| w.as_str())
        .filter(|w| !lyk.contains_key(*w))
        .collect();
    missing.sort_unstable();
    assert!(missing.is_empty(), "common words with no outline at all: {missing:?}");
}

/// The count of words settled on a worse outline than their own, unclaimed plain reading (`build`
/// prints this list; see `docs/decisions.md`'s stranded-word entry) was fixed down to zero, one
/// `[word]` override at a time by hand - not something that should ever silently grow back. If
/// this fails, something in collision resolution regressed. A future regression that can't be
/// fixed immediately gets its own named, reasoned exception here (a nonzero baseline *with* a
/// comment pointing at an open issue) - not a quiet bump of a bare constant.
#[test]
fn stranded_word_count_has_not_grown_past_its_known_baseline() {
    let stranded = &dictionary_fixture::generated().stranded;
    assert!(
        stranded.is_empty(),
        "stranded-word count grew from 0 to {}: {:?}",
        stranded.len(),
        stranded
    );
}

/// `generate::reserved_outlines` is seeded into `taken` before a single word is placed
/// specifically so this can never happen; this confirms it from the outside, against the real
/// built dictionary, rather than trusting the seeding order inside `generate()` never regresses.
#[test]
fn no_generated_outline_collides_with_a_reserved_stroke() {
    let reserved = generate::reserved_outlines().expect("reserved dictionaries parse");
    let lyk = dictionary_fixture::cheapest();
    let mut collisions = Vec::new();
    for (&word, outline) in &lyk {
        let text = render_outline(outline);
        if let Some(kind) = reserved.get(&text) {
            collisions.push(format!("{word:?} landed on {text:?}, {kind}"));
        }
    }
    collisions.sort();
    assert!(collisions.is_empty(), "words colliding with a reserved stroke: {collisions:#?}");
}
