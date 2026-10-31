//! A harsh, hardcoded-text regression benchmark: a hundred short passages nobody picked to
//! flatter lyk, scored exactly the way the live `quote` command scores one (`quote::rows_for`,
//! `quote::cost`), aggregated into per-word and per-passage win/loss counts against Plover and
//! Lapwing. Run this after any rule change - a drop in these numbers against the thresholds below
//! is a real regression in how terse the theory actually is, not noise, since the corpus itself
//! never changes.
//!
//! The passages are a mix of public-domain lines (old enough that attribution doesn't matter for
//! a test fixture) and plain, unremarkable English sentences written for this file - ordinary
//! prose nobody picked to make lyk look good, which is the entire point: a benchmark curated to
//! win would be measuring the curation, not the theory.

use crate::dictionary_fixture;
use crate::quote::{cost, rows_for};

const PASSAGES: &[&str] = &[
    "Four score and seven years ago our fathers brought forth on this continent a new nation.",
    "A house divided against itself cannot stand.",
    "Government of the people, by the people, for the people, shall not perish from the earth.",
    "To be or not to be, that is the question.",
    "All that glitters is not gold.",
    "Something is rotten in the state of Denmark.",
    "The lady doth protest too much, methinks.",
    "We hold these truths to be self evident, that all men are created equal.",
    "Give me liberty or give me death.",
    "I only regret that I have but one life to lose for my country.",
    "Early to bed and early to rise makes a man healthy, wealthy, and wise.",
    "A penny saved is a penny earned.",
    "Energy and persistence conquer all things.",
    "If you would not be forgotten as soon as you are dead, either write something worth reading or do something worth writing.",
    "I went to the woods because I wished to live deliberately.",
    "Our life is frittered away by detail, simplify, simplify.",
    "Not until we are lost do we begin to understand ourselves.",
    "Trust thyself, every heart vibrates to that iron string.",
    "A foolish consistency is the hobgoblin of little minds.",
    "Nothing great was ever achieved without enthusiasm.",
    "To be yourself in a world that is constantly trying to make you something else is a great accomplishment.",
    "It is not length of life, but depth of life that matters.",
    "The only true wisdom is in knowing you know nothing.",
    "An unexamined life is not worth living.",
    "Know thyself.",
    "He who has a why to live can bear almost any how.",
    "That which does not kill us makes us stronger.",
    "Without music, life would be a mistake.",
    "Patience is bitter, but its fruit is sweet.",
    "We are what we repeatedly do, excellence then is not an act but a habit.",
    "The whole is greater than the sum of its parts.",
    "It is during our darkest moments that we must focus to see the light.",
    "You have power over your mind, not outside events, realize this and you will find strength.",
    "Waste no more time arguing about what a good man should be, be one.",
    "The impediment to action advances action, what stands in the way becomes the way.",
    "It is not death that a man should fear, but he should fear never beginning to live.",
    "When I despair, I remember that all through history the way of truth and love has always won.",
    "Live as if you were to die tomorrow, learn as if you were to live forever.",
    "The weak can never forgive, forgiveness is the attribute of the strong.",
    "It does not matter how slowly you go as long as you do not stop.",
    "The journey of a thousand miles begins with a single step.",
    "Life is really simple, but we insist on making it complicated.",
    "Real knowledge is to know the extent of one's ignorance.",
    "Our greatest glory is not in never falling, but in rising every time we fall.",
    "The mind is everything, what you think you become.",
    "Three things cannot be long hidden, the sun, the moon, and the truth.",
    "Peace comes from within, do not seek it without.",
    "Happiness is not something ready made, it comes from your own actions.",
    "A room without books is like a body without a soul.",
    "Fall seven times and stand up eight.",
    "A smooth sea never made a skilled sailor.",
    "A ship in harbor is safe, but that is not what ships are built for.",
    "When one door of happiness closes, another opens.",
    "The best and most beautiful things in the world cannot be seen or touched.",
    "Alone we can do so little, together we can do so much.",
    "Life is either a daring adventure or nothing at all.",
    "If you tell the truth, you don't have to remember anything.",
    "The secret of getting ahead is getting started.",
    "Kindness is a language which the deaf can hear and the blind can see.",
    "Do what you can, with what you have, where you are.",
    "Nothing in life is to be feared, it is only to be understood.",
    "I am among those who think that science has great beauty.",
    "In three words I can sum up everything I have learned about life, it goes on.",
    "Two roads diverged in a wood, and I took the one less traveled by.",
    "The woods are lovely, dark and deep, but I have promises to keep.",
    "Do not go gentle into that good night.",
    "Hope is the thing with feathers that perches in the soul.",
    "It always seems impossible until it is done.",
    "Education is the most powerful weapon which you can use to change the world.",
    "The greatest glory in living lies not in never falling, but in rising every time we fall.",
    "Genius is one percent inspiration and ninety nine percent perspiration.",
    "I have not failed, I have just found ten thousand ways that will not work.",
    "The best way to predict the future is to create it.",
    "If you want to go fast, go alone, if you want to go far, go together.",
    "Not everything that is faced can be changed, but nothing can be changed until it is faced.",
    "Whether you think you can, or you think you can't, you're right.",
    "The future belongs to those who believe in the beauty of their dreams.",
    "Imagination is more important than knowledge.",
    "In the middle of difficulty lies opportunity.",
    "Float like a butterfly, sting like a bee.",
    "Not all those who wander are lost.",
    "There is some good in this world, and it's worth fighting for.",
    "Even the smallest person can change the course of the future.",
    "All we have to decide is what to do with the time that is given us.",
    "I don't know where we are going, but we can't be late.",
    "She wasn't sure if it would rain, so she didn't bring an umbrella.",
    "He couldn't find his keys, and he hadn't checked the kitchen table yet.",
    "They're planning to repaint the fence before it gets any colder outside.",
    "We've been waiting for almost an hour and the bus still hasn't arrived.",
    "I'm not certain what time the meeting starts, but it's probably after lunch.",
    "You shouldn't leave the window open when it's raining this hard.",
    "It's getting late, and we haven't finished packing for the trip yet.",
    "The children were laughing so loudly that the neighbors could hear them from next door.",
    "A quiet morning walk along the river can clear your mind before a busy day.",
    "The recipe calls for two cups of flour, a pinch of salt, and three eggs.",
    "After the storm passed, the whole garden smelled like wet grass and flowers.",
    "She spent the afternoon reorganizing her bookshelf by color instead of by author.",
    "The old clock in the hallway hasn't kept accurate time in nearly a decade.",
    "He finally finished the crossword puzzle after three cups of coffee and a long nap.",
    "The train was delayed again, so everyone on the platform just sat down and waited.",
    "Learning to type without looking at the keyboard takes a surprising amount of practice.",
    "A good night's sleep often matters more than any other single habit.",
    "The library closes early on weekends, which nobody seems to remember until they arrive.",
];

/// Running totals and win/loss counts for a passage corpus, next to Plover and Lapwing.
#[derive(Default)]
struct Stats {
    passages: u32,
    passage_wins_plover: u32,
    passage_losses_plover: u32,
    passage_wins_lapwing: u32,
    passage_losses_lapwing: u32,
    words: u32,
    word_fewer_keys_plover: u32,
    word_more_keys_plover: u32,
    word_fewer_keys_lapwing: u32,
    word_more_keys_lapwing: u32,
    word_fewer_strokes_plover: u32,
    word_fewer_strokes_lapwing: u32,
    total_keys_lyk: u64,
    total_keys_plover: u64,
    total_keys_lapwing: u64,
}

impl Stats {
    fn pct(num: u32, den: u32) -> f64 {
        if den == 0 { 0.0 } else { f64::from(num) * 100.0 / f64::from(den) }
    }

    fn report(&self) {
        println!("passages: {}", self.passages);
        println!(
            "  vs plover:  wins {} ({:.1}%)  losses {}",
            self.passage_wins_plover,
            Self::pct(self.passage_wins_plover, self.passages),
            self.passage_losses_plover
        );
        println!(
            "  vs lapwing: wins {} ({:.1}%)  losses {}",
            self.passage_wins_lapwing,
            Self::pct(self.passage_wins_lapwing, self.passages),
            self.passage_losses_lapwing
        );
        println!("words: {}", self.words);
        println!(
            "  fewer keys    vs plover:  {} ({:.1}%)   more keys: {} ({:.1}%)",
            self.word_fewer_keys_plover,
            Self::pct(self.word_fewer_keys_plover, self.words),
            self.word_more_keys_plover,
            Self::pct(self.word_more_keys_plover, self.words)
        );
        println!(
            "  fewer keys    vs lapwing: {} ({:.1}%)   more keys: {} ({:.1}%)",
            self.word_fewer_keys_lapwing,
            Self::pct(self.word_fewer_keys_lapwing, self.words),
            self.word_more_keys_lapwing,
            Self::pct(self.word_more_keys_lapwing, self.words)
        );
        println!(
            "  fewer strokes vs plover:  {} ({:.1}%)",
            self.word_fewer_strokes_plover,
            Self::pct(self.word_fewer_strokes_plover, self.words)
        );
        println!(
            "  fewer strokes vs lapwing: {} ({:.1}%)",
            self.word_fewer_strokes_lapwing,
            Self::pct(self.word_fewer_strokes_lapwing, self.words)
        );
        println!(
            "total keys: lyk {} plover {} lapwing {}",
            self.total_keys_lyk, self.total_keys_plover, self.total_keys_lapwing
        );
    }
}

fn score(passages: &[&str]) -> Stats {
    let setup = dictionary_fixture::setup();
    let lyk = dictionary_fixture::cheapest();
    let mut stats = Stats::default();

    for &passage in passages {
        let rows = rows_for(setup, &lyk, passage);
        let (mut p_keys, mut l_keys, mut w_keys) = (0u64, 0u64, 0u64);
        for row in &rows {
            stats.words += 1;
            let ours = cost(&row.lyk);
            if let Some((_, k)) = ours {
                p_keys += u64::from(k);
            }
            if let (Some((os, ok)), Some((ps, pk))) = (ours, cost(&row.plover)) {
                l_keys += u64::from(pk);
                match ok.cmp(&pk) {
                    std::cmp::Ordering::Less => stats.word_fewer_keys_plover += 1,
                    std::cmp::Ordering::Greater => stats.word_more_keys_plover += 1,
                    std::cmp::Ordering::Equal => {}
                }
                if os < ps {
                    stats.word_fewer_strokes_plover += 1;
                }
            }
            if let (Some((os, ok)), Some((ws, wk))) = (ours, cost(&row.lapwing)) {
                w_keys += u64::from(wk);
                match ok.cmp(&wk) {
                    std::cmp::Ordering::Less => stats.word_fewer_keys_lapwing += 1,
                    std::cmp::Ordering::Greater => stats.word_more_keys_lapwing += 1,
                    std::cmp::Ordering::Equal => {}
                }
                if os < ws {
                    stats.word_fewer_strokes_lapwing += 1;
                }
            }
        }
        stats.passages += 1;
        stats.total_keys_lyk += p_keys;
        stats.total_keys_plover += l_keys;
        stats.total_keys_lapwing += w_keys;
        match p_keys.cmp(&l_keys) {
            std::cmp::Ordering::Less => stats.passage_wins_plover += 1,
            std::cmp::Ordering::Greater => stats.passage_losses_plover += 1,
            std::cmp::Ordering::Equal => {}
        }
        match p_keys.cmp(&w_keys) {
            std::cmp::Ordering::Less => stats.passage_wins_lapwing += 1,
            std::cmp::Ordering::Greater => stats.passage_losses_lapwing += 1,
            std::cmp::Ordering::Equal => {}
        }
    }
    stats
}

/// Every threshold here was set just under what a real run against `PASSAGES` actually measured
/// (see `docs/decisions.md`'s quote-corpus entry for the exact numbers it was calibrated from), not
/// a loose, anything-passes sanity floor - deliberately harsh, so an incoming change that makes the
/// theory even slightly less terse fails this test immediately instead of drifting unnoticed. A
/// *real* improvement should raise these numbers right back up along with the thresholds, in the
/// same commit, not leave them stranded below what the theory now actually does.
mod threshold {
    /// Share of the hundred-odd passages whose total keys lyk beats outright.
    pub const PASSAGE_WIN_PCT_PLOVER: f64 = 60.0;
    pub const PASSAGE_WIN_PCT_LAPWING: f64 = 75.0;
    /// Share of individual words where lyk strictly beats the other theory on keys.
    pub const WORD_FEWER_KEYS_PCT_PLOVER: f64 = 18.0;
    pub const WORD_FEWER_KEYS_PCT_LAPWING: f64 = 33.0;
    /// Share of individual words where lyk strictly *loses* on keys - capped, so a change that
    /// quietly makes more words worse doesn't hide behind an unrelated set of words getting better.
    pub const WORD_MORE_KEYS_PCT_PLOVER_MAX: f64 = 9.0;
    pub const WORD_MORE_KEYS_PCT_LAPWING_MAX: f64 = 5.5;
    /// How much cheaper lyk's aggregate key total must be, as a percentage of the other theory's
    /// own total - the single hardest number to fake, since it is not swayed by lots of tiny common
    /// words tying (`a`, `the`, `is` cost the same everywhere) the way a per-word percentage is.
    pub const AGGREGATE_SAVINGS_PCT_PLOVER: f64 = 1.3;
    pub const AGGREGATE_SAVINGS_PCT_LAPWING: f64 = 8.0;
}

/// Prints the full breakdown (`cargo test --release quote_corpus -- --nocapture` to see it) and
/// holds the theory to the thresholds above.
#[test]
fn lyk_beats_plover_and_lapwing_across_a_hundred_unpicked_passages() {
    use threshold as t;
    let stats = score(PASSAGES);
    stats.report();

    let passage_win_pct_plover = Stats::pct(stats.passage_wins_plover, stats.passages);
    let passage_win_pct_lapwing = Stats::pct(stats.passage_wins_lapwing, stats.passages);
    let word_fewer_keys_pct_plover = Stats::pct(stats.word_fewer_keys_plover, stats.words);
    let word_fewer_keys_pct_lapwing = Stats::pct(stats.word_fewer_keys_lapwing, stats.words);
    let word_more_keys_pct_plover = Stats::pct(stats.word_more_keys_plover, stats.words);
    let word_more_keys_pct_lapwing = Stats::pct(stats.word_more_keys_lapwing, stats.words);
    let savings_pct_plover = (stats.total_keys_plover as f64 - stats.total_keys_lyk as f64) * 100.0
        / stats.total_keys_plover as f64;
    let savings_pct_lapwing = (stats.total_keys_lapwing as f64 - stats.total_keys_lyk as f64) * 100.0
        / stats.total_keys_lapwing as f64;

    assert!(
        passage_win_pct_plover >= t::PASSAGE_WIN_PCT_PLOVER,
        "only {passage_win_pct_plover:.1}% of passages beat Plover outright, want >= {}%",
        t::PASSAGE_WIN_PCT_PLOVER
    );
    assert!(
        passage_win_pct_lapwing >= t::PASSAGE_WIN_PCT_LAPWING,
        "only {passage_win_pct_lapwing:.1}% of passages beat Lapwing outright, want >= {}%",
        t::PASSAGE_WIN_PCT_LAPWING
    );
    assert!(
        word_fewer_keys_pct_plover >= t::WORD_FEWER_KEYS_PCT_PLOVER,
        "only {word_fewer_keys_pct_plover:.1}% of words beat Plover on keys, want >= {}%",
        t::WORD_FEWER_KEYS_PCT_PLOVER
    );
    assert!(
        word_fewer_keys_pct_lapwing >= t::WORD_FEWER_KEYS_PCT_LAPWING,
        "only {word_fewer_keys_pct_lapwing:.1}% of words beat Lapwing on keys, want >= {}%",
        t::WORD_FEWER_KEYS_PCT_LAPWING
    );
    assert!(
        word_more_keys_pct_plover <= t::WORD_MORE_KEYS_PCT_PLOVER_MAX,
        "{word_more_keys_pct_plover:.1}% of words cost more than Plover's, want <= {}%",
        t::WORD_MORE_KEYS_PCT_PLOVER_MAX
    );
    assert!(
        word_more_keys_pct_lapwing <= t::WORD_MORE_KEYS_PCT_LAPWING_MAX,
        "{word_more_keys_pct_lapwing:.1}% of words cost more than Lapwing's, want <= {}%",
        t::WORD_MORE_KEYS_PCT_LAPWING_MAX
    );
    assert!(
        savings_pct_plover >= t::AGGREGATE_SAVINGS_PCT_PLOVER,
        "lyk is only {savings_pct_plover:.2}% cheaper than Plover in aggregate, want >= {}%",
        t::AGGREGATE_SAVINGS_PCT_PLOVER
    );
    assert!(
        savings_pct_lapwing >= t::AGGREGATE_SAVINGS_PCT_LAPWING,
        "lyk is only {savings_pct_lapwing:.2}% cheaper than Lapwing in aggregate, want >= {}%",
        t::AGGREGATE_SAVINGS_PCT_LAPWING
    );
}
