# How short

[← back to the table of contents](README.md) · [← back to the main README](../README.md)

**How short** (`[[brevity]]` in the rules). Each step is a transformation of the full outline
that can be done by hand, with a frequency rank below which words take it. `lead` drops an
unstressed first syllable that is only a vowel (*about* PWOUT, *again* TKPWEPB, *around*
ROUPBD); `lean` puts the consonants of an unstressed first syllable on the left hand of the
next stroke when they come first in steno order (*believe* PWHRAOEF, *deliver* TKHREUFR,
*complete* KPHRAOET). `fold` is the outline with reduced syllables folded out of steno order
where they would not fit in it, as Plover does (*student* STAOUPBTD, *goodness*
TKPWAOPBSD). The one-stroke steps for long words: `skeleton` is one
stroke of the first consonants and every consonant after the first vowel, `shell` keeps those
of the stressed and last syllables, `ends` those of the last; each with or without the
stressed vowel. An empty list gives a Lapwing-like dictionary, a stroke per syllable; a huge
rank on every step gives the shortest one the rules allow. Words that take no step are written
by the plain **[prefix](prefixes.md)**/**[syllable](syllables.md)** rules. An unstressed first
syllable gives a one-stroke form its left hand (*information* TPH-FGS, *condition* K-PB,
*opinion* P*PB), and the glide of *-ion* is not written. A short form never takes an outline that
any word has by the rules, so the forms a learner can work out stay theirs; when a short form is
taken, a star on it is tried before giving up, since a star costs a key and a longer form costs a
stroke. Every word keeps every short form still free beside its own rule outline once its primary
is settled, so the dictionary holds both ends of the spectrum and the cheapest one counts toward
`cheapest` regardless — this now runs as its own pass strictly after every word's primary outline is
final, which closes a real ordering hazard the old, interleaved version had: a common word's bonus
short form could squat on text a later, rarer word needed as its own *primary*, simply because the
common word was processed first. A word pinned to a literal `[word]` override (see
**[briefs](briefs.md)**) is the one exception — its single entry is a human decision, not reopened
by a bonus short form any more than by anything else.

`build` prints how many strokes and keys a word takes on average, weighted by how often it is
said, next to Plover and Lapwing on the same words — see
**[the main README](../README.md#where-lyk-stands)** for the numbers.

Next: **[the full suffix table](suffixes.md)** — every standalone continuation stroke this gives a
suffix on its own, derived from the same rules as any other syllable.
