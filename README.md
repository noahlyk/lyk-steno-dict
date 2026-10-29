# lyk-steno-dict

**lyk is steno with explicit word boundaries.** Every stroke after the first of a word carries a
join key (`^`), so a stroke without it always starts a word. Continuation strokes live apart from
the words, which lets a suffix be as short as `-G` or `-D` without ever being read as a word, and
lets a stem overlap with a word without a clash.

This program generates the dictionary (Plover JSON) from phonetic rules, starting from what Plover
and Lapwing already do and measured against them. Inspired by [Froj](https://github.com/StenoHarri/Froj);
written in Rust, with the rules as data. American English only.

## Where lyk stands

On the 100,000 most common words that Plover or Lapwing write and CMUdict pronounces, the rules
write 99.9% of them, and write the word the way Plover or Lapwing does (ignoring the join key and
the `KWR` or star they put on a continuation stroke, which the join key replaces) for:

<!-- lyk:score-table:start -->
| Syllables | Same as Plover or Lapwing | Plover | Lapwing | Keys/Stroke (this) | Keys/Stroke (Plover) | Keys/Stroke (Lapwing) | Keys/Stroke (this, by usage) |
|---|---|---|---|---|---|---|---|
| 1 | 90% | 78% | 83% | 5.72 | 5.66 | 5.83 | 3.09 |
| 2 | 60% | 45% | 52% | 5.68 | 4.93 | 5.27 | 4.93 |
| 3 | 39% | 26% | 33% | 5.29 | 4.80 | 5.15 | 4.97 |
| 4+ | 24% | 17% | 19% | 5.30 | 4.97 | 5.22 | 5.18 |
<!-- lyk:score-table:end -->

The last column weighs lyk's keys/stroke by how often each word is actually said, instead of
counting every distinct word once regardless of how rare it is — one-syllable words drop sharply
under this weighting (5.72 unweighted vs. 3.09 by usage) because a handful of extremely common
short words ("the", "a", "to", "of"...) dominate real typing far more than the long tail of rare
one-syllable words. This is run and checked automatically (`score --check-readme`), not hand-typed
— see [the full benchmarks page](docs/benchmarks.md) for the rest: strokes/keys per word, cost by
how common a word is, and the caveats behind these numbers.

## Documentation

The full literature lives in [`docs/`](docs/README.md) — start there for the table of contents.
Quick links:

- **[How lyk writes a word](docs/theory.md)** — the rule book: briefs, phonetic rules, spelling,
  prefixes, syllables, brevity, the join key. Read this first.
- **[Suffix strokes](docs/suffixes.md)** — every standalone suffix (`^-G` "ing", `^-D` "ed", ...),
  the one rule they're derived from, and what isn't covered yet.
- **[The orthographic fallback](docs/fallback.md)** — what a stroke that misses the dictionary
  reads back as, and how that logic becomes keymux's WASM plugin, generated rather than hand-copied.
- **[Where lyk stands](docs/benchmarks.md)** — the numbers, next to Plover and Lapwing, on the
  same words.

## Main features

- **Rule-generated, not hand-picked**: every outline but a brief comes from phonetic rules applied
  uniformly, not chosen word by word.
- **One outline per briefed word**: a word with a brief has exactly one way to stroke it — no
  competing long form to also learn, and whatever that long form would have claimed is free for a
  less common word instead.
- **An explicit join key**: continuation strokes are never mistaken for words, which is what lets
  suffixes be as short as a single key.
- **Measured, not asserted**: `build` and `score` print cost and match-rate against Plover and
  Lapwing on the same words, every time — see [the benchmarks](docs/benchmarks.md).
- **A miss always resolves**: a stroke the dictionary doesn't have still reads back as something
  sensible, phonetically, as a real engine's last resort — see [the fallback page](docs/fallback.md).

## Use

```
cargo run --release -- fetch               # download the data into data/ (safe to re-run)
cargo run --release -- report              # what was loaded
cargo run --release -- score               # how often the rules match Plover or Lapwing
cargo run --release -- score --check-readme   # does docs/benchmarks.md's score table still match?
cargo run --release -- build               # briefs + rules -> out/lyk.json, with the cost
cargo run --release -- plugin              # generate keymux's WASM fallback plugin from the rules
cargo run --release -- explain moon        # which rules wrote a word, next to Plover and Lapwing
cargo run --release -- mismatches --syllables 2 --after 500    # where the rules differ
cargo run --release -- learn               # which keys Plover and Lapwing use for each sound group
cargo run --release -- build --against magnum.rtf   # add a third dictionary to the cost tables
cargo run --release -- quote                        # a random quote, stroked lyk next to Plover and Lapwing
cargo run --release -- resolve STKPW                # a stroke not in the dictionary, read back
```

`--rules FILE` and `--briefs FILE` use your own rules and briefs. `learn` is how most rules were
found: it counts what keys Plover and Lapwing use where they agree.

| What | Source |
|---|---|
| Pronunciations | CMUdict, MFA US dictionary, WikiPron US broad |
| Word frequency | hermitdave/FrequencyWords (OpenSubtitles 2018) |
| Dictionaries to match | Plover `main.json` and `commands.json`, Lapwing dictionaries |
