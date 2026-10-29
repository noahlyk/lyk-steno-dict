# lyk documentation

[← back to the main README](../README.md) for the pitch, the CLI commands, and the data sources.

| Page | What's in it |
|---|---|
| [theory.md](theory.md) | How a word gets its stroke — briefs, phonetic rules, spelling, prefixes, syllables, brevity steps, the join key. The core literature: read this first if you want to understand the theory itself. |
| [suffixes.md](suffixes.md) | Every standalone suffix stroke (`^-G` "ing", `^-D` "ed", and the rest), the one rule they're all derived from, and which ones aren't covered yet. |
| [fallback.md](fallback.md) | What happens when a stroke misses the dictionary outright — `Rules::resolve`'s read-back, why a vowelless stroke never gets an invented vowel, and how that same logic becomes keymux's WASM fallback plugin (`cargo run -- plugin`), generated instead of hand-copied. |

The benchmarks — all of them, not a separate page — live on [the main README](../README.md#where-lyk-stands):
the one overall number up top, and the full breakdown (every table, auto-checked against
`score --check-readme`) at [its bottom](../README.md#full-benchmarks).

If you're looking for something specific and not sure which page: the **theory** page is the rule
book, **suffixes** is a lookup table, and **fallback** is "what if a stroke hits nothing". "Is
this actually any good" is answered on the main README, not here.
