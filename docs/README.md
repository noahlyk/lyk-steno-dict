# lyk documentation

[← back to the main README](../README.md) for the pitch, the CLI commands, and the data sources.

| Page | What's in it |
|---|---|
| [theory.md](theory.md) | How a word gets its stroke — briefs, phonetic rules, spelling, prefixes, syllables, brevity steps, the join key. The core literature: read this first if you want to understand the theory itself. |
| [suffixes.md](suffixes.md) | Every standalone suffix stroke (`^-G` "ing", `^-D` "ed", and the rest), the one rule they're all derived from, and which ones aren't covered yet. |
| [fallback.md](fallback.md) | What happens when a stroke misses the dictionary outright — `Rules::resolve`'s read-back, why a vowelless stroke never gets an invented vowel, and how that same logic becomes keymux's WASM fallback plugin (`cargo run -- plugin`), generated instead of hand-copied. |
| [benchmarks.md](benchmarks.md) | The headline score table lives on [the main README](../README.md#where-lyk-stands) now (auto-checked against `score --check-readme`); this page has the rest — keys per word by frequency bucket, and what's still unwritten. |

If you're looking for something specific and not sure which page: the **theory** page is the rule
book, **suffixes** is a lookup table, **fallback** is "what if a stroke hits nothing", and
**benchmarks** is "is this actually any good".
