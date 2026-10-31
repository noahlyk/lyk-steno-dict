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

/// Plan verification step 5 for the candidate-enumeration-and-solver rewrite (see
/// `docs/decisions.md`): the whole algorithm choice (successive shortest augmenting paths over the
/// *contested* subgraph only, split further into independent connected components, rather than a
/// dense assignment algorithm over the full dictionary) rests on fixed-point peeling settling most
/// of the dictionary on its own, leaving only a real minority still needing the solver's own
/// frequency-weighted arbitration - a fact this checks directly against the real, built dictionary
/// rather than trusting it by assumption.
///
/// The threshold is *not* the plan's own original estimate ("hundreds to low thousands", under 5%)
/// - a real run measured roughly a quarter of the dictionary genuinely contested, not under 5%,
/// because the blanket `matches_real_consonants` filter's deeper fragment-drops mean far more words
/// now legitimately converge on the same small handful of very short, nearly-bare outlines than a
/// single-source brief pool ever produced. That is a real, now-measured property of this design,
/// not a bug to chase back down - connected-component splitting is what actually keeps each solve
/// small in practice (see `generate`'s own doc), not a low raw contested-word count. This guard
/// exists for the failure mode that *is* a bug: the contested set swallowing nearly the whole
/// dictionary the way an earlier, incorrect peeling test once did (confirmed directly: that bug
/// measured 99.3% contested, not ~25%).
#[test]
fn the_contested_set_after_peeling_stays_a_minority_of_the_dictionary() {
    const CONTESTED_SHARE_MAX: f64 = 0.35;
    let generated = dictionary_fixture::generated();
    let lyk = dictionary_fixture::cheapest();
    let share = generated.contested as f64 / lyk.len() as f64;
    assert!(
        share <= CONTESTED_SHARE_MAX,
        "{:.1}% of the dictionary ({} of {} words) was still contested after fixed-point peeling, \
         want <= {:.0}% - peeling itself likely regressed back toward treating a word's full \
         candidate-list length as the test, rather than whether its own cheapest candidate is \
         uniquely its own (see `generate`'s own peeling doc)",
        share * 100.0,
        generated.contested,
        lyk.len(),
        CONTESTED_SHARE_MAX * 100.0
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
