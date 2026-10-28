# lyk-steno-dict

Generates a steno dictionary (Plover JSON) from phonetic rules, starting from what Plover and
Lapwing already do. Inspired by [Froj](https://github.com/StenoHarri/Froj); written in Rust, with
the rules as data. American English only.

## How it works

1. **Briefs first** (`rules/briefs.toml`). The commonest words keep the outlines people already
   know, from Plover by default (`HE`, not `HAOE`) or Lapwing, per word. They are pinned before
   anything is generated, and nothing generated can take their strokes.
2. **Phonetic rules** (`rules/plover.toml`). Where each sound goes on the keyboard in Plover
   theory: consonants at the start of a syllable, the vowel, consonants at the end, and the
   clusters that fold (`NG K` is `*PBG`, `S T` is `-FT`).
3. **Spelling picks between ways to write a sound.** `UW` is `AO` in *moon* and `AOU` in *tune*;
   `T` is `-D` in *asked*. A spelling rule turns a sound into a named variant (`UW.oo`) that has
   its own entry, so every choice is a line in the rules file and `explain` shows which fired.
4. **Several syllables.** An unstressed syllable with a reduced vowel is not written: its
   consonants join the stroke before it (*living* `HREUFG`, *special* `SPERBL`). The commonest
   long words can instead be one stroke of just their consonants, the fewest keys a word can
   take (`skeleton_rank` in the rules).
5. **Words that sound the same** are settled in a fixed order, the commonest word first: the
   plain outline, then a star on the first stroke, a star on the last, then one and two extra
   star strokes. Nothing depends on the order words are listed in.
6. **Judged by cost.** `build` prints how many strokes and keys a word takes on average, weighted
   by how often it is said, next to Plover and Lapwing on the same words. The aim is fewer keys
   overall.

## Where it stands

On the 20,000 most common words that Plover or Lapwing write and CMUdict pronounces, the rules
alone write the word the way Plover or Lapwing does for:

| Syllables | Same as Plover or Lapwing |
|---|---|
| 1 | 80% |
| 2 | 41% |
| 3 | 22% |
| 4+ | 12% |

Longer words differ on purpose: the rules are predictable where Plover's briefs are not.

`build` on the words all three dictionaries write, strokes and keys per word:

| | Strokes | Keys |
|---|---|---|
| this | 1.034 | 3.633 |
| Plover | 1.035 | 3.673 |
| Lapwing | 1.037 | 4.069 |

## The join key

`join_key = true` in the rules marks every stroke after the first of a word with a join key
(`^`), so those strokes live apart from the standalone words and can be short and overlap. The
cost is one key on every continuation. With the strokes as they are now it costs 3.662 keys per
word, slightly more than without (3.633): the saving needs continuation strokes written for it,
which is the next step. Plover cannot read these strokes.

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
