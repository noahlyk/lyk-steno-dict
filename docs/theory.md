# How lyk writes a word

[← back to the table of contents](README.md) · [← back to the main README](../README.md)

1. **Reserved strokes first of all** (`rules/reserved/`), ahead of even briefs: every command,
   punctuation mark, and fingerspelled letter in a real keymux config for lyk (copies of the
   deployed `01-lyk-commands.json`, `03-lyk-punctuation.json`, `04-lyk-emilys-modifiers.json`) is
   pinned before any word is, so a word can never silently take over a stroke like `TP-BG` (`!`)
   out from under it. These files are not strokes for words at all; nothing generated competes
   with them, and no star form or alternate ever reaches them either.
2. **Briefs** (`rules/briefs.toml`). The commonest words keep the outlines people already
   know, from Plover by default (`HE`, not `HAOE`) or Lapwing, per word. They are pinned before
   anything else is generated, and nothing generated can take their strokes. A briefed word gets
   exactly that one entry: no rule outline, no short form, no alternate pronunciation reachable
   alongside it. There is only ever one way to stroke the word, faster to learn than picking the
   cheapest of several, and whatever its rule outline or short forms would have claimed is never
   reserved, so a later, less common word is free to take it instead — a brief can shorten a
   completely unrelated word as a side effect, since words are settled once, commonest first,
   with nothing undone to recheck afterwards. A brief is free to be a pure memorized shortcut with
   no phonetic connection to its word at all — that's the point of a brief, and nothing about it
   needs to be "learnable from the rules" the way a generated outline is. What a candidate may
   never do is take only *part* of a key combination these same rules only ever use *together* for
   one sound. Plover's own shortest outline is sometimes exactly that: a fragment of its own
   theory's multi-key sound, not a clean shortcut (`good`'s Plover brief `KPWAOD` keeps the `P W`
   of `TKPW`'s own "g" but drops the `T`, which these rules read back as "k", not "g" — a
   different, wrong sound, not just an unrelated memorized one). That fragment is rejected, and
   two repairs are generated alongside the original candidates to replace it: completing the
   fragment into the whole, correct cluster, or dropping it outright, since a brief is always
   free to skip a sound it isn't attempting at all. Dropping is usually fewer keys, so it's what
   wins when it's available (`good`'s brief is `AOD` — no attempt at its "g" sound at all, rather
   than the seven-key `TKPWAOD` completing it) — but either repair only exists because the word's
   own gold dictionary pointed at that sound to begin with; nothing is invented from nothing.
   Fewest keys then picks among whichever candidates, original or repaired, are left.
3. **Phonetic rules** (`rules/plover.toml`). Where each sound goes on the keyboard in Plover
   theory: consonants at the start of a syllable, the vowel, consonants at the end, and the
   clusters that fold (`NG K` is `*PBG`, `S T` is `-FT`). A vowel with an `R` of its own (`ER`,
   `AR`) shares the key with a cluster that has it too: `serve` SEFRB, `march` PHAFRPB.
4. **Spelling picks between ways to write a sound.** `UW` is `AO` in *moon* and `AOU` in *tune*;
   `T` is `-D` in *asked*; `ou` is the plain `U` in *rough* where `o` is `O` in *love*. A spelling
   rule can look at the whole word, the start, the end, the sound before, or a prefix on an
   unstressed first vowel (`be-` is `E` in *believe*, `a-` is `A` in *about*). Each turns a sound
   into a named variant (`UW.oo`) with its own entry, so every choice is a line in the rules file
   and `explain` shows which fired.
5. **Prefixes with keys of their own.** `ex-` is `KP` and `dis-` is `STK`; the keys join the
   next stroke's left hand: *expect* KPEBGT, *distance* STKUPBS.
6. **Several syllables.** An unstressed syllable with a reduced vowel is not written: its
   consonants join the stroke before it, in steno order (*living* HREUFG, *special* SPERBL),
   except that a lone `-er` or `-ly` may land out of order as in Plover (*better* PWERT,
   *lately* HRAEULT, *color* KORL). When they don't fit, the stem keeps the consonant it owns
   and the rest is a continuation stroke of consonants only on the right hand: *ended* EPBD/^-D,
   *breaking* PWRAEUBG/^-G, *goodness* TKPWAOD/^-PBS, *harder* HARD/^-R, *confidence*
   KOPB/^-FD/^-PBS. So the suffixes words fold onto are a fixed set that never needs memorising
   apart from the rules, and `rules/suffixes.toml` gives every one of them a stroke on its own
   too, Plover-suffix-dictionary style, so it can be added to a stem that never happened to need
   it while the dictionary was built — see **[the full suffix table](suffixes.md)** for every
   stroke this gives, old and new. Final `-ly` is `^PHREU` after a cluster and a double `l`
   (*silly* SEUL/^EU); any other final `y` is `^EU`, and a final `a` is a syllable of its own
   (*china* KHAOEU/^TPHA). Suffix clusters both dictionaries use are rules: `-ment` -PLT,
   `-tion` -GS, `-ting` -GT, `-ding` -GD, and Plover's F for an S before another consonant
   (*castle* KAFL, *lesson* HREFPB).
7. **Words that sound the same** are settled in this order. First the briefs. Then the spelling
   decides: `sail` (`ai`) keeps the full vowel `SAEUL`, and `sale` (silent e) takes the short
   one `SAEL`, whichever is commoner; the same for `ea` against `ee`, `oa` against `o-e`, and
   `air` against `are`. Then the commonest word keeps the plain outline and the rest take a star
   on the first stroke, a star on the last, then one and two extra star strokes. A short form
   never takes an outline that a one-syllable word's own rules produce.
8. **How short** (`[[brevity]]` in the rules). Each step is a transformation of the full outline
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
   by rule 5. An unstressed first syllable gives a one-stroke form its left hand (*information*
   TPH-FGS, *condition* K-PB, *opinion* P*PB), and the glide of *-ion* is not written. A short
   form never takes an outline that any word has by the rules, so the forms a learner can work
   out stay theirs; when a short form is taken, a star on it is tried before giving up, since a
   star costs a key and a longer form costs a stroke. A word with no brief keeps every free short
   form beside its rule outline, so the dictionary holds both ends of the spectrum and the
   cheapest counts; a word with a brief has already settled for its one entry (see rule 1).
9. **Judged by cost.** `build` prints how many strokes and keys a word takes on average, weighted
   by how often it is said, next to Plover and Lapwing on the same words. The numbers are on
   **[the main README](../README.md#where-lyk-stands)**.

See also: **[the full suffix table](suffixes.md)**, and **[how a miss resolves](fallback.md)**
for what happens when a stroke misses the dictionary outright.
