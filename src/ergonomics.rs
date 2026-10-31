//! Physical finger ergonomics, separate from key count. Two strokes can cost the same number of
//! keys and feel completely different to type - `TK` (one finger, both its own rows, used
//! everywhere for the "D" sound) is comfortable, while `STWH` (alternating top/bottom across four
//! different fingers) is not. Nothing before this module modeled that difference at all; `cheapest`
//! and the whole collision/brief system only ever counted keys.
//!
//! Physical layout (confirmed against the user's own keyboard diagram, `# +S TK PW HR AO - EU *
//! FR PB LG TS DZ`): every finger owns exactly one column with a top and bottom key, except the
//! right pinky, which owns two columns (`T`/`S`, the normal pair, and `D`/`Z`, a further stretch) -
//! a real, structural asymmetry in the layout itself, not a theory choice. The left pinky (`S`
//! only) is the opposite extreme: the least contested key on the board.
//!
//! The score has three parts, in descending weight:
//! - **Row transitions**: walking a hand's active fingers in physical order (pinky, ring, middle,
//!   index), a transition is a pair of neighbors that share no common row. `SKPH` (bottom, bottom,
//!   top, top) is one transition - a clean staircase. `STWH` (bottom, top, bottom, top) is three -
//!   alternating every finger, the worst case for a given set of active fingers. A finger pressing
//!   *both* its own rows is compatible with either neighbor, so it never forces a transition on its
//!   own - same-finger double presses (`TK` for "D", `PW` for "B") are foundational, universal steno chords,
//!   not a penalty by themselves.
//! - **Gaps**: a finger skipped between two active neighbors on the same hand (`SWR` skips the
//!   ring between pinky and middle) - fewer keys than a full run, but breaks what would otherwise
//!   be a clean multi-finger roll into an isolated-finger-lift motion.
//! - **Pinky usage**: a small flat tax per press of `S` (left) or `T`/`S`/`D`/`Z` (right) -
//!   deliberately *excluding* the join key, which is bound to its own separate physical key and
//!   appears in essentially every multi-stroke continuation regardless of which words are
//!   involved, so taxing it would penalize "more than one stroke," not any real word choice.
//!
//! This module is read-only and diagnostic today - it does not yet change which outline any word
//! gets. See `docs/decisions.md` for the real numbers this was calibrated against and the plan for
//! using it.

use crate::stroke::Stroke;

// Bit indices, matching stroke.rs's KEYS = "#STKPWHRAO*EUFRPBLGTSDZ"
const S: u32 = 1 << 1;
const T: u32 = 1 << 2;
const K: u32 = 1 << 3;
const P: u32 = 1 << 4;
const W: u32 = 1 << 5;
const H: u32 = 1 << 6;
const R: u32 = 1 << 7;
const F: u32 = 1 << 13;
const R2: u32 = 1 << 14;
const P2: u32 = 1 << 15;
const B: u32 = 1 << 16;
const L: u32 = 1 << 17;
const G: u32 = 1 << 18;
const T2: u32 = 1 << 19;
const S2: u32 = 1 << 20;
const D: u32 = 1 << 21;
const Z: u32 = 1 << 22;

const BOTTOM: u8 = 1;
const TOP: u8 = 2;
const BOTH: u8 = 3;

/// Per-finger coupling cost, x10 to stay in clean integers: `index` is the baseline (1.0x),
/// `little`/`middle` ~1.3x, `ring` ~1.8x - a linear mapping from Häger-Ross & Schieber's measured
/// individuation indices (~0.978/~0.944/~0.936/~0.903), kept deliberately under 2x per the
/// research's own caution against overfitting an ~8%-spread proxy metric into a dramatic
/// multiplier. Used for both row-transition and gap costs - see the module doc.
mod coupling {
    pub const INDEX: u32 = 10;
    pub const MIDDLE: u32 = 13;
    pub const LITTLE: u32 = 13;
    pub const RING: u32 = 18;
}

/// Physical finger order, pinky to index, with each slot's coupling weight alongside it.
const LEFT_COUPLING: [u32; 4] = [coupling::LITTLE, coupling::RING, coupling::MIDDLE, coupling::INDEX];
/// Physical finger order, index to pinky (mirrored from the left hand).
const RIGHT_COUPLING: [u32; 4] = [coupling::INDEX, coupling::MIDDLE, coupling::RING, coupling::LITTLE];

/// A finger's row state from the bits it owns: `None` if unpressed, else which row(s) are down.
fn row(bits: u32, top_bit: u32, bottom_bit: u32) -> Option<u8> {
    let state = (u8::from(bits & top_bit != 0) * TOP) | (u8::from(bits & bottom_bit != 0) * BOTTOM);
    (state != 0).then_some(state)
}

/// A single stroke's ergonomic components. See the module doc for what each one means.
pub struct Score {
    /// Raw count of row-mismatched adjacent finger pairs (unweighted - see `transition_cost` for
    /// the finger-identity-weighted version `weighted()` actually uses). Kept for tests/diagnostics
    /// that care about the raw shape rather than its cost; not read by `weighted()` itself.
    #[allow(dead_code)]
    pub transitions: u32,
    /// Raw count of skipped-finger gaps (unweighted - see `gap_cost`). Same caveat as `transitions`.
    #[allow(dead_code)]
    pub gaps: u32,
    pub double_presses: u32,
    /// Pinky key presses, excluding the join bit - see the module doc.
    pub pinky_keys: u32,
    /// Right-pinky stretch-key (`D`/`Z`) presses specifically - a structural overload tax separate
    /// from the general pinky leverage tax.
    pub stretch_keys: u32,
    /// Sum of `COUPLING` weights across every row-mismatched adjacent finger pair.
    pub transition_cost: u32,
    /// Sum of `COUPLING` weights across every skipped-finger gap.
    pub gap_cost: u32,
}

/// Scores one stroke's physical comfort. Independent of word, frequency, or key count - just the
/// shape of the keys actually held down.
#[must_use]
pub fn score(stroke: Stroke) -> Score {
    let bits = stroke.0 & !(Stroke::JOIN | Stroke::SPELL | Stroke::STAR);

    // S has no top counterpart - fixed at "bottom" so it still participates in the row walk.
    let left: [Option<u8>; 4] = [
        (bits & S != 0).then_some(BOTTOM),
        row(bits, T, K),
        row(bits, P, W),
        row(bits, H, R),
    ];
    let right: [Option<u8>; 4] = [
        row(bits, F, R2),
        row(bits, P2, B),
        row(bits, L, G),
        // Right pinky: T/D read as "top" (D further out), S2/Z read as "bottom" (Z further out).
        row(bits, T2 | D, S2 | Z),
    ];

    let walk = |fingers: &[Option<u8>; 4], coupling: &[u32; 4]| -> (u32, u32, u32, u32, u32) {
        // (original slot index, row state), active fingers only - transitions are checked between
        // *consecutively active* fingers, same as the dictionary-scale behavior before this
        // revision: an inactive finger between two active ones is a gap, not a transition skip.
        let active: Vec<(usize, u8)> =
            fingers.iter().enumerate().filter_map(|(i, &f)| f.map(|row| (i, row))).collect();
        let mut transitions = 0;
        let mut transition_cost = 0;
        let mut doubles = 0;
        for &(_, row) in &active {
            if row == BOTH {
                doubles += 1;
            }
        }
        for pair in active.windows(2) {
            let (&(i, row), &(j, next_row)) = (&pair[0], &pair[1]);
            if row & next_row == 0 {
                transitions += 1;
                transition_cost += coupling[i] + coupling[j];
            }
        }
        let mut gaps = 0;
        let mut gap_cost = 0;
        for i in 1..fingers.len() - 1 {
            if fingers[i].is_none() && fingers[i - 1].is_some() && fingers[i + 1].is_some() {
                gaps += 1;
                gap_cost += coupling[i];
            }
        }
        (transitions, transition_cost, doubles, gaps, gap_cost)
    };

    let (lt, ltc, ld, lg, lgc) = walk(&left, &LEFT_COUPLING);
    let (rt, rtc, rd, rg, rgc) = walk(&right, &RIGHT_COUPLING);
    let pinky_keys = u32::from(bits & S != 0) + (bits & (T2 | S2 | D | Z)).count_ones();
    let stretch_keys = (bits & (D | Z)).count_ones();

    Score {
        transitions: lt + rt,
        gaps: lg + rg,
        double_presses: ld + rd,
        pinky_keys,
        stretch_keys,
        transition_cost: ltc + rtc,
        gap_cost: lgc + rgc,
    }
}

/// A single weighted number, low = comfortable. The coupling weights (`transition_cost`,
/// `gap_cost`) carry the literature-grounded ordering; `pinky_keys`/`stretch_keys` are a separate,
/// smaller leverage/overload tax. See the module doc for the research behind each term and
/// `docs/decisions.md` for the real distribution these produce.
#[must_use]
pub fn weighted(stroke: Stroke) -> u32 {
    let s = score(stroke);
    s.transition_cost + s.gap_cost + s.double_presses + s.pinky_keys + s.stretch_keys * 2
}

/// A whole outline's ergonomic cost: the sum of each of its strokes' own.
#[must_use]
pub fn weighted_outline(outline: &[Stroke]) -> u32 {
    outline.iter().map(|&s| weighted(s)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary_fixture;
    use crate::stroke::parse_outline;

    struct Band {
        name: &'static str,
        max_rank: usize,
        avg_ceiling: f64,
        bad_pct_ceiling: f64,
    }

    #[test]
    fn a_lone_same_finger_double_press_is_not_penalized_as_a_transition() {
        // TK ("D") and PW ("B") are foundational, universal steno chords, each one finger's own
        // two rows - a double press with nothing else active costs nothing but the press itself.
        for text in ["TK", "PW"] {
            let stroke = parse_outline(text).unwrap()[0];
            let s = score(stroke);
            assert_eq!(s.transitions, 0, "{text} should have no transitions");
            assert_eq!(s.double_presses, 1, "{text} should register its own double press");
        }
    }

    #[test]
    fn alternating_rows_cost_more_than_a_staircase() {
        // SKPH: bottom, bottom, top, top - one staircase step.
        let skph = parse_outline("SKPH").unwrap()[0];
        // STWH: bottom, top, bottom, top - alternates every finger.
        let stwh = parse_outline("STWH").unwrap()[0];
        assert_eq!(score(skph).transitions, 1);
        assert_eq!(score(stwh).transitions, 3);
        assert!(weighted(stwh) > weighted(skph));
    }

    #[test]
    #[allow(clippy::similar_names)] // swr/skwr deliberately echo the SWR/SKWR outlines compared
    fn a_skipped_finger_between_two_active_neighbors_is_a_gap() {
        // SWR skips the ring (K) between the pinky (S) and middle (W).
        let swr = parse_outline("SWR").unwrap()[0];
        assert_eq!(score(swr).gaps, 1);
        // SKWR has no gap - every finger from pinky to index is active.
        let skwr = parse_outline("SKWR").unwrap()[0];
        assert_eq!(score(skwr).gaps, 0);
    }

    #[test]
    fn the_join_key_alone_never_costs_a_pinky_tax() {
        let joined = Stroke(Stroke::JOIN | (1 << 18)); // +-G, a plain continuation
        assert_eq!(score(joined).pinky_keys, 0);
    }

    /// A score at or above this is "bad" for the `bad_pct_ceiling` counts below - rescaled from the
    /// pre-kinematics-revision model's cutoff of 8 (itself ~3.9x that model's own top-1,000 average
    /// of ~2.05) to stay proportionate under the new, larger coupling-weighted scale (new top-1,000
    /// average ~9.99, so ~4x that).
    const BAD_SCORE: u32 = 40;

    /// Every ceiling here is set a little above a real scan of the dictionary, post-kinematics
    /// revision (see `docs/decisions.md`'s ergonomics entries for the exact numbers and the research
    /// behind the new scale) - loose enough that adding a handful of new words doesn't break it,
    /// tight enough that a real regression (a rule change that systematically makes a whole class of
    /// words less comfortable) still fails it.
    #[test]
    fn the_real_dictionary_stays_within_known_ergonomic_ceilings() {
        let setup = dictionary_fixture::setup();
        let lyk = dictionary_fixture::cheapest();

        let bands = [
            Band { name: "top 1,000", max_rank: 999, avg_ceiling: 11.0, bad_pct_ceiling: 35.0 },
            Band { name: "1,000-10,000", max_rank: 9_999, avg_ceiling: 17.0, bad_pct_ceiling: 48.0 },
            Band { name: "10,000+", max_rank: usize::MAX, avg_ceiling: 19.0, bad_pct_ceiling: 49.0 },
        ];

        for band in &bands {
            let min_rank = if band.name == "1,000-10,000" { 1_000 } else if band.name == "10,000+" { 10_000 } else { 0 };
            let scores: Vec<u32> = lyk
                .iter()
                .filter(|(word, _)| {
                    let rank = setup.rank.get(**word).copied().unwrap_or(usize::MAX);
                    rank >= min_rank && rank <= band.max_rank
                })
                .map(|(_, outline)| weighted_outline(outline))
                .collect();
            if scores.is_empty() {
                continue;
            }
            let n = scores.len();
            let avg = scores.iter().map(|&s| f64::from(s)).sum::<f64>() / n as f64;
            let bad = scores.iter().filter(|&&s| s >= BAD_SCORE).count();
            let bad_pct = f64::from(u32::try_from(bad).unwrap()) * 100.0 / n as f64;
            assert!(
                avg <= band.avg_ceiling,
                "{}: average ergonomic score {avg:.2} exceeds ceiling {}",
                band.name,
                band.avg_ceiling
            );
            assert!(
                bad_pct <= band.bad_pct_ceiling,
                "{}: {bad_pct:.2}% of words score >= {BAD_SCORE}, exceeds ceiling {}%",
                band.name,
                band.bad_pct_ceiling
            );
        }
    }
}
