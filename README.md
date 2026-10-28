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

## Use

American English only. Everything is Rust subcommands; there are no scripts.

```
cargo run --release -- fetch    # download the data into data/ (not committed; safe to re-run)
cargo run --release -- report   # what was loaded, and how well it covers Plover and Lapwing
```

| What | Source |
|---|---|
| Pronunciations | CMUdict, MFA US dictionary, WikiPron US broad |
| Word frequency | hermitdave/FrequencyWords (OpenSubtitles 2018) |
| Dictionaries to match | Plover `main.json` and `commands.json`, Lapwing dictionaries |
