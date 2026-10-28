# lyk-steno-dict

Generates a steno dictionary (Plover JSON) from phonetic rules, starting from what Plover and
Lapwing already do. Inspired by [Froj](https://github.com/StenoHarri/Froj); written in Rust, with
the rules as data.

## Plan

1. **Measure.** Load pronunciations and the Plover/Lapwing dictionaries, and report how much of
   what they write the generator reproduces. Every rule is judged by that number.
2. Single-syllable phonetic rules (consonant clusters, vowels, steno order).
3. Multi-syllable words and morphology (prefixes, roots, suffixes).
4. Conflicts, ranked by word frequency and ambiguity, with a report.
5. Common-word briefs from Plover or Lapwing come first and reserve their strokes. The rules fill
   in the rest and never override a brief.
6. Optional theory ideas, such as an explicit word-boundary key.
7. A lookup command that says which rules built an outline.

## Data

`scripts/fetch-data.sh` downloads into `data/` (not committed):

| What | Source |
|---|---|
| Pronunciations | CMUdict, MFA US/UK dictionaries, WikiPron US/UK broad |
| Word frequency | hermitdave/FrequencyWords (OpenSubtitles 2018) |
| Dictionaries to match | Plover `main.json`, Lapwing dictionaries |

`cargo run --release` prints what was loaded and how well the pronunciations cover the words
Plover and Lapwing write.
