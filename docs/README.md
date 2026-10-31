# lyk documentation — read in this order

[← back to the main README](../README.md) for the pitch, the CLI commands, and the data sources.

This isn't a lookup table — it's a sequential read. Each chapter builds on the one before it, and
each one ends with a link to the next. Start at the top and read straight through if you want to
learn the whole theory; come back here to jump to a specific chapter once you know your way around.

**One rule underlies everything below: a word gets exactly one single-stroke outline, never
more.** "Only one way to type it" means that, and only that — there is never a choice between
`NOT`, `OT`, and `PHOT` for *not*, the way there sometimes is in a dictionary built by hand.
Briefs, the star ladder, and spelling disambiguation (chapters 3, 5, and 8) exist specifically to
hold that invariant even when two words would otherwise fight over the same stroke. **Multi-stroke
outlines built on the join key are a different thing entirely, and unlimited**: a word can have as
many `+`-joined backup outlines as the generator can honestly derive, on top of its one
single-stroke way in. What matters for a multi-stroke outline isn't brevity — it's predictability:
whether it breaks down into pieces a learner already knows (an already-placed whole word, or one
of the hardcoded suffix/prefix patterns in chapters 7 and 10) rather than its own one-off,
memorized shorthand. See **[syllables & folding](syllables.md)** for where that shows up as actual
generated entries.

1. **[The join key](join-key.md)** — why word boundaries are explicit, and what the `+` key buys
   every chapter after this one.
2. **[Reserved strokes](reserved-strokes.md)** — commands, punctuation, and fingerspelling are
   pinned before any word is, ahead of even briefs. (The actual content of those strokes is
   chapters 10 and 11, once the word-generation rules below are out of the way.)
3. **[Briefs](briefs.md)** — the commonest words keep the outlines people already know, pinned
   before anything generated.
4. **[Phonetic rules](phonetic-rules.md)** — onset, vowel, coda: how a plain syllable gets its
   stroke when it has no brief.
5. **[Spelling disambiguation](spelling.md)** — picking between more than one way to write the
   same sound.
6. **[Prefixes with their own keys](prefixes.md)** — `ex-`, `dis-`, `un-`, `mis-`, `over-`,
   `under-`, `inter-`.
7. **[Syllables & folding](syllables.md)** — how an unstressed syllable joins the stroke before
   it, or becomes a continuation stroke of its own.
8. **[When two words sound the same](collisions.md)** — briefs, then spelling, then the star
   ladder, in that order.
9. **[How short](brevity.md)** — the brevity steps that shrink a long word's outline, by hand or
   by rule.
10. **[The full suffix table](suffixes.md)** — every standalone continuation stroke a suffix gets
    on its own (`+-G` "ing", `+-D` "ed", and the rest), the one rule they're all derived from, and
    which ones aren't covered yet.
11. **[Punctuation & commands](punctuation.md)** — the actual content of the punctuation and
    command strokes reserved back in chapter 2.
12. **[Fingerspelling & misc keys](fingerspelling.md)** ("Emily's") — the alphabet, numbers,
    function keys, symbols, and OS-modifier chording, likewise reserved back in chapter 2.
13. **[The fallback](fallback.md)** — what happens when a stroke misses every dictionary above
    outright, and how that same logic becomes keymux's WASM fallback plugin.
14. **[Spelling mode](spelling-mode.md)** — a deliberate, letter-exact way to type a word with no
    pronunciation to generate from at all, reusing the join key's own multi-stroke mechanism.

The benchmarks come last, as a measured result of everything above rather than a rule of their
own — and they already live on [the main README](../README.md#where-lyk-stands), not here: the one
overall number up top, and the full breakdown (every table, auto-checked against
`score --check-readme`) at [its bottom](../README.md#full-benchmarks).

**[The roadmap](roadmap.md)** is not part of this reading order — it's a working plan, not theory.

**[Decisions](decisions.md)** is also not part of this reading order — a maintainer's log of why
real changes landed the way they did, so none of them get silently reverted by accident.

If you only have time for one chapter: start at 1 and keep going: it's written to be read straight
through, not sampled.
