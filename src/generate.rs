//! Putting the dictionary together: briefs first, then the rules, with clashes settled in a fixed
//! order so the result is predictable.

use crate::analysis::Setup;
use crate::ergonomics;
use crate::rules::Step;
use crate::stroke::{Stroke, parse_outline, render_outline};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, BinaryHeap, HashMap, HashSet};
use std::path::Path;

const BUILT_IN_BRIEFS: &str = include_str!("../rules/briefs.toml");
const BUILT_IN_SUFFIXES: &str = include_str!("../rules/suffixes.toml");

/// Every stroke that isn't a word at all: the commands, punctuation, and fingerspelling/OS-key
/// dictionaries deployed alongside the generated dictionary in a real keymux config. These load
/// at *higher* priority than the generated words there, so a word that happens to land on the
/// same stroke as one of these is never reachable — a real stroke, that was never a bug in the
/// deployed config, just unknown to the generator. Reserved the same way a brief's outline is
/// (see `generate`), so the collision can't resurface on the next regeneration.
const RESERVED: &[(&str, &str)] = &[
    ("a command", include_str!("../rules/reserved/commands.json")),
    ("a punctuation mark", include_str!("../rules/reserved/punctuation.json")),
    ("a fingerspelled letter or OS shortcut", include_str!("../rules/reserved/emilys-modifiers.json")),
];

/// A brief this short was hand-picked by a theory's authors, not derived from frequency: see
/// [`generate`].
const TINY_BRIEF_KEYS: u32 = 2;

#[derive(Deserialize, Default)]
pub struct Briefs {
    /// How many of the most common words get a brief
    #[serde(default)]
    count: usize,
    #[serde(default = "plover")]
    default: String,
    #[serde(default)]
    word: HashMap<String, String>,
}

fn plover() -> String {
    "plover".into()
}

impl Briefs {
    pub fn built_in() -> Result<Self> {
        Ok(toml::from_str(BUILT_IN_BRIEFS)?)
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).with_context(|| path.display().to_string())?;
        Ok(toml::from_str(&text)?)
    }
}

/// How a dictionary entry came about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Origin {
    /// A standalone suffix: a continuation stroke that stands for it on its own, not folded onto
    /// any particular word (see `rules/suffixes.toml`)
    Suffix,
    Brief,
    Rules,
    /// A long word written shorter by one of the shortening steps
    Short(Step),
    /// The spelling told it from a word that sounds the same
    Spelling,
    /// Another pronunciation of a word
    Alternate,
    /// The ladder step that settled a clash: 1 is the star on the first stroke
    Ladder(u8),
    /// Composed from one or two already-placed words' own outlines: a stem plus a bare
    /// continuation stroke for a reduced trailing syllable, a leading consonant folded onto
    /// another word's first stroke, or two whole words back to back — struck on top of however
    /// each piece is actually briefed or folded, so a learner who knows the pieces can always
    /// reach the word that way too (see the backup-entry pass in [`generate`]).
    Backup,
}

pub struct Entry {
    pub word: String,
    pub outline: Vec<Stroke>,
    pub origin: Origin,
    /// The word that has the outline this one wanted
    pub lost_to: Option<String>,
}

pub struct Generated {
    pub entries: Vec<Entry>,
    pub unresolved: Vec<String>,
    /// A backup entry (see [`Origin::Backup`]) whose composed outline another entry already had.
    pub backup_skipped: usize,
    /// A word whose own plain-rules outline costs less than its actual entry, and is sitting
    /// completely unclaimed: a regression guard, not something expected to ever hold more than a
    /// handful of words (see where it is built in [`generate`]).
    pub stranded: Vec<String>,
}

/// Among words that all share one exact pronunciation, the one whose own spelling also agrees
/// with `word`'s at the expected boundary — resolving an ambiguity the sounds alone can't (two
/// homophones, like `build`/`billed`, sharing the stem sounds `building` needs) the same way
/// spelling already breaks a tie between words that sound alike elsewhere in this theory (see
/// `docs/spelling.md`). Sound is still what found these candidates in the first place; spelling
/// only ever narrows a tie between them, never widens the search. `None` if nobody's spelling
/// agrees, or if more than one still does.
/// `sounds`, with each token's trailing stress digit removed — the same trimming
/// `Rules::syllables` already does to tell a vowel from a consonant, reused here so a word's own
/// primary-stress pronunciation can still match its secondary-stress form inside a compound
/// (`my`'s own `M AY1` against `myself`'s own `M AY2 ...`).
fn stress_stripped(sounds: &[String]) -> Vec<String> {
    sounds.iter().map(|s| s.trim_end_matches(|c: char| c.is_ascii_digit()).to_owned()).collect()
}

fn spelled_stem<'a>(word: &str, stems: &[&'a str], boundary: fn(&str, &str) -> bool) -> Option<&'a str> {
    match *stems {
        [stem] => Some(stem),
        _ => {
            let matches: Vec<&'a str> = stems.iter().copied().filter(|s| boundary(word, s)).collect();
            match matches[..] {
                [stem] => Some(stem),
                _ => None,
            }
        }
    }
}

/// One legal way to write a word's *primary* outline, considered on equal footing against every
/// other legal way regardless of where it came from - a gold brief, lyk's own plain rules reading,
/// a spelling-told-apart variant, a brevity-ladder compression, or a star-ladder step. See
/// [`enumerate_candidates`], which builds these, and `generate`'s own peeling/solver step, which
/// picks among them.
pub struct Candidate {
    pub outline: Vec<Stroke>,
    pub origin: Origin,
    pub cost: u32,
}

/// Intrinsic cost of an outline alone, independent of word or frequency: fewer strokes dominates,
/// fewer keys breaks ties within that, ergonomics only breaks ties within *that* - the same
/// priority `shortest`/`ladder` already encoded informally before this existed, made explicit and
/// total. The weight separation is deliberate: ergonomics (bounded well under 1,000 per its own
/// ceiling test) can never override a key-count difference, and a key-count difference can never
/// override a stroke-count difference - this preserves the theory's already-tuned priority order
/// rather than re-litigating it.
///
/// Frequency does NOT belong here - it is a property of a *word* wanting a candidate, not of the
/// candidate itself, reusable as-is by any other word considering the exact same outline. Frequency
/// only enters arbitration as the solver's own edge weight (`generate`'s own `edge_cost`), making
/// the global optimum the solver finds literally the same metric `build` already reports as "keys
/// per word, weighted by how often it is said" - the objective is the headline quality number
/// itself, not a proxy for it.
#[must_use]
pub fn candidate_cost(outline: &[Stroke]) -> u32 {
    const STROKE_WEIGHT: u32 = 1_000_000;
    const KEY_WEIGHT: u32 = 1_000;
    STROKE_WEIGHT * outline.len() as u32 + KEY_WEIGHT * keys(outline) + ergonomics::weighted_outline(outline)
}

/// Every raw and drop-repaired candidate from one gold dictionary's own listed outlines for
/// `word`, in list order: a fragment-safe raw outline as itself, plus its drop-repair whenever
/// `drop_fragments` finds one to offer alongside it. Unlike `best_brief`'s own pre-solver version
/// of this (now gone), a raw outline that only holds *part* of a real key combination is never
/// offered at all here - not even as a worse-ranked fallback - since the solver has a real
/// alternative (the repair, or another source's candidate entirely) to reach for instead of ever
/// needing to settle for a half-held chord. See `enumerate_candidates`'s own doc for why this is
/// new rigor over what `best_brief` used to allow.
fn gold_candidates(
    rules: &crate::rules::Rules,
    outlines: Option<&Vec<String>>,
    word: &str,
    sounds: &[String],
) -> Vec<Vec<Stroke>> {
    let mut found = Vec::new();
    for text in outlines.into_iter().flatten() {
        let Some(raw) = parse_outline(text) else { continue };
        if rules.matches_real_consonants(&raw, word, sounds) {
            found.push(raw.clone());
        }
        if let Some(repaired) = rules.drop_fragments(&raw, word, sounds) {
            found.push(repaired);
        }
    }
    found
}

/// Every legal, fragment-safe primary-outline candidate for `word` - backups excluded (composing
/// from an already-placed word's own outline is inherently iterative, the opposite of this
/// function's whole point; see the backup-entry pass further down in `generate`, which is
/// untouched by this). A pure function: no `taken`, no mutable state, no dependence on any other
/// word's decision, so it can be called in any order, any number of times, for a unit test or for
/// the real solver alike. Multiple candidates may render to the same outline text; callers dedupe,
/// keeping the cheaper one (see `generate`'s own peeling step) - this never happens itself.
///
/// Sources merged into one list, every one gated exactly as the mechanism it reuses already gates
/// it:
/// 1. **Gold-brief candidates.** A `[word]` literal override in `rules/briefs.toml` still forces
///    exactly one candidate and nothing else - a human decision, not something to reopen to
///    competition even against this word's *own* rules-derived or brevity candidates, the same way
///    a briefed word kept exactly one entry before this existed. Short of a literal override,
///    *both* `setup.gold.plover` and `setup.gold.lapwing` are merged unconditionally (today only
///    one source is ever tried, gated by `briefs.default`/`briefs.word`'s `"plover"`/`"lapwing"`
///    selector - that gating only ever narrows *which* dictionaries to merge now, never whether a
///    brief competes at all). `matches_real_consonants` is a blanket filter here on every raw
///    candidate (new rigor `best_brief` never had outside its own dropped-candidate sort) - see
///    `gold_candidates`.
/// 2. **Rules-derived candidate**: `setup.rules.outline(word, sounds)` - lyk's own plain reading,
///    unconditionally, for every word with a brief or not. This is the piece missing from the old
///    brief pool entirely, and the whole reason lyk's own derivation used to never get a chance to
///    compete against a common word's gold brief.
/// 3. **Homophone resolution, folded into legality rather than cost**: `rules.variant(word,
///    sounds)`, exactly the same call the old pre-solver homophone pass made per word, added
///    alongside the plain rules candidate rather than replacing it. `variant` only ever returns
///    `Some` for the member of a sound-alike group whose own spelling needs telling apart from the
///    others (`billed` against `build`); the member that already matches the default convention
///    gets `None` and simply keeps competing on its plain reading alone. This answers "what text
///    may this word even propose," not "who should win it" - a homophone fight is then just an
///    ordinary contested-text fight the solver settles on cost and frequency weight, same as any
///    other collision.
/// 4. **Brevity-ladder candidates**: identical eligibility gate to before (`setup.rank[word] <
///    step.rank && first.len() > 1`), every `Shortening` from `setup.rules.brevity()`, every
///    `setup.rules.shorten(...)` result shorter than the full form, plus its two star-ladder
///    variants (`Origin::Short(step.step)`) - mirrors the old asymmetry where short forms get fewer
///    ladder steps than full outlines.
/// 5. **Full-outline ladder candidates**: the rules-derived outline *and* the spelling variant (if
///    it has one), each with all four `ladder(outline, join)` steps (`Origin::Ladder(n)`) - a
///    generalization of the old behavior, which only ever laddered a word that fell through to the
///    main loop, never one that had already claimed a spelling variant.
///
/// Every word is guaranteed at least one candidate (source 2) unless its own rules genuinely
/// cannot write it in a single stroke at all (a real, pre-existing failure mode -
/// `setup.rules.outline` already returns `None` for that today - not a new one introduced here); a
/// pathological word with neither a gold brief nor a rules outline has zero candidates and is
/// routed to `unresolved` by peeling, exactly as it would have been unresolved before.
pub fn enumerate_candidates(
    setup: &Setup,
    briefs: &Briefs,
    word: &str,
    simple: &HashSet<&str>,
    wanted: &HashSet<&str>,
) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = Vec::new();
    let Some(prons) = setup.pron.get(word) else {
        return out;
    };
    let Some(sounds) = prons.first() else {
        return out;
    };
    let join = setup.rules.join_key();

    let push = |out: &mut Vec<Candidate>, mut outline: Vec<Stroke>, origin: Origin| {
        setup.rules.joined(&mut outline);
        let cost = candidate_cost(&outline);
        out.push(Candidate { outline, origin, cost });
    };

    // 1. Gold-brief candidates (see this function's own doc). A literal override forces exactly
    // one candidate and returns immediately - nothing else in this word's list competes with it.
    if let Some(explicit) = briefs.word.get(word) {
        match explicit.as_str() {
            "plover" => {
                for outline in gold_candidates(&setup.rules, setup.gold.plover.get(word), word, sounds) {
                    push(&mut out, outline, Origin::Brief);
                }
            }
            "lapwing" => {
                for outline in gold_candidates(&setup.rules, setup.gold.lapwing.get(word), word, sounds) {
                    push(&mut out, outline, Origin::Brief);
                }
            }
            literal => {
                if let Some(outline) = parse_outline(literal) {
                    push(&mut out, outline, Origin::Brief);
                }
                return out;
            }
        }
    } else {
        for outline in gold_candidates(&setup.rules, setup.gold.plover.get(word), word, sounds) {
            push(&mut out, outline, Origin::Brief);
        }
        for outline in gold_candidates(&setup.rules, setup.gold.lapwing.get(word), word, sounds) {
            push(&mut out, outline, Origin::Brief);
        }
    }

    // 2. Rules-derived candidate: lyk's own plain reading, every time, brief or not.
    let base = setup.rules.outline(word, sounds);
    if let Some(outline) = &base {
        push(&mut out, outline.clone(), Origin::Rules);
    }

    // 3. Homophone resolution folded into legality: see this function's own doc for why this is
    // always offered alongside the plain reading rather than instead of it.
    let variant = setup.rules.variant(word, sounds);
    if let Some(outline) = &variant {
        push(&mut out, outline.clone(), Origin::Spelling);
    }

    // 4. Brevity-ladder candidates: a long word written shorter by one of the shortening steps,
    // gated by rank exactly as before.
    if let Some(first) = &base {
        let own = setup.rank.get(word).copied().unwrap_or(usize::MAX);
        if first.len() > 1 {
            for short in setup.rules.brevity().iter().filter(|s| own < s.rank) {
                let mut seen_this_step: HashSet<Vec<Stroke>> = HashSet::new();
                for pron_sounds in prons {
                    for outline in setup.rules.shorten(*short, word, pron_sounds) {
                        if outline.len() >= first.len() {
                            continue;
                        }
                        let starred: Vec<Vec<Stroke>> = ladder(&outline, join)
                            .into_iter()
                            .filter(|(n, _)| *n <= 2)
                            .map(|(_, o)| o)
                            .collect();
                        for candidate in std::iter::once(outline).chain(starred) {
                            let text = render_outline(&candidate);
                            if simple.contains(text.as_str()) || wanted.contains(text.as_str()) {
                                continue;
                            }
                            if !seen_this_step.insert(candidate.clone()) {
                                continue;
                            }
                            push(&mut out, candidate, Origin::Short(short.step));
                        }
                    }
                }
            }
        }
    }

    // 5. Full-outline ladder candidates: the star ladder on top of the plain reading, and on top
    // of the spelling variant too, when it has one.
    for outline in [base.as_ref(), variant.as_ref()].into_iter().flatten() {
        for (n, laddered) in ladder(outline, join) {
            push(&mut out, laddered, Origin::Ladder(n));
        }
    }

    out
}

/// A min-cost bipartite assignment over the contested subgraph only: words with edges to a shared
/// pool of outline-text nodes, each usable by at most one word. Solved by successive shortest
/// augmenting paths (hand-rolled, not dense Hungarian - O(n^3) is infeasible and wasteful for a
/// sparse graph here - and not a crate: every edge cost is non-negative by construction, so
/// repeated Dijkstra with Johnson-style potentials, initialized to zero, needs no Bellman-Ford
/// seeding pass; `Cargo.toml` has five dependencies today, none a graph library, and this project's
/// whole ethos - exhaustive justification, exact and explainable - argues against an opaque
/// third-party solver for the single piece of logic now most load-bearing for every word's primary
/// outline).
///
/// `contested[i] = (word, candidates)`, each candidate paired with its own *edge* cost (frequency
/// weight already folded in - see `generate`'s own `edge_cost`), widened to `u64` here rather than
/// the `u32` a bare `candidate_cost` fits in: a weight up to ~100,000 times a multi-stroke,
/// multi-star candidate's cost can exceed `u32::MAX`, and there is no reason to risk that wrapping
/// silently in the one place a wrong number would be hardest to notice. Returns, for every
/// contested word, the index into its own candidate list it was assigned - asserted to cover every
/// word passed in: given every word here has at least one real edge (peeling already routed a
/// zero-candidate word to `unresolved` before this ever runs), a perfect assignment must exist, so
/// a flow that saturates anything less than every word is a bug in this function, not a legitimate
/// shortfall to paper over silently.
fn solve_contested<'a>(contested: &[(&'a str, Vec<(Candidate, u64)>)]) -> HashMap<&'a str, usize> {
    if contested.is_empty() {
        return HashMap::new();
    }

    // Node numbering: 0 = source, 1..=W = one per word, W+1..=W+T = one per distinct outline text,
    // W+T+1 = sink.
    let word_count = contested.len();
    let mut text_index: HashMap<String, usize> = HashMap::new();
    // For each word: (text node offset, edge cost) per candidate, in the word's own candidate order
    // - the candidate's own index in this word's list is implicit in its position here.
    let mut word_edges: Vec<Vec<(usize, u64)>> = vec![Vec::new(); word_count];

    for (w, (_, candidates)) in contested.iter().enumerate() {
        for (candidate, cost) in candidates {
            let text = render_outline(&candidate.outline);
            let next = text_index.len();
            let t = *text_index.entry(text).or_insert(next);
            word_edges[w].push((t, *cost));
        }
    }

    let text_count = text_index.len();
    let source = 0usize;
    let word_node = |w: usize| 1 + w;
    let text_node = |t: usize| 1 + word_count + t;
    let sink = 1 + word_count + text_count;
    let node_count = sink + 1;

    // Residual capacity and cost for every directed edge actually used (adjacency list of
    // (to, cap_remaining, cost, reverse_edge_index) per node - a standard residual graph).
    #[derive(Clone)]
    struct Edge {
        to: usize,
        cap: u32,
        cost: i64,
    }
    let mut graph: Vec<Vec<Edge>> = vec![Vec::new(); node_count];
    let add_edge = |graph: &mut Vec<Vec<Edge>>, from: usize, to: usize, cap: u32, cost: i64| {
        let fwd = graph[from].len();
        let rev = graph[to].len();
        graph[from].push(Edge { to, cap, cost });
        graph[to].push(Edge { to: from, cap: 0, cost: -cost });
        (from, fwd, to, rev)
    };
    // Track each edge's reverse location explicitly, since Rust can't hold two mutable borrows of
    // `graph` for the forward/reverse pair at once the way a C-style index pair would.
    let mut rev_of: HashMap<(usize, usize), (usize, usize)> = HashMap::new();
    let link = |graph: &mut Vec<Vec<Edge>>, rev_of: &mut HashMap<(usize, usize), (usize, usize)>, from: usize, to: usize, cap: u32, cost: i64| {
        let (f, fi, t, ti) = add_edge(graph, from, to, cap, cost);
        rev_of.insert((f, fi), (t, ti));
        rev_of.insert((t, ti), (f, fi));
    };

    for w in 0..word_count {
        link(&mut graph, &mut rev_of, source, word_node(w), 1, 0);
        for &(t, cost) in &word_edges[w] {
            link(&mut graph, &mut rev_of, word_node(w), text_node(t), 1, i64::try_from(cost).unwrap_or(i64::MAX));
        }
    }
    for t in 0..text_count {
        link(&mut graph, &mut rev_of, text_node(t), sink, 1, 0);
    }

    let mut potential = vec![0i64; node_count];
    let mut flow = 0usize;

    while flow < word_count {
        // Dijkstra over reduced costs `cost(u,v) + potential[u] - potential[v]`, which stay
        // non-negative as long as `potential` was last updated by a real shortest-path run - true
        // on the first iteration (all zero, all real costs non-negative) and maintained below.
        let mut dist = vec![i64::MAX; node_count];
        let mut prev_edge: Vec<Option<(usize, usize)>> = vec![None; node_count];
        dist[source] = 0;
        let mut heap: BinaryHeap<std::cmp::Reverse<(i64, usize)>> = BinaryHeap::new();
        heap.push(std::cmp::Reverse((0, source)));
        while let Some(std::cmp::Reverse((d, u))) = heap.pop() {
            if d > dist[u] {
                continue;
            }
            for (ei, edge) in graph[u].iter().enumerate() {
                if edge.cap == 0 {
                    continue;
                }
                let reduced = edge.cost + potential[u] - potential[edge.to];
                let nd = d + reduced;
                if nd < dist[edge.to] {
                    dist[edge.to] = nd;
                    prev_edge[edge.to] = Some((u, ei));
                    heap.push(std::cmp::Reverse((nd, edge.to)));
                }
            }
        }
        if dist[sink] == i64::MAX {
            break;
        }
        for (v, slot) in potential.iter_mut().enumerate() {
            if dist[v] < i64::MAX {
                *slot += dist[v];
            }
        }
        // Walk the augmenting path once more, pushing one unit of flow along it (every capacity
        // here is 1, so there is never a smaller bottleneck to find first) - this only ever moves
        // flow between a word's own candidate edges (see this function's own doc: the source-word
        // edge, once saturated, is never the one that gets displaced), so the final matching is
        // read off directly from edge capacities once flow is complete, not tracked here.
        let mut v = sink;
        let mut path: Vec<(usize, usize)> = Vec::new();
        while let Some((u, ei)) = prev_edge[v] {
            path.push((u, ei));
            v = u;
        }
        for (u, ei) in path {
            let to = graph[u][ei].to;
            graph[u][ei].cap -= 1;
            let &(ru, ri) = rev_of.get(&(u, ei)).expect("every forward edge has a tracked reverse");
            debug_assert_eq!(graph[ru][ri].to, u);
            graph[to][ri].cap += 1;
        }
        flow += 1;
    }

    assert_eq!(
        flow, word_count,
        "the contested-subgraph solver failed to match every word - every word here is guaranteed \
         at least one edge by enumeration, so this is a bug in solve_contested, not a legitimate \
         shortfall"
    );
    // The source-word edge (index 0 in each word node's own adjacency list) is always saturated
    // and never displaced once flow is complete (see the loop above); exactly one of the
    // candidate edges after it is the one actually carrying flow - that candidate's index in the
    // word's own list is what `generate` asked for.
    let mut result = HashMap::new();
    for (w, (word, _)) in contested.iter().enumerate() {
        let candidate_index = graph[word_node(w)]
            .iter()
            .skip(1)
            .position(|edge| edge.cap == 0)
            .expect("a fully matched word has exactly one saturated candidate edge");
        result.insert(*word, candidate_index);
    }
    result
}

/// One candidate composed outline from the exhaustive backup pass in [`generate`]: checked for a
/// real fragment (the same safety net every composition kind already used before this pass was
/// unified) and for a collision, then either added as a new [`Origin::Backup`] entry or counted
/// in `backup_skipped`. `added_this_round` is what the fixed-point loop in `generate` watches to
/// know when nothing further is left to find.
#[allow(clippy::too_many_arguments)]
fn try_add_backup(
    setup: &Setup,
    entries: &mut Vec<Entry>,
    taken: &mut HashMap<String, String>,
    backup_skipped: &mut usize,
    added_this_round: &mut usize,
    word: &str,
    sounds: &[String],
    composed: Vec<Stroke>,
) {
    if !setup.rules.matches_real_consonants(&composed, word, sounds) {
        return;
    }
    let text = render_outline(&composed);
    if taken.contains_key(&text) {
        *backup_skipped += 1;
        return;
    }
    taken.insert(text, word.to_owned());
    entries.push(Entry {
        word: word.to_owned(),
        outline: composed,
        origin: Origin::Backup,
        lost_to: None,
    });
    *added_this_round += 1;
}

/// The shortest outline (fewest strokes, then fewest keys) from a list.
pub fn shortest(outlines: &[String]) -> Option<Vec<Stroke>> {
    outlines
        .iter()
        .filter_map(|o| parse_outline(o))
        .min_by_key(|o| (o.len(), keys(o)))
}

/// Among a word's candidate outlines, the one a brief should actually use: fewest strokes, but
/// preferring one whose first and last consonants read, under lyk's own rules, as the word's real
/// first and last sounds, before falling back to fewest keys. Plover and Lapwing's own shortest
/// outline for a word is sometimes their own theory's memorized one-off trick, not derived from
/// either theory's general rules — which can mean only part of a two-key consonant is actually
/// left in the stroke (`good`'s Plover brief `KPWAOD` drops the `T` of `TKPW`'s own `G`, `work`'s
/// `WORB` drops the `G` of `-BG`'s own `K`). Where that happens, two repairs are tried alongside
/// the original candidates, not just one: completing the fragment into the word's whole, correct
/// sound (what used to be the only fallback), and dropping the fragment outright, since a brief
/// is always free to leave out a sound it isn't attempting at all — only holding *part* of one is
/// disallowed. Dropping is usually the fewer keys of the two, so it's what wins when it's free;
/// both are only ever offered when the word's own gold dictionary pointed at the sound to begin
/// with, never invented from nothing.
fn best_brief(
    outlines: &[String],
    rules: &crate::rules::Rules,
    word: &str,
    sounds: &[String],
    natural: &HashMap<String, &str>,
    taken: &HashMap<String, String>,
) -> Option<Vec<Stroke>> {
    // Whether a candidate is one of the word's own gold outlines (never second-guessed by a
    // collision below — a hand-picked Plover brief like "and"'s `SKP` is allowed to sit on
    // whatever some other, unrelated word would have wanted by the plain rules; that trade is
    // this whole mechanism's normal, accepted cost, not a bug) or this function's own synthesized
    // drop repair (which is).
    let mut candidates: Vec<(bool, Vec<Stroke>)> = outlines
        .iter()
        .filter_map(|o| parse_outline(o))
        .flat_map(|o| {
            let repaired = rules.drop_fragments(&o, word, sounds).map(|d| (true, d));
            std::iter::once((false, o)).chain(repaired)
        })
        .collect();
    // A *dropped* sound that happens to land exactly on a different word's own plain outline
    // isn't a saving: it just hands that other word a ladder fight it would never otherwise have
    // had, almost always a shorter, commoner word than the one being briefed (`fact` dropping its
    // own `K` entirely used to take `fat`'s rightful, un-briefed `TPAT`). Only the synthesized
    // drop candidate is checked — a raw gold outline colliding with another word is the ordinary,
    // already-accepted case below, not this one. Checked after the key-count/fragment comparisons
    // that matter when there's no such collision, so a candidate only pays this cost when it
    // would otherwise have won anyway.
    candidates.sort_by_key(|(dropped, o)| {
        let collides = *dropped && natural.get(&render_outline(o)).is_some_and(|&owner| owner != word);
        // Two of Plover's own gold outlines sometimes tie on every measure above (`best`'s own
        // `PW*ES` and `PWEFT` both cost 5 keys) — a star used only as a tie-break coin flip in
        // their own source, not for anything this word actually needs disambiguated. Preferring
        // the star-free one when every other measure is equal costs nothing and never overrides
        // a real win on length, fragment-safety, collision avoidance or key count above it.
        let starred = o.iter().any(|s| s.0 & Stroke::STAR != 0);
        (o.len(), !rules.matches_real_consonants(o, word, sounds), collides, keys(o), starred)
    });
    // The best candidate by that order is usually free, but not always: the commonest word to
    // ever want an outline keeps it (that's the whole point of settling words commonest-first),
    // so a rarer word's own top pick can already be spoken for. Rather than giving up outright —
    // which used to mean falling all the way back to the word's full, multi-stroke phonetic
    // outline even when its second- or third-choice brief candidate was sitting right there,
    // untaken, and still far shorter — this tries every candidate in the same fragment-safe order
    // and takes the first one nobody else already has.
    candidates
        .into_iter()
        .map(|(_, o)| o)
        .find(|o| !taken.contains_key(&render_outline(o)))
}

/// How many keys an outline takes to press.
pub fn keys(outline: &[Stroke]) -> u32 {
    outline.iter().map(|s| s.0.count_ones()).sum()
}

/// Every word `generate` is willing to consider at all, most common first: a real pronunciation on
/// record, inside the requested frequency window, plain lowercase (apostrophes aside), with a
/// single-letter word only kept for the two that are real English words rather than frequency-list
/// noise. Factored out so a test can build the exact same `simple`/`wanted` sets `generate` does
/// (see [`group_outline_sets`]) without re-deriving this filter by hand.
fn ranked_words(setup: &Setup, common: usize) -> Vec<(&String, usize)> {
    let mut words: Vec<(&String, usize)> = setup
        .rank
        .iter()
        .filter(|(w, r)| {
            **r < common
                && setup.pron.contains_key(w.as_str())
                && (w.len() >= 2 || w.as_str() == "a" || w.as_str() == "i")
                && w.bytes().all(|b| b.is_ascii_lowercase() || b == b'\'')
        })
        .map(|(w, r)| (w, *r))
        .collect();
    words.sort_by_key(|(w, r)| (*r, w.as_str()));
    words
}

/// The texts a one-syllable word's own plain rules would produce (`simple`) and the texts *any*
/// word's plain rules would produce (`wanted`) - the same two sets `generate` has always built
/// before settling a single word, now feeding [`enumerate_candidates`]'s own brevity-ladder gate
/// instead of a dedicated pre-pass: a brevity candidate is never offered an outline that one of
/// these texts already names, the same way a learner could already work either one out by the
/// plain rules without the brief being guessable at all.
fn group_outline_sets(setup: &Setup, words: &[(&String, usize)]) -> (HashSet<String>, HashSet<String>) {
    let mut groups: HashMap<String, Vec<&str>> = HashMap::new();
    for (word, _) in words {
        if let Some(outline) = setup.pron[word.as_str()]
            .iter()
            .find_map(|sounds| setup.rules.outline(word, sounds))
        {
            groups.entry(render_outline(&outline)).or_default().push(word.as_str());
        }
    }
    let mut simple: HashSet<String> = HashSet::new();
    for (text, members) in &groups {
        let one_syllable = |w: &&str| {
            setup.pron[*w][0]
                .iter()
                .filter(|s| s.ends_with(|c: char| c.is_ascii_digit()))
                .count()
                == 1
        };
        if members.iter().any(one_syllable) {
            simple.insert(text.clone());
        }
    }
    let wanted: HashSet<String> = groups.keys().cloned().collect();
    (simple, wanted)
}

/// The other outlines to try for a word whose outline is taken, in the order they are tried.
fn ladder(outline: &[Stroke], join: bool) -> Vec<(u8, Vec<Stroke>)> {
    let mut steps = Vec::new();
    let star = |outline: &[Stroke], at: usize| -> Option<Vec<Stroke>> {
        let stroke = outline.get(at)?;
        (stroke.0 & Stroke::STAR == 0).then(|| {
            let mut next = outline.to_vec();
            next[at] = Stroke(stroke.0 | Stroke::STAR);
            next
        })
    };
    if let Some(next) = star(outline, 0) {
        steps.push((1, next));
    }
    if outline.len() > 1
        && let Some(next) = star(outline, outline.len() - 1)
    {
        steps.push((2, next));
    }
    // A stroke of just the star after the word marks it as the other one
    let marker = Stroke(Stroke::STAR | if join { Stroke::JOIN } else { 0 });
    for (n, extra) in [(3u8, 1usize), (4, 2)] {
        let mut next = outline.to_vec();
        next.extend(std::iter::repeat_n(marker, extra));
        steps.push((n, next));
    }
    steps
}

/// Every outline a real keymux config already gives to a command, a punctuation mark, or
/// fingerspelling - never available to a word, whatever the rules would have generated for it.
/// Pulled out of [`generate`] so a test can confirm the invariant it exists to guarantee (no
/// generated word ever lands on one of these) without re-deriving this list by hand.
pub(crate) fn reserved_outlines() -> Result<HashMap<String, String>> {
    let mut taken = HashMap::new();
    for (kind, json) in RESERVED {
        let strokes: HashMap<String, String> = serde_json::from_str(json)
            .with_context(|| format!("rules/reserved: {kind} dictionary"))?;
        for outline_text in strokes.keys() {
            let Some(outline) = parse_outline(outline_text) else {
                continue;
            };
            let text = render_outline(&outline);
            taken.entry(text).or_insert_with(|| format!("reserved ({kind})"));
        }
    }
    Ok(taken)
}

pub fn generate(setup: &Setup, briefs: &Briefs, common: usize) -> Result<Generated> {
    let mut taken: HashMap<String, String> = reserved_outlines()?;
    let mut entries: Vec<Entry> = Vec::new();
    let mut pinned: HashSet<String> = HashSet::new();

    // Standalone suffixes, first of all: a word that would otherwise land on the same outline
    // as one of these loses to it, the same way a brief does, since a suffix is struck on its
    // own far more often than any one rare word would want that exact outline.
    let suffixes: BTreeMap<String, String> = toml::from_str(BUILT_IN_SUFFIXES)?;
    for (outline_text, word) in &suffixes {
        let outline = parse_outline(outline_text)
            .with_context(|| format!("{outline_text:?} in rules/suffixes.toml"))?;
        let text = render_outline(&outline);
        if let Some(holder) = taken.get(&text) {
            anyhow::bail!("suffix {word:?} wants the outline {holder:?} already has");
        }
        taken.insert(text, word.clone());
        entries.push(Entry {
            word: word.clone(),
            outline,
            origin: Origin::Suffix,
            lost_to: None,
        });
    }

    // The pronoun "I": the frequency list only ever has it lowercase, indistinguishable from
    // the letter name "i" ("spell that: i, c, e"), and the letter already won that one word's
    // only slot (see the single-letter carve-out below). A capitalized word needs an entry of
    // its own regardless, so it gets one directly, on Plover's own real convention for it.
    let outline = parse_outline("EU").context("\"EU\" for the pronoun \"I\"")?;
    let text = render_outline(&outline);
    if let Some(holder) = taken.get(&text) {
        anyhow::bail!("the pronoun \"I\" wants the outline {text:?} {holder:?} already has");
    }
    taken.insert(text, "I".to_string());
    entries.push(Entry {
        word: "I".to_string(),
        outline,
        origin: Origin::Brief,
        lost_to: None,
    });

    // Words, most common first
    let mut words: Vec<(&String, usize)> = setup
        .rank
        .iter()
        .filter(|(w, r)| {
            // A single letter is almost always noise in a frequency list (a stray scan
            // artifact, a placeholder), except the two that are real English words
            **r < common
                && setup.pron.contains_key(w.as_str())
                && (w.len() >= 2 || w.as_str() == "a" || w.as_str() == "i")
                && w.bytes().all(|b| b.is_ascii_lowercase() || b == b'\'')
        })
        .map(|(w, r)| (w, *r))
        .collect();
    words.sort_by_key(|(w, r)| (*r, w.as_str()));

    // Briefs: the common words Plover or Lapwing already write, plus any word either of them
    // writes in two keys or fewer, however rare: a brief that short was hand-picked, not derived
    // from frequency, so frequency is not what should gate keeping it. A word named explicitly in
    // `[word]` is a third way in, the same reasoning extended past "two keys or fewer" to "someone
    // looked at this specific word and decided its brief was worth keeping": the frequency window
    // is a default for words nobody has looked at individually, not a ceiling on them.
    let has_tiny_brief = |outlines: Option<&Vec<String>>| {
        outlines.and_then(|o| shortest(o)).is_some_and(|o| keys(&o) <= TINY_BRIEF_KEYS)
    };

    // A dry run of the brief loop below, exactly as the original algorithm picks it (no collision
    // awareness: an empty `natural` map never flags anything as colliding), to find which words
    // actually end up relying on their own plain outline because nothing better came of their own
    // brief attempt. Only those words are worth a different word paying extra keys to avoid
    // colliding with below — a word that lands a brief of its own regardless (`a`, pinned to
    // `AEU`) never needs its plain reading (`A`) at all, so it shouldn't block some other word
    // from cleanly dropping a sound down to that same text (see `fact`'s own drop candidate `TPAT`
    // falsely "colliding" with `a`'s unused plain reading before this check existed).
    let empty_natural: HashMap<String, &str> = HashMap::new();
    let mut baseline_taken = taken.clone();
    let mut baseline_pinned: HashSet<&str> = HashSet::new();
    for (word, rank) in &words {
        let plover = setup.gold.plover.get(word.as_str());
        let lapwing = setup.gold.lapwing.get(word.as_str());
        let named = briefs.word.contains_key(word.as_str());
        if *rank >= briefs.count && !has_tiny_brief(plover) && !has_tiny_brief(lapwing) && !named {
            continue;
        }
        let sounds = &setup.pron[word.as_str()][0];
        let source = briefs.word.get(word.as_str()).unwrap_or(&briefs.default);
        let outline = match source.as_str() {
            "plover" => {
                plover.and_then(|o| best_brief(o, &setup.rules, word, sounds, &empty_natural, &baseline_taken))
            }
            "lapwing" => {
                lapwing.and_then(|o| best_brief(o, &setup.rules, word, sounds, &empty_natural, &baseline_taken))
            }
            explicit => parse_outline(explicit),
        };
        let outline = outline
            .or_else(|| {
                setup.gold.plover.get(word.as_str()).and_then(|o| {
                    best_brief(o, &setup.rules, word, sounds, &empty_natural, &baseline_taken)
                })
            })
            .or_else(|| {
                setup.gold.lapwing.get(word.as_str()).and_then(|o| {
                    best_brief(o, &setup.rules, word, sounds, &empty_natural, &baseline_taken)
                })
            });
        let Some(outline) = outline else { continue };
        let text = render_outline(&outline);
        if baseline_taken.contains_key(&text) {
            continue;
        }
        baseline_taken.insert(text, (*word).clone());
        baseline_pinned.insert(word.as_str());
    }

    // Every common-enough word's own outline by the plain rules, so a brief below can tell a
    // genuine saving from one that only moves a collision onto a word a learner would also know
    // (see `best_brief`). Limited to words inside the same "commonly known" window a brief itself
    // uses (`briefs.count`) that the dry run above shows are NOT themselves landing a brief:
    // losing a plain outline to a brief is this whole mechanism's normal, accepted cost for a rare
    // word (that's what the ladder is for), so only a collision with a word that actually needs
    // this outline is worth a brief paying extra keys to avoid. The commonest word keeps a
    // contested outline, since `words` is already sorted by rank.
    let mut natural: HashMap<String, &str> = HashMap::new();
    for (word, rank) in &words {
        if *rank >= briefs.count || baseline_pinned.contains(word.as_str()) {
            continue;
        }
        if let Some(outline) = setup.pron[word.as_str()]
            .iter()
            .find_map(|sounds| setup.rules.outline(word, sounds))
        {
            natural.entry(render_outline(&outline)).or_insert(word.as_str());
        }
    }

    for (word, rank) in &words {
        let plover = setup.gold.plover.get(word.as_str());
        let lapwing = setup.gold.lapwing.get(word.as_str());
        let named = briefs.word.contains_key(word.as_str());
        if *rank >= briefs.count && !has_tiny_brief(plover) && !has_tiny_brief(lapwing) && !named {
            continue;
        }
        let sounds = &setup.pron[word.as_str()][0];
        let source = briefs.word.get(word.as_str()).unwrap_or(&briefs.default);
        let outline = match source.as_str() {
            "plover" => plover.and_then(|o| best_brief(o, &setup.rules, word, sounds, &natural, &taken)),
            "lapwing" => lapwing.and_then(|o| best_brief(o, &setup.rules, word, sounds, &natural, &taken)),
            explicit => {
                let outline = parse_outline(explicit)
                    .with_context(|| format!("{word:?}'s brief {explicit:?} in rules/briefs.toml"))?;
                let text = render_outline(&outline);
                // A literal outline in `[word]` was looked at and picked by hand, unlike a
                // `plover`/`lapwing` source's own collision-avoiding `best_brief` call above: if
                // it's already spoken for, that is always a stale or wrong entry worth fixing in
                // `rules/briefs.toml`, never something to fall back from silently. `up`'s own
                // `"-P"` sat this way for a long time, unnoticed, quietly losing every build to
                // `rules/reserved/punctuation.json`'s own `"-P"` and falling back to the
                // two-key `UP` the rules generate anyway, with nothing printed to say so.
                if let Some(holder) = taken.get(&text) {
                    anyhow::bail!(
                        "{word:?}'s brief ({explicit:?} in rules/briefs.toml) wants the outline \
                         {text:?}, which {holder:?} already has"
                    );
                }
                Some(outline)
            }
        };
        // A word only one of them writes still gets its brief
        let outline = outline
            .or_else(|| {
                setup
                    .gold
                    .plover
                    .get(word.as_str())
                    .and_then(|o| best_brief(o, &setup.rules, word, sounds, &natural, &taken))
            })
            .or_else(|| {
                setup
                    .gold
                    .lapwing
                    .get(word.as_str())
                    .and_then(|o| best_brief(o, &setup.rules, word, sounds, &natural, &taken))
            });
        let Some(outline) = outline else { continue };
        let text = render_outline(&outline);
        if taken.contains_key(&text) {
            continue;
        }
        taken.insert(text, (*word).clone());
        pinned.insert((*word).clone());
        entries.push(Entry {
            word: (*word).clone(),
            outline,
            origin: Origin::Brief,
            lost_to: None,
        });
    }

    let join = setup.rules.join_key();
    let mut unresolved = Vec::new();

    // Words that sound the same and would have the same outline: the spelling decides which one
    // moves to its own, before frequency decides anything
    let usual = |word: &str| {
        setup.pron[word]
            .iter()
            .find_map(|sounds| setup.rules.outline(word, sounds))
    };
    let mut groups: HashMap<String, Vec<&str>> = HashMap::new();
    for (word, _) in &words {
        if let Some(outline) = usual(word) {
            groups
                .entry(render_outline(&outline))
                .or_default()
                .push(word.as_str());
        }
    }
    // The outlines that one-syllable words have by the rules: the ones a learner would work out
    let mut simple: HashSet<&String> = HashSet::new();
    for (text, members) in &groups {
        let one_syllable = |w: &&str| {
            setup.pron[*w][0]
                .iter()
                .filter(|s| s.ends_with(|c: char| c.is_ascii_digit()))
                .count()
                == 1
        };
        if members.iter().any(one_syllable) {
            simple.insert(text);
        }
    }
    // The outlines some word has by the rules
    let wanted: HashSet<&String> = groups.keys().collect();
    let mut spelled: HashSet<String> = HashSet::new();
    // Words whose outline another word has too, or a brief has already: the commonest first, so
    // `heal` gets HAEL before `hale` asks for it
    let mut clashing: Vec<&str> = groups
        .iter()
        .filter(|(text, members)| members.len() > 1 || taken.contains_key(text.as_str()))
        .flat_map(|(_, members)| members.iter().copied())
        .filter(|w| !pinned.contains(*w))
        .collect();
    clashing.sort_by_key(|w| (setup.rank[*w], *w));
    {
        for word in &clashing {
            let Some(mut outline) = setup.pron[*word]
                .iter()
                .find_map(|sounds| setup.rules.variant(word, sounds))
            else {
                continue;
            };
            setup.rules.joined(&mut outline);
            let text = render_outline(&outline);
            if taken.contains_key(&text) || wanted.contains(&text) {
                continue;
            }
            taken.insert(text, (*word).to_owned());
            spelled.insert((*word).to_owned());
            entries.push(Entry {
                word: (*word).to_owned(),
                outline,
                origin: Origin::Spelling,
                lost_to: None,
            });
        }
    }
    for (word, _) in &words {
        if spelled.contains(word.as_str()) {
            continue;
        }
        // A word with a brief keeps it as its only entry: one outline, one way to learn it, and
        // whatever its rule outline or short forms would have been stays free for some other
        // word to claim instead. Words are handled once, in rank order, with `taken` checked at
        // the moment each one claims an outline, so never claiming these on a briefed word's
        // behalf is enough on its own for a later, less common word to pick them up as its own —
        // no separate pass is needed.
        if pinned.contains(word.as_str()) {
            continue;
        }
        let mut outlines: Vec<Vec<Stroke>> = Vec::new();
        for sounds in &setup.pron[word.as_str()] {
            if let Some(outline) = setup.rules.outline(word, sounds)
                && !outlines.contains(&outline)
            {
                outlines.push(outline);
            }
        }
        let Some((first, others)) = outlines.split_first() else {
            unresolved.push((*word).clone());
            continue;
        };
        let mut placed = false;
        let mut lost_to: Option<String> = None;
        // The commonest long words go first for the fewest keys: the shortening steps, in order.
        // Only where they save a stroke; the outline by the rules is added as well. Each short
        // form is a group of its plain and starred forms, of which the first free one is used.
        let mut options: Vec<(Origin, Vec<Vec<Stroke>>)> = Vec::new();
        let own = setup.rank[word.as_str()];
        for short in setup
            .rules
            .brevity()
            .iter()
            .filter(|s| own < s.rank && first.len() > 1)
        {
            for sounds in &setup.pron[word.as_str()] {
                for outline in setup.rules.shorten(*short, word, sounds) {
                    if outline.len() >= first.len() {
                        continue;
                    }
                    // Never an outline some word has by the rules: those stay theirs. When the
                    // form is taken, a star on it still beats a whole extra stroke.
                    let starred: Vec<Vec<Stroke>> = ladder(&outline, join)
                        .into_iter()
                        .filter(|(n, _)| *n <= 2)
                        .map(|(_, o)| o)
                        .collect();
                    let group: Vec<Vec<Stroke>> = std::iter::once(outline)
                        .chain(starred)
                        .filter(|outline| {
                            let text = render_outline(outline);
                            !simple.contains(&text)
                                && !wanted.contains(&text)
                                && !options.iter().any(|(_, g)| g.contains(outline))
                        })
                        .collect();
                    if !group.is_empty() {
                        options.push((Origin::Short(short.step), group));
                    }
                }
            }
        }
        options.push((Origin::Rules, vec![first.clone()]));
        options.extend(
            ladder(first, join)
                .into_iter()
                .map(|(n, o)| (Origin::Ladder(n), vec![o])),
        );
        for (origin, group) in options {
            // Every short form that is free is added: the dictionary has all of a word's forms,
            // and the cheapest counts. A rule or ladder outline settles the word.
            if placed && !matches!(origin, Origin::Short(_)) {
                break;
            }
            for mut outline in group {
                setup.rules.joined(&mut outline);
                let text = render_outline(&outline);
                if let Some(holder) = taken.get(&text) {
                    // Who has the outline the rules give this word (not one of its short forms)
                    if origin == Origin::Rules {
                        lost_to = Some(holder.clone());
                    }
                    continue;
                }
                taken.insert(text, (*word).clone());
                let lost_to = if matches!(origin, Origin::Ladder(_)) {
                    lost_to.clone()
                } else {
                    None
                };
                entries.push(Entry {
                    word: (*word).clone(),
                    outline,
                    origin,
                    lost_to,
                });
                if !matches!(origin, Origin::Short(_)) {
                    placed = true;
                }
                break;
            }
        }
        if !placed {
            unresolved.push((*word).clone());
        }
        // Other pronunciations are only added where they clash with nothing
        for outline in others {
            let mut outline = outline.clone();
            setup.rules.joined(&mut outline);
            let text = render_outline(&outline);
            if !taken.contains_key(&text) {
                taken.insert(text, (*word).clone());
                entries.push(Entry {
                    word: (*word).clone(),
                    outline,
                    origin: Origin::Alternate,
                    lost_to: None,
                });
            }
        }
    }

    // Backup entries: exhaustive, run to a fixed point. Every round indexes everything placed so
    // far (the primaries from the loop above in round 0, plus every earlier round's own backup
    // entries from round 1 on) and tries every split point of every word's own sound sequence
    // against three composition kinds — a trailing reduced-syllable suffix continuation (one or
    // two strokes), a leading consonant cluster folded onto a placed word's first stroke, and two
    // whole placed words back to back — adding every safe, non-colliding composed outline it
    // finds, not just the first. A word can and does end up with more than one backup entry;
    // that is the explicit goal here, not something to dedupe away.
    //
    // Chaining falls out for free from re-indexing from scratch every round: once `building` has
    // a backup entry (round 0: `build`'s own outline plus the "-ing" stroke), round 1's rebuilt
    // index includes it as an eligible stem the same as any primary entry, so `buildings`' own
    // search finds `building` and continues it with "-s" — `build`/`+-G`/`+-S`, reached without
    // `building` ever needing its own primary outline to be that shape.
    //
    // A standalone suffix's own entry (`rules/suffixes.toml`) is a continuation-stroke shape, not
    // a real word's own outline — using one as a stem here would carry its join key onto
    // whatever gets composed from it (`ed`'s own entry is the bare continuation `+-D`, the "-ed"
    // stroke struck on its own), so those are excluded from standing in as a stem entirely, every
    // round. Collisions are skipped and counted, never a hard failure, since this is a
    // best-effort pass over the whole dictionary, not an explicit human-authored override.
    //
    // Capped at a small, fixed number of rounds: growth should already have stopped well before
    // that in practice (each round can only add entries built from the *previous* round's new
    // stems, so the search space shrinks every round rather than compounding), and hitting the
    // cap is worth investigating as a sign something is matching too permissively, not a reason
    // to just raise it further.
    const MAX_BACKUP_ROUNDS: usize = 5;
    let mut backup_skipped = 0usize;
    for _round in 0..MAX_BACKUP_ROUNDS {
        let mut word_outline: HashMap<String, Vec<Stroke>> = HashMap::new();
        for entry in entries.iter().filter(|e| e.origin != Origin::Suffix) {
            let cost = (entry.outline.len(), keys(&entry.outline));
            match word_outline.get(&entry.word) {
                Some(current) if (current.len(), keys(current)) <= cost => {}
                _ => {
                    word_outline.insert(entry.word.clone(), entry.outline.clone());
                }
            }
        }
        let mut by_sounds: HashMap<&[String], Vec<&str>> = HashMap::new();
        // A stress-insensitive copy of the same index, for the compound direction (c) only: a
        // word keeps its own primary stress when struck alone (`my` is `M AY1`) but a compound's
        // own internal half often carries secondary stress instead (`myself` is `M AY2 S EH1 L
        // F`) - the same vowel, read the same way, just not the exact token an exact phoneme
        // match requires. Stripping the trailing stress digit the same way `syllables` already
        // does elsewhere lets `myself` find `my` as its own first half without also loosening the
        // suffix/prefix directions (a)/(b), which have never needed it.
        let mut by_sounds_loose: HashMap<Vec<String>, Vec<&str>> = HashMap::new();
        for word in word_outline.keys() {
            if let Some(sounds) = setup.pron.get(word.as_str()).and_then(|prons| prons.first()) {
                by_sounds.entry(sounds.as_slice()).or_default().push(word.as_str());
                by_sounds_loose.entry(stress_stripped(sounds)).or_default().push(word.as_str());
            }
        }

        let mut added_this_round = 0usize;

        for word in word_outline.keys().cloned().collect::<Vec<_>>() {
            let Some(sounds) = setup.pron.get(&word).and_then(|prons| prons.first()) else {
                continue;
            };

            // (a) stem + reduced trailing-syllable suffix continuation, one stroke (`build` +
            // "-ing" -> building; see `Rules::suffix_split`). More than one already-placed word
            // with the exact same pronunciation (`build`/`billed`): spelling breaks the tie
            // (`building` starts with `build`, not `billed`) if it can. The two-syllable suffixes
            // (`-ism`, `-ary`/`-ery`/`-ory`, `-ical`, `-logy`, `-cy`/`-sy`, `-ify`/`-fy`) are
            // handled separately, by (a2) below, not by extending this one: their own second
            // syllable's onset consonant sometimes migrates backward onto the *first* suffix
            // stroke rather than staying on the second the way plain syllabification would place
            // it (`-ism`'s own `+EUZ/+-PL` ends its first stroke in the "z" that `self.syllables`'
            // ordinary onset-assignment would otherwise give to the second syllable), so (a2) finds
            // its stem by spelling and reuses `rules/suffixes.toml`'s own, already hand-verified
            // chord as-is instead of re-deriving it through sound rules. See `docs/suffixes.md`.
            if let Some((split, bare)) = setup.rules.suffix_split(&word, sounds)
                && let Some(stems) = by_sounds.get(&sounds[..split])
                && let Some(stem) = spelled_stem(&word, stems, |w, s| w.starts_with(s))
                && stem != word
            {
                let mut composed = word_outline[stem].clone();
                composed.push(bare);
                setup.rules.joined(&mut composed);
                try_add_backup(
                    setup,
                    &mut entries,
                    &mut taken,
                    &mut backup_skipped,
                    &mut added_this_round,
                    &word,
                    sounds,
                    composed,
                );
            }

            // (a2) stem + a known two-stroke suffix (`-ism`, `-ary`/`-ery`/`-ory`, `-ical`,
            // `-logy`, `-cy`/`-sy`, `-ify`/`-fy`): the gap (a)'s own comment names above. Found by
            // spelling, not sound (see `Rules::two_stroke_suffix`'s own doc for why), so the stem
            // is looked up by its own spelling directly rather than through `by_sounds`.
            if let Some((suffix_text, suffix_strokes)) = setup.rules.two_stroke_suffix(&word) {
                let stem_spelling = &word[..word.len() - suffix_text.len()];
                if let Some(stem_outline) = word_outline.get(stem_spelling) {
                    let mut composed = stem_outline.clone();
                    composed.extend_from_slice(suffix_strokes);
                    setup.rules.joined(&mut composed);
                    try_add_backup(
                        setup,
                        &mut entries,
                        &mut taken,
                        &mut backup_skipped,
                        &mut added_this_round,
                        &word,
                        sounds,
                        composed,
                    );
                }
            }

            // (b) leading consonant cluster onto a placed word's first stroke, since lyk's join
            // key only ever continues a stroke forward, never backward: a continuation stroke can
            // follow a stem, but nothing can precede one (`ever` -> `never`, the extra sound "N").
            // Tries every leading length, not just the first that works. When the leading sounds'
            // own keys overlap a key the matched word's first stroke already uses, this direction
            // simply can't rescue the word (`unclear` needs `un-`'s own `TPH`, which shares an
            // `H` with `clear`'s own "cl" onset cluster `KHR`) — skipped, not forced; a real,
            // structural gap in the theory, not a bug in this pass.
            for split in 1..sounds.len().min(4) {
                let Some(stems) = by_sounds.get(&sounds[split..]) else {
                    continue;
                };
                let Some(stem) = spelled_stem(&word, stems, |w, s| w.ends_with(s)) else {
                    continue;
                };
                if stem == word {
                    continue;
                }
                let Some(leading) = setup.rules.prefix_keys(&word, sounds, split) else {
                    continue;
                };
                let Some(&target_first) = word_outline[stem].first() else {
                    continue;
                };
                if leading.0 & target_first.0 != 0 {
                    continue;
                }
                let mut composed = word_outline[stem].clone();
                composed[0] = Stroke(leading.0 | target_first.0);
                try_add_backup(
                    setup,
                    &mut entries,
                    &mut taken,
                    &mut backup_skipped,
                    &mut added_this_round,
                    &word,
                    sounds,
                    composed,
                );
            }

            // (c) two whole placed words back to back (`myself` -> `my` + `self`), composed as
            // the first word's own outline followed by the second's, every stroke of the second
            // marked as a join continuation exactly the way any other multi-stroke word already
            // is. Unlike (a), the second half doesn't have to be a *reduced* syllable at all —
            // it's a whole word's own outline, stressed vowel and all.
            //
            // Sound alone is too loose a net here: `myself` also sounds exactly like `mice` +
            // `elf` (a real, phonetically exact split at a different point), no relation to
            // either word's own sense and nothing a learner would ever guess — unlike
            // `doctor`/`docked`+`-er` or `crash`/`rash`, which are at least coincidences at the
            // word's own *real* boundary. Spelling is what tells these apart: only a split where
            // the word's own spelling is the literal concatenation of both halves' own spelling
            // is trusted (`myself` really is `"my"` + `"self"`, letter for letter; `"mice"` +
            // `"elf"` isn't `"myself"`'s spelling at all). Tries every split, adding every one
            // that passes — `outside` is genuinely both `"out"` + `"side"` and `"outs"` + `"ide"`,
            // and both are kept as separate entries.
            let loose = stress_stripped(sounds);
            for split in 1..loose.len() {
                let Some(firsts) = by_sounds_loose.get(&loose[..split]) else {
                    continue;
                };
                let Some(seconds) = by_sounds_loose.get(&loose[split..]) else {
                    continue;
                };
                let Some(first) = spelled_stem(&word, firsts, |w, s| w.starts_with(s)) else {
                    continue;
                };
                let Some(second) = spelled_stem(&word, seconds, |w, s| w.ends_with(s)) else {
                    continue;
                };
                if first == word || second == word || word != format!("{first}{second}") {
                    continue;
                }
                let mut composed = word_outline[first].clone();
                composed.extend(word_outline[second].iter().copied());
                setup.rules.joined(&mut composed);
                try_add_backup(
                    setup,
                    &mut entries,
                    &mut taken,
                    &mut backup_skipped,
                    &mut added_this_round,
                    &word,
                    sounds,
                    composed,
                );
            }
        }

        if added_this_round == 0 {
            break;
        }
    }

    // A word is stranded when its own plain-rules outline would cost less than whatever it
    // actually settled for, and that cheaper outline's text is sitting completely unclaimed - the
    // only way that happens today is an automatic brief that Plover or Lapwing mark with a star
    // or an extra key for their own, unrelated reasons (see the brief loop above, which now
    // checks for exactly this before pinning a word to one); kept here as a standing regression
    // guard for any other way this could happen, not just that one. Checked once, against the
    // fully-settled dictionary (after briefs, rules, the ladder, and every backup round), so it
    // only ever reports a genuine, still-available opportunity, never a transient one that got
    // claimed later by the word itself through some other path.
    let stranded: Vec<String> = {
        let best = cheapest(&entries);
        let mut found = Vec::new();
        for (word, outline) in &best {
            let Some(sounds) = setup.pron.get(*word).and_then(|p| p.first()) else { continue };
            let Some(mut natural_outline) = setup.rules.outline(word, sounds) else { continue };
            setup.rules.joined(&mut natural_outline);
            let text = render_outline(&natural_outline);
            let cur_cost = (outline.len(), keys(outline));
            let nat_cost = (natural_outline.len(), keys(&natural_outline));
            if nat_cost < cur_cost && !taken.contains_key(&text) {
                found.push((*word).to_owned());
            }
        }
        found.sort();
        found
    };

    Ok(Generated {
        entries,
        unresolved,
        backup_skipped,
        stranded,
    })
}

/// Each word's cheapest outline (fewest strokes, then fewest keys) among a list of entries.
///
/// A standalone suffix's own entry (`rules/suffixes.toml`) never counts, even when its own
/// display text happens to also be a real word (`less`, `age`, `some`, `ism`, and 17 others): its
/// outline is a continuation-stroke fragment that is never a word by itself when struck alone
/// (the join key it carries refuses that, by design — see [the join key](../docs/join-key.md)),
/// so it is never really "the outline" for that word even when it numerically looks cheapest.
pub fn cheapest(entries: &[Entry]) -> HashMap<&str, &[Stroke]> {
    let mut best: HashMap<&str, &[Stroke]> = HashMap::new();
    for entry in entries.iter().filter(|e| e.origin != Origin::Suffix) {
        let cost = |o: &[Stroke]| (o.len(), keys(o));
        match best.get(entry.word.as_str()) {
            Some(current) if cost(current) <= cost(&entry.outline) => {}
            _ => {
                best.insert(&entry.word, &entry.outline);
            }
        }
    }
    best
}

/// Every outline in a generated dictionary, by its rendered text, to the word it writes.
pub fn index(generated: &Generated) -> HashMap<String, &str> {
    generated
        .entries
        .iter()
        .map(|e| (render_outline(&e.outline), e.word.as_str()))
        .collect()
}

/// Every single-stroke word in the dictionary, by its own bare stroke — what
/// `Rules::resolve`/the deployed plugin check before reading an unmatched run's first stroke as
/// raw phonemes, so a continuation after a known word (`SKP/+-R`, "and" plus "-er") reads as that
/// word's own real text ("ander") instead of garbling the word itself ("sper", "and"'s own `SKP`
/// read as if it meant nothing). A standalone suffix's own entry is excluded the same way
/// [`cheapest`] excludes it everywhere else — its outline is a continuation-stroke fragment, never
/// a real word struck alone. Collision-free by construction: `taken` already guarantees one word
/// per outline text, so there is never more than one candidate for a given stroke here.
pub fn single_stroke_stems(generated: &Generated) -> HashMap<Stroke, String> {
    generated
        .entries
        .iter()
        .filter(|e| e.origin != Origin::Suffix)
        .filter_map(|e| match e.outline[..] {
            [stroke] => Some((stroke, e.word.clone())),
            _ => None,
        })
        .collect()
}

/// The dictionary as Plover JSON.
pub fn write_json(generated: &Generated, path: &Path) -> Result<()> {
    let mut map: BTreeMap<String, &str> = BTreeMap::new();
    for entry in &generated.entries {
        map.insert(render_outline(&entry.outline), entry.word.as_str());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(&map)?)
        .with_context(|| path.display().to_string())?;
    Ok(())
}

/// Build the dictionary, write it, and say what it costs to type next to Plover and Lapwing.
pub fn build(
    setup: &Setup,
    briefs: &Briefs,
    common: usize,
    out: &Path,
    against: Option<&(String, HashMap<String, String>)>,
) -> Result<()> {
    let generated = generate(setup, briefs, common)?;
    write_json(&generated, out)?;

    let mut by_origin: BTreeMap<String, usize> = BTreeMap::new();
    for entry in &generated.entries {
        *by_origin.entry(format!("{:?}", entry.origin)).or_default() += 1;
    }
    println!(
        "wrote {} entries to {}",
        generated.entries.len(),
        out.display()
    );
    for (origin, n) in &by_origin {
        println!("  {origin:<12} {n:>7}");
    }
    println!(
        "  words the rules could not place: {}",
        generated.unresolved.len()
    );
    println!(
        "  backup entries skipped as collisions: {}",
        generated.backup_skipped
    );
    println!(
        "  words stranded on a worse outline than their own, unclaimed plain reading: {}",
        generated.stranded.len()
    );
    if !generated.stranded.is_empty() {
        println!("    {}", generated.stranded.join(", "));
    }
    let sample: Vec<String> = generated
        .unresolved
        .iter()
        .take(40)
        .map(|w| format!("{w} ({})", setup.pron[w.as_str()][0].join(" ")))
        .collect();
    println!("    the commonest: {}", sample.join(", "));

    // Each word's cheapest outline here, and what Plover and Lapwing spend on the same word
    let ours: HashMap<&str, (usize, u32)> = cheapest(&generated.entries)
        .into_iter()
        .map(|(word, outline)| (word, (outline.len(), keys(outline))))
        .collect();
    let cost_of = |outlines: Option<&Vec<String>>| {
        let outline = shortest(outlines?)?;
        Some((outline.len(), keys(&outline)))
    };
    // Another dictionary to compare with, by its own shortest outline for each word
    let other_cost = |word: &str| -> Option<(usize, u32)> {
        let outline = parse_outline(against?.1.get(word)?)?;
        Some((outline.len(), keys(&outline)))
    };
    let mut total = [0f64; 9];
    // The same, by how common the word is: where the keys go
    const BANDS: [(usize, &str); 5] = [
        (100, "top 100"),
        (1_000, "100-1k"),
        (5_000, "1k-5k"),
        (20_000, "5k-20k"),
        (usize::MAX, "20k+"),
    ];
    let mut bands = [[0f64; 5]; BANDS.len()];
    let mut worse: Vec<(f64, &str, u32, u32)> = Vec::new();
    for (word, &(strokes, key_count)) in &ours {
        let (Some(plover), Some(lapwing)) = (
            cost_of(setup.gold.plover.get(*word)),
            cost_of(setup.gold.lapwing.get(*word)),
        ) else {
            continue;
        };
        let other = other_cost(word);
        if against.is_some() && other.is_none() {
            continue;
        }
        let other = other.unwrap_or_default();
        let weight = setup.count.get(*word).copied().unwrap_or(0) as f64;
        total[0] += weight;
        total[1] += weight * strokes as f64;
        total[2] += weight * f64::from(key_count);
        total[3] += weight * plover.0 as f64;
        total[4] += weight * f64::from(plover.1);
        total[5] += weight * lapwing.0 as f64;
        total[6] += weight * f64::from(lapwing.1);
        total[7] += weight * other.0 as f64;
        total[8] += weight * f64::from(other.1);
        let rank = setup.rank.get(*word).copied().unwrap_or(usize::MAX);
        let band = BANDS.iter().position(|(to, _)| rank < *to).unwrap_or(0);
        bands[band][0] += weight;
        bands[band][1] += weight * f64::from(key_count);
        bands[band][2] += weight * f64::from(plover.1);
        bands[band][3] += weight * f64::from(lapwing.1);
        bands[band][4] += weight * f64::from(other.1);
        if key_count > plover.1 {
            worse.push((
                weight * f64::from(key_count - plover.1),
                word,
                key_count,
                plover.1,
            ));
        }
    }
    let other_name = against.map_or("", |(name, _)| name.as_str());
    println!("\nwhat a word costs on average, weighted by how often it is said");
    println!("(words all the dictionaries write; strokes and keys pressed per word)");
    println!("            strokes     keys");
    let mut rows = vec![
        ("this", total[1], total[2]),
        ("plover", total[3], total[4]),
        ("lapwing", total[5], total[6]),
    ];
    if against.is_some() {
        rows.push((other_name, total[7], total[8]));
    }
    for (name, strokes, key_count) in rows {
        println!(
            "  {name:<9} {:>7.3} {:>8.3}",
            strokes / total[0],
            key_count / total[0]
        );
    }
    println!("\nkeys per word by how common the word is, and the share of all words said");
    println!("            share     this   plover  lapwing  {other_name}");
    for ((_, name), band) in BANDS.iter().zip(&bands) {
        if band[0] == 0.0 {
            continue;
        }
        let other = if against.is_some() {
            format!("  {:>7.3}", band[4] / band[0])
        } else {
            String::new()
        };
        println!(
            "  {name:<9} {:>5.1}%  {:>7.3}  {:>7.3}  {:>7.3}{other}",
            100.0 * band[0] / total[0],
            band[1] / band[0],
            band[2] / band[0],
            band[3] / band[0]
        );
    }
    println!("\nthe commonest words that lost their outline to a word that sounds the same");
    let mut lost: Vec<&Entry> = generated
        .entries
        .iter()
        .filter(|e| e.lost_to.is_some() && matches!(e.origin, Origin::Ladder(_)))
        .collect();
    lost.sort_by_key(|e| setup.rank.get(&e.word).copied().unwrap_or(usize::MAX));
    for entry in lost.iter().take(30) {
        let holder = entry.lost_to.as_deref().unwrap_or("");
        let theirs = generated
            .entries
            .iter()
            .find(|e| e.word == holder)
            .map_or_else(String::new, |e| render_outline(&e.outline));
        println!(
            "  {:<12} {:<18} (the outline of {holder}: {theirs})",
            entry.word,
            render_outline(&entry.outline)
        );
    }
    worse.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!("\nthe words that cost the most extra keys over plover");
    for (_, word, ours_keys, plover_keys) in worse.iter().take(40) {
        let outline = generated
            .entries
            .iter()
            .filter(|e| e.word == *word)
            .map(|e| render_outline(&e.outline))
            .collect::<Vec<_>>()
            .join(" ");
        let plover = setup
            .gold
            .plover
            .get(*word)
            .and_then(|o| o.first())
            .map_or("", String::as_str);
        println!(
            "  {word:<12} {ours_keys} vs {plover_keys} keys   ours {outline:<24} plover {plover}"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stroke::parse_outline;

    fn entry(word: &str, outline: &str, origin: Origin) -> Entry {
        Entry {
            word: word.to_owned(),
            outline: parse_outline(outline).unwrap(),
            origin,
            lost_to: None,
        }
    }

    use std::sync::LazyLock;

    /// The same `simple`/`wanted` sets `generate` builds from the real, full dictionary, shared
    /// across every test below instead of recomputed per word (the groups pass alone touches every
    /// word in the dictionary).
    static GROUPS: LazyLock<(HashSet<String>, HashSet<String>)> = LazyLock::new(|| {
        let setup = crate::dictionary_fixture::setup();
        let words = ranked_words(setup, 100_000);
        group_outline_sets(setup, &words)
    });

    fn candidates_for(word: &str, briefs: &Briefs) -> Vec<Candidate> {
        let setup = crate::dictionary_fixture::setup();
        let (simple, wanted) = &*GROUPS;
        let simple: HashSet<&str> = simple.iter().map(String::as_str).collect();
        let wanted: HashSet<&str> = wanted.iter().map(String::as_str).collect();
        enumerate_candidates(setup, briefs, word, &simple, &wanted)
    }

    /// The real, built-in briefs, minus the named `[word]` overrides - simulates the dictionary's
    /// state *after* the acceptance-gate override removals this plan's verification calls for, so
    /// a test can check what the new pipeline does unprompted, without needing those overrides
    /// gone from `rules/briefs.toml` yet.
    fn briefs_without(words: &[&str]) -> Briefs {
        let mut value: toml::Value = toml::from_str(BUILT_IN_BRIEFS).expect("built-in briefs parse");
        if let Some(table) = value.get_mut("word").and_then(toml::Value::as_table_mut) {
            for word in words {
                table.remove(*word);
            }
        }
        value.try_into().expect("briefs minus a few words still deserializes")
    }

    fn cheapest_candidate(candidates: &[Candidate]) -> &Candidate {
        candidates.iter().min_by_key(|c| c.cost).expect("at least one candidate")
    }

    #[test]
    fn a_word_named_in_briefs_toml_as_a_literal_outline_gets_exactly_that_one_candidate() {
        // `make` is pinned today (`rules/briefs.toml`) to its own correct phonetic reading - a
        // human decision this function must never reopen to competition against its own word's
        // other candidate sources.
        let briefs = Briefs::built_in().unwrap();
        let candidates = candidates_for("make", &briefs);
        assert_eq!(candidates.len(), 1, "a literal override must force exactly one candidate");
        assert_eq!(render_outline(&candidates[0].outline), "PHAEUBG");
        assert_eq!(candidates[0].origin, Origin::Brief);
    }

    #[test]
    fn candidate_cost_orders_by_strokes_then_keys_then_ergonomics() {
        let one_stroke_few_keys = parse_outline("TK").unwrap();
        let one_stroke_more_keys = parse_outline("STKPW").unwrap();
        let two_strokes = parse_outline("TK/TK").unwrap();
        assert!(candidate_cost(&one_stroke_few_keys) < candidate_cost(&one_stroke_more_keys));
        assert!(candidate_cost(&one_stroke_more_keys) < candidate_cost(&two_strokes));
    }

    #[test]
    fn the_take_takes_inconsistency_is_gone_from_the_candidate_pool() {
        // `take` happens to have a short brief Plover lists directly (`TAEBG`); `takes` does not
        // (Plover's own shortest listed candidate is the longer `TAEUBGS`) - today's `best_brief`
        // only ever sees each word's own gold-listed candidates, so `takes` never got a chance to
        // compete against lyk's own shorter spelling-variant reading, `TAEBGS`, the way `take`'s
        // own `TAEBG` already matches. Both are now in the same pool for both words.
        let briefs = Briefs::built_in().unwrap();
        let take = candidates_for("take", &briefs);
        let takes = candidates_for("takes", &briefs);
        assert!(
            take.iter().any(|c| render_outline(&c.outline) == "TAEBG"),
            "take's candidates: {:?}",
            take.iter().map(|c| render_outline(&c.outline)).collect::<Vec<_>>()
        );
        assert!(
            takes.iter().any(|c| render_outline(&c.outline) == "TAEBGS"),
            "takes's candidates: {:?}",
            takes.iter().map(|c| render_outline(&c.outline)).collect::<Vec<_>>()
        );
        assert_eq!(render_outline(&cheapest_candidate(&take).outline), "TAEBG");
        assert_eq!(render_outline(&cheapest_candidate(&takes).outline), "TAEBGS");
    }

    #[test]
    #[ignore = "diagnostic: run with --ignored --nocapture to inspect the natural candidate pool"]
    fn dump_natural_candidates_for_the_override_words() {
        let briefs = briefs_without(&[
            "make", "but", "on", "from", "no", "then", "day", "which", "wow", "honest", "bottle",
            "public", "hiding", "gasps", "flowers", "grunting", "honey", "bloody",
        ]);
        for word in [
            "make", "but", "on", "from", "no", "then", "day", "which", "wow", "honest", "bottle",
            "public", "hiding", "gasps", "flowers", "grunting", "honey", "bloody",
        ] {
            let candidates = candidates_for(word, &briefs);
            let mut rows: Vec<(String, Origin, u32)> = candidates
                .iter()
                .map(|c| (render_outline(&c.outline), c.origin, c.cost))
                .collect();
            rows.sort_by_key(|(_, _, cost)| *cost);
            println!("{word}:");
            for (text, origin, cost) in rows {
                println!("  {text:<16} {origin:?} cost={cost}");
            }
        }
    }

    /// The exact `[word]` overrides the acceptance gate (`docs/decisions.md`) removes once this
    /// pipeline is proven to reproduce or beat them, next to the outline each one forces today.
    const OVERRIDDEN_WORDS: &[(&str, &str)] = &[
        ("make", "PHAEUBG"),
        ("but", "PWUT"),
        ("on", "OB"),
        ("from", "PR"),
        ("no", "PHO"),
        ("then", "THEB"),
        ("day", "TAEU"),
        ("which", "KH"),
        ("wow", "WOU"),
        ("honest", "OPBS"),
        ("bottle", "PWOLT"),
        ("public", "PUBLG"),
        ("hiding", "HAOEUGD"),
        ("gasps", "TKPWAFPS"),
        ("flowers", "TPHROURZ"),
        ("grunting", "TKPWRUPBGT"),
        ("honey", "HOPB/EU"),
        ("bloody", "PWHROD/EU"),
    ];

    #[test]
    fn the_natural_pool_never_costs_more_than_the_override_it_would_replace() {
        // Verification step 1 of the plan this implements: before the real solver ever runs,
        // each of these words' own natural candidate list (brief, rules, spelling, brevity,
        // ladder - the override itself excluded) must have a cheapest member that costs no more
        // than the override it is meant to replace. This is the half of "reproduces or beats"
        // that is checkable per word, in isolation, before frequency-weighted arbitration against
        // the rest of the dictionary decides which word actually keeps a contested outline - see
        // `the_fragment_check_words_all_find_something_cheaper_not_worse` below for what these
        // words' own cheapest candidate actually turns out to be, and why.
        let words: Vec<&str> = OVERRIDDEN_WORDS.iter().map(|(w, _)| *w).collect();
        let briefs = briefs_without(&words);
        for &(word, overridden) in OVERRIDDEN_WORDS {
            let mut forced = parse_outline(overridden).expect("every override text parses");
            // Multi-stroke overrides (`honey`, `bloody`) are written `"HOPB/EU"` - their own
            // outline needs the same join-key marking `enumerate_candidates`'s own `push` applies
            // to every candidate, for a fair cost comparison.
            setup_for_join(&mut forced);
            let overridden_cost = candidate_cost(&forced);
            let candidates = candidates_for(word, &briefs);
            let cheapest = cheapest_candidate(&candidates);
            assert!(
                cheapest.cost <= overridden_cost,
                "{word}: natural pool's cheapest ({}, cost {}) is worse than the override \
                 {overridden:?} (cost {overridden_cost}) - a real bug in enumeration or the cost \
                 function, not an acceptable tradeoff",
                render_outline(&cheapest.outline),
                cheapest.cost
            );
        }
    }

    fn setup_for_join(outline: &mut [Stroke]) {
        let setup = crate::dictionary_fixture::setup();
        setup.rules.joined(outline);
    }

    #[test]
    fn six_of_the_fragment_check_overrides_reproduce_exactly_in_isolation() {
        // Unlike `make`/`but`/`on`/`from`/`no`/`then`/`day`/`which` (see this module's own report
        // for why those land on something cheaper instead - the blanket `matches_real_consonants`
        // filter in `enumerate_candidates` finds an even deeper, legal fragment-drop than the
        // human override itself was), these six stranded-word fixes are already the natural
        // pool's own cheapest candidate, confirmed here in isolation before the real solver (with
        // every other word also contesting the dictionary) ever runs.
        let exact: &[(&str, &str)] = &[
            ("wow", "WOU"),
            ("honest", "OPBS"),
            ("bottle", "PWOLT"),
            ("public", "PUBLG"),
            ("hiding", "HAOEUGD"),
            ("flowers", "TPHROURZ"),
        ];
        let words: Vec<&str> = exact.iter().map(|(w, _)| *w).collect();
        let briefs = briefs_without(&words);
        for &(word, want) in exact {
            let candidates = candidates_for(word, &briefs);
            let cheapest = cheapest_candidate(&candidates);
            assert_eq!(
                render_outline(&cheapest.outline),
                want,
                "{word}: expected the override's own outline to already be the natural pool's \
                 cheapest"
            );
        }
    }

    #[test]
    fn a_standalone_suffixs_own_entry_never_wins_cheapest_even_when_shorter() {
        // "less" is both a real word (HRES, 4 keys) and the display text of the standalone
        // "-less" suffix's own continuation stroke (+-LS, 2 keys) - a stroke that is never a
        // word by itself when struck alone (see the join key), so it must never be picked as
        // "the" outline for the real word just because it numerically looks cheaper.
        let entries = vec![
            entry("less", "+-LS", Origin::Suffix),
            entry("less", "HRES", Origin::Rules),
        ];
        let best = cheapest(&entries);
        assert_eq!(best.get("less").copied(), Some(&parse_outline("HRES").unwrap()[..]));
    }
}
