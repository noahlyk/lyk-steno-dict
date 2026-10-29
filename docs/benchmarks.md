# Where lyk stands

[← back to the table of contents](README.md) · [← back to the main README](../README.md)

On the 20,000 most common words that Plover or Lapwing write and CMUdict pronounces, the rules
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

The next three columns are keys pressed per stroke, lyk next to Plover and Lapwing, on the words
each pair both write (`score` prints this; `score --check-readme` checks this table is still what
it would print, so it can be re-run after a dictionary change instead of trusted to stay accurate
by hand). Lower is
not automatically better here — it says how much a stroke is doing, not how short the word comes
out — but a stroke-count difference between two theories tends to show up here too, since more
strokes for the same word usually means each one is doing less.

The last column weighs that same keys/stroke figure for lyk by how often each word is actually
said (`Setup::count`, the same weighting `build`'s own cost summary uses), instead of counting
every distinct word once regardless of how rare it is. One-syllable words drop sharply under this
weighting (5.72 unweighted vs. 3.09 by usage) because a handful of extremely common one-syllable
words ("the", "a", "to", "of"...) have very short outlines and dominate real typing far more than
the long tail of one-syllable words nobody actually says much. This column is closer to what
typing this theory really feels like than the plain per-word average above it.

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
