# Prefixes with their own keys

[← back to the table of contents](README.md) · [← back to the main README](../README.md)

**Prefixes with keys of their own.** `ex-` is `KP` and `dis-` is `STK`; the keys join the
next stroke's left hand: *expect* KPEBGT, *distance* STKUPBS. The same fixed-prefix pattern also
covers `un-`, `mis-`, `over-`, `under-`, and `inter-` (`rules/plover.toml`'s `[[prefix]]` table).
The first `[[prefix]]` whose spelling and sounds both match wins, so a prefix that is itself the
start of a longer one (`un-` inside `under-`) is listed so the longer one is checked first. `re-`
and `pre-` are deliberately left out of this table: they already have their own carve-out in
`[[spelled]]` (`IH.e`/`IY.e`) that keeps their reduced vowel, which a fixed prefix chord would
silently override. `non-` is left out too, for a different reason: its sound is a vowel between
two copies of the same consonant (`N AA N`), so there is no second onset chord to combine with
`un-`'s own the way `under-`/`inter-` combine two different ones.

Next: **[syllables & folding](syllables.md)** — how the rest of a word, beyond its first stroke's
onset/vowel/coda, gets written.
