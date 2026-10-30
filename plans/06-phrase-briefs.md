# Plan: a small, lyk-native phrase-brief system

Scoping doc only — **nothing here gets built until this is reviewed.** Repo:
`/home/fib/code/noahlyk/lyk-steno-theory`.

## Why

Everything in the theory so far optimizes one axis: keys per *word*. Phrase-briefs — one stroke
for a whole pronoun+verb fragment ("I am", "there is", "I don't") — attack a different, almost
untouched axis: keys per *word boundary*. A handful of pronoun+verb bigrams dominate real typing
the same way a handful of single words do, so this is plausibly higher-leverage than any further
per-word tuning at this point. lyk's own join key also removes the main reason traditional
theories have to be cautious here (word-boundary ambiguity with a continuing stroke), so there's
a real argument lyk can do this more cleanly than the theory it's inspired by.

## What this explicitly is NOT

Not a port of Plover/Lapwing's "Jeff's phrasing." That system is real but enormous — I generated
its actual dictionary to get honest numbers: **173,780 entries**, more than double lyk's entire
word dictionary. It also isn't portable as data at all: it decodes *Lapwing's own* raw key
positions to figure out subject/tense/verb, so every one of its ~100 verb endings is literally
"that verb's coda cluster in Lapwing's theory," which has nothing to do with lyk's own outlines
for the same verbs. Porting the idea means designing lyk's own version from scratch, not
translating a file.

Most of those 173,780 entries are also long chords (8-14 keys pressed in one stroke — the length
distribution peaks there), which real single words almost never produce. The real collision risk
with lyk's existing 77k-word dictionary concentrates in the ~10,000 *short* entries (≤7 keys).
That's the part worth being careful about; the long tail isn't where the danger is, but it's also
not where most of the value is per entry spent, which is the other reason to not just port it all.

## Scope for a v1, not the full system

- **Subjects**: start with the highest-frequency ones only — `I`, `you`, `he`, `she`, `it`, `we`,
  `they`, `there`. Drop `this`/`that` as phrase subjects for v1 (Jeff's own `THERE_SUFFIXES` extra
  layer for "there" is also a v1 cut — just the plain forms).
- **Verbs**: a shortlist of the ~20-30 most common/irregular ones worth a phrase ending, not
  Jeff's ~100 — think `be`, `have`, `do`, `go`, `know`, `think`, `want`, `need`, `get`, `make`,
  `say`, `see`, `come`, `take`, `look`, `use`, `find`, `give`, `tell`, `ask`. Exact list: rank by
  frequency × how irregular the conjugation is (regular verbs barely benefit, since their own
  plain outline is already cheap).
- **Tense**: present and past only. Drop present/past-participle forms, drop modal auxiliaries
  (`can`/`could`/`will`/`would`/`shall`/`should` — Jeff's `MIDDLES` table) and drop contracted
  negatives for v1. Both are easy to add later once the base shape is proven.
- **The ending rule, and this is the part that matters most**: a verb's phrase-ending is always
  *that verb's own existing final coda cluster*, read straight out of the generated dictionary
  (`src/generate.rs` already computes this for every word), plus a small, fixed set of
  grammatical additions applied the same way to every verb. Never a hand-picked ending per verb.
  This is what keeps it a rule, not a second hand-curated exception table living next to a theory
  that has otherwise earned the right to call itself systematic.

## The actual engineering steps, in order

1. Audit `rules/plover.toml`'s key layout for a stroke-slot design: which key group carries the
   subject, which carries the verb-ending, whether there's room for a present/past flag at all
   without colliding with how ordinary words already use those same keys.
2. Pull each shortlisted verb's coda cluster mechanically from the already-generated dictionary —
   no manual lookup, this should be a script/tool pass over `out/lyk.json`.
3. Generate every candidate phrase stroke (subject × verb × tense) and run the same collision
   check this session already used for everything else (`cargo run --release -- resolve`, and
   `build`'s own collision reporting) against the full word dictionary. This is the step that
   actually decides whether the design holds up — expect to revise the stroke-slot design in step
   1 if it doesn't.
4. Decide dictionary vs. WASM **based on the result of step 3**, not in advance: if any candidate
   needs to beat out a real word on purpose, it has to be a loaded-above-the-base dictionary file
   (WASM fallback plugins can only fill gaps, never override). If nothing needs to override
   anything, WASM is the better fit and matches how the orthographic fallback already works here
   (generated from the rules, not hand-maintained as a giant table).
5. Build, verify with the same rigor as every other change (`cargo build/test --release`, `score
   --check-readme`, `clippy`), and write a new docs chapter for it — exhaustively, since this is a
   genuinely different kind of rule (grammar, not phonetics) and deserves its own clear chapter
   rather than being folded into the phonetic rules as if it were one of them.

## Sequencing

Land this **after** the proper-nouns work and the two still-open decisions ("both"'s onset-drop
tradeoff, the KPA-family scheme) settle. Every suffix or brief added later reshuffles which
strokes are free, and step 3 above needs a stable dictionary to audit collisions against — doing
this in parallel with other word-level churn means redoing the collision pass more than once.

## Deliverable of this doc

Nothing but this file. No code, no data, no new dictionary entries until the scope above is
reviewed and the stroke-slot design in step 1 is sanity-checked against real key availability.
