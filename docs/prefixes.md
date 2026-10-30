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

**A related but separate mechanism** adds a *backup* entry for a word whose leading sound sits in
front of some other already-placed word's own *exact* pronunciation, whether or not that leading
sound is one of the fixed prefixes above (`never` is the extra sound "N" in front of *ever*'s own
sounds — no `un-`/`dis-`/etc. involved at all). See **[syllables & folding](syllables.md)** for the
rule. It can only ever land on the *same* stroke as the matched word's own first one, the same way
a fixed prefix above folds onto the next stroke's left hand — lyk's join key only ever continues a
stroke forward, so nothing can be struck *before* another word's own stroke the way a suffix
continues *after* one. That's a real, structural gap for a word like `unclear`: `un-`'s own fixed
`TPH` needs the same `H` that `clear`'s own "cl" onset cluster `KHR` already uses, so there's no
stroke this mechanism (or the fixed-prefix one above) can rescue it with — `unclear` keeps whatever
its own independent derivation gives it, inconsistency and all, until this theory has some way to
strike a prefix *before* a stem's own stroke, which it doesn't yet.

Next: **[syllables & folding](syllables.md)** — how the rest of a word, beyond its first stroke's
onset/vowel/coda, gets written.
