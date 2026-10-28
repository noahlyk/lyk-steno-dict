# lyk-steno-dict

**lyk is steno with explicit word boundaries.** Every stroke after the first of a word carries a
join key (`^`), so a stroke without it always starts a word. Continuation strokes live apart from
the words, which lets a suffix be as short as `-G` or `-D` without ever being read as a word, and
lets a stem overlap with a word without a clash.

This program generates the dictionary (Plover JSON) from phonetic rules, starting from what Plover
and Lapwing already do and measured against them. Inspired by [Froj](https://github.com/StenoHarri/Froj);
written in Rust, with the rules as data. American English only.

## How it works

1. **Briefs first** (`rules/briefs.toml`). The commonest words keep the outlines people already
   know, from Plover by default (`HE`, not `HAOE`) or Lapwing, per word. They are pinned before
   anything is generated, and nothing generated can take their strokes.
2. **Phonetic rules** (`rules/plover.toml`). Where each sound goes on the keyboard in Plover
   theory: consonants at the start of a syllable, the vowel, consonants at the end, and the
   clusters that fold (`NG K` is `*PBG`, `S T` is `-FT`). A vowel with an `R` of its own (`ER`,
   `AR`) shares the key with a cluster that has it too: `serve` SEFRB, `march` PHAFRPB.
3. **Spelling picks between ways to write a sound.** `UW` is `AO` in *moon* and `AOU` in *tune*;
   `T` is `-D` in *asked*; `ou` is the plain `U` in *rough* where `o` is `O` in *love*. A spelling
   rule can look at the whole word, the start, the end, the sound before, or a prefix on an
   unstressed first vowel (`be-` is `E` in *believe*, `a-` is `A` in *about*). Each turns a sound
   into a named variant (`UW.oo`) with its own entry, so every choice is a line in the rules file
   and `explain` shows which fired.
4. **Prefixes with keys of their own.** `ex-` is `KP` and `dis-` is `STK`; the keys join the
   next stroke's left hand: *expect* KPEBGT, *distance* STKUPBS.
5. **Several syllables.** An unstressed syllable with a reduced vowel is not written: its
   consonants join the stroke before it, in steno order (*living* HREUFG, *special* SPERBL,
   *lately* HRAEUT/^-L). When they don't fit, the stem keeps the consonant it owns and the rest
   is a continuation stroke on the right hand: *ended* EPBD/^-D, *breaking* PWRAEUBG/^-G,
   *goodness* TKPWAOD/^-PBS, *better* PWET/^ER. Final `-ly` folds as `-L`; any other final `y`
   is `^EU`.
6. **Words that sound the same** are settled in this order. First the briefs. Then the spelling
   decides: `sail` (`ai`) keeps the full vowel `SAEUL`, and `sale` (silent e) takes the short
   one `SAEL`, whichever is commoner; the same for `ea` against `ee`, `oa` against `o-e`, and
   `air` against `are`. Then the commonest word keeps the plain outline and the rest take a star
   on the first stroke, a star on the last, then one and two extra star strokes. A short form
   never takes an outline that a one-syllable word's own rules produce.
7. **How short** (`[[brevity]]` in the rules). Each step is a transformation of the full outline
   that can be done by hand, with a frequency rank below which words take it: `skeleton` is one
   stroke of the first consonants and every consonant after the first vowel, `shell` keeps those
   of the stressed and last syllables, `ends` those of the last; each with or without the
   stressed vowel. An empty list gives a Lapwing-like dictionary, a stroke per syllable; a huge
   rank on every step gives the shortest one the rules allow. Words that take no step are written
   by rule 5.
8. **Judged by cost.** `build` prints how many strokes and keys a word takes on average, weighted
   by how often it is said, next to Plover and Lapwing on the same words.

## Where it stands

On the 20,000 most common words that Plover or Lapwing write and CMUdict pronounces, the rules
write 99.9% of them, and write the word the way Plover or Lapwing does (ignoring the join key and
the `KWR` or star they put on a continuation stroke, which the join key replaces) for:

| Syllables | Same as Plover or Lapwing | Plover | Lapwing |
|---|---|---|---|
| 1 | 90% | 78% | 83% |
| 2 | 61% | 48% | 51% |
| 3 | 36% | 26% | 29% |
| 4+ | 21% | 15% | 16% |

Longer words differ mostly where Plover and Lapwing have a brief. `mismatches --syllables N
--after 1500` lists what is left.

`build` on the words all three dictionaries write, strokes and keys per word:

| | Strokes | Keys |
|---|---|---|
| this | 1.044 | 3.724 |
| Plover | 1.038 | 3.709 |
| Lapwing | 1.040 | 4.115 |

That is with the join key on every continuation stroke counted. About 1,100 words end up with a
star form because another word sounds the same and has the plain outline (`site` and `sight`,
`don` and `done`); `build` lists the commonest. 105 of the 30,000 words can't be written by these
rules yet, nearly all names and interjections (`hm`, `schmidt`, `vlad`).

## Use

```
cargo run --release -- fetch               # download the data into data/ (safe to re-run)
cargo run --release -- report              # what was loaded
cargo run --release -- score               # how often the rules match Plover or Lapwing
cargo run --release -- build               # briefs + rules -> out/lyk.json, with the cost
cargo run --release -- explain moon        # which rules wrote a word, next to Plover and Lapwing
cargo run --release -- mismatches --syllables 2 --after 500    # where the rules differ
cargo run --release -- learn               # which keys Plover and Lapwing use for each sound group
```

`--rules FILE` and `--briefs FILE` use your own rules and briefs. `learn` is how most rules were
found: it counts what keys Plover and Lapwing use where they agree.

| What | Source |
|---|---|
| Pronunciations | CMUdict, MFA US dictionary, WikiPron US broad |
| Word frequency | hermitdave/FrequencyWords (OpenSubtitles 2018) |
| Dictionaries to match | Plover `main.json` and `commands.json`, Lapwing dictionaries |
