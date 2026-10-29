# Where lyk stands

[← back to the table of contents](README.md) · [← back to the main README](../README.md)

The headline numbers — the score table by syllable count, keys per stroke against Plover and
Lapwing, and the usage-weighted figure — are on [the main README](../README.md#where-lyk-stands)
now, kept in sync automatically by `score --check-readme`. The rest of the detail lives here.

Longer words differ mostly where Plover and Lapwing have a brief, and where lyk writes a bare
`^-R` for an `-er` they write `*ER`. `mismatches --syllables N
--after 1500` lists what is left.

`build` on the words all three dictionaries write, strokes and keys per word:

| | Strokes | Keys |
|---|---|---|
| this | 1.033 | 4.019 |
| Plover | 1.051 | 4.085 |
| Lapwing | 1.051 | 4.164 |

By how common the word is (`build` prints this), keys per word:

| Words | Share of all words said | this | Plover | Lapwing |
|---|---|---|---|---|
| top 100 | 57% | 2.81 | 2.82 | 2.91 |
| 100 to 1,000 | 28% | 5.04 | 5.18 | 5.23 |
| 1,000 to 5,000 | 10% | 6.14 | 6.42 | 6.52 |
| 5,000 to 20,000 | 4% | 8.04 | 7.87 | 8.06 |

The top thousand words are briefs in all three, which is why the dictionaries are close: the
rules can only move the rest.

That is with the join key on every continuation stroke counted, and with a Plover outline
counted only when Lapwing has it too: Plover's `main.json` is full of misstrokes (`O` for *to*,
`SP` for *and*) that would otherwise make it look a third of a key cheaper than it is. About
1,200 words end up with a
star form because another word sounds the same and has the plain outline (`site` and `sight`,
`don` and `done`); `build` lists the commonest. 93 of the 30,000 words can't be written by these
rules yet, nearly all names and interjections (`hm`, `schmidt`, `vlad`).

These numbers move whenever a brief or suffix is added — see [the theory page](theory.md) for the
rule, and run `cargo run --release -- score --check-readme` before committing a rules change to
catch drift here automatically.
