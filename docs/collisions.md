# When two words sound the same

[← back to the table of contents](README.md) · [← back to the main README](../README.md)

This is the machinery behind [the one rule stated up front](README.md): a word gets exactly one
single-stroke outline, never a choice between several. Everything here only ever settles *which*
word keeps a given stroke when two or more collide — it never multiplies outlines for a single word;
see **[syllables & folding](syllables.md)** for the separate, unlimited multi-stroke backups
instead.

**Every word's own collisions are settled together, by one global optimization, not a sequence of
passes run in a fixed order.** Briefs, spelling, and the star ladder used to be three separate steps
— first every brief was pinned, then spelling broke ties among whatever was left, then the star
ladder mopped up what still collided — each one settling its own words commonest-first and never
revisiting a decision once made. That meant an early, common word's outline could never be reopened
even when a later, rarer word's own circumstances made a different global arrangement cheaper
overall — see [`docs/briefs.md`](briefs.md) and [`docs/decisions.md`](decisions.md) for the
confirmed bugs that shape produced. Now every word's full set of legal candidates — a gold brief,
lyk's own plain rules reading, the spelling-told-apart variant for its sound-alike group, every
brevity-shortened form, and the full star ladder on top of all of that — go into one shared pool,
and a single solve picks the assignment that minimizes the whole dictionary's total cost at once,
weighted by how often each word is actually said.

**Spelling's own job is unchanged in substance, just reframed**: it still answers "who may even
propose this word's own plain, unstressed-vowel reading," not "who should win it." `sail` (`ai`) and
`sale` (silent e) sound identical but spell the stressed vowel differently; the rules read that
difference back as two distinct outlines, `SAEUL` for the `ai` spelling and the shorter `SAEL` for
the silent-e one — see **[spelling disambiguation](spelling.md)** for how a sound becomes a named,
spelling-conditioned variant in the first place. Both `sail`'s and `sale`'s own readings are offered
as candidates for their respective words; which one actually lands on the longer, "default" form and
which on the shorter, spelling-told-apart variant is no longer decided by a dedicated pre-pass
ahead of frequency — it falls out of the same cost-and-frequency solve as every other contested
text, which in practice still tends to hand the commoner member of a pair its cheaper pick, since
that is usually (not axiomatically) what minimizes the total.

**The star ladder is the fallback every word always carries, not a separate pass that only runs
once a word has already lost.** A word's own four star-ladder steps (a star on the first stroke, a
star on the last, then one and two extra star-marker strokes) are always present in its candidate
pool alongside everything else, so a word that loses a contested outline to a more valuable word
falls through to its own next-cheapest option automatically, as part of the same solve — not a
retry loop invoked afterward. A short form never takes an outline that a one-syllable word's own
rules produce, the same eligibility gate a brevity candidate has always had.

Next: **[how short](brevity.md)** — how a long word's outline gets shorter.
