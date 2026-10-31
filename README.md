# lyk-steno-theory

**lyk is steno with explicit word boundaries.** Every stroke after the first of a word carries a
join key (`+`), so a stroke without it always starts a word. Continuation strokes live apart from
the words, which lets a suffix be as short as `-G` or `-D` without ever being read as a word, and
lets a stem overlap with a word without a clash.

This program generates the dictionary (Plover JSON) from phonetic rules, starting from what Plover
and Lapwing already do and measured against them. Inspired by [Froj](https://github.com/StenoHarri/Froj);
written in Rust, with the rules as data. American English only.

## Where lyk stands

One overall number: keys pressed per stroke across every word at once, weighted by how often each
word is actually said — so common short words dominate it and rare long ones barely register,
which is closer to what typing each theory really feels like than counting every distinct word
once would be.

<!-- lyk:overall-table:start -->
| Theory | Keys/Stroke, weighted by how often each word is said |
|---|---|
| Lyk | 3.37 |
| Plover | 3.52 |
| Lapwing | 3.94 |
<!-- lyk:overall-table:end -->

This one number is a composition of two things: how many keys a stroke takes for a word of a
given syllable count, and how much of everything anyone actually types falls into that syllable
count — both broken out in full, along with the rest of the numbers, at
[the bottom of this README](#full-benchmarks). Run and checked automatically
(`score --check-readme`), not hand-typed.

Lyk beats Plover here. A brief is free to be a pure memorized shortcut with no phonetic
connection to its word at all (`it` on the onset `T-`, which "it" doesn't start with, is exactly
as fine as a brief has always been), and free to drop a sound entirely rather than attempt it
(`good` on bare `AOD`, no attempt at its "g" at all). What a brief can never do is take only
*part* of a key combination the rules only ever use *together* for one sound (`good`'s old Plover
brief kept half of its "g" sound's four keys and dropped the rest, reading back as a different,
wrong consonant) — a fragment like that is confusing in a way neither a whole memorized shortcut
nor a cleanly dropped sound ever is. See [the briefs chapter](docs/briefs.md) for the rule.

## Documentation

The full literature lives in [`docs/`](docs/README.md), written to be read start to finish, not
looked up — **start there for the full table of contents and reading order.** A few waypoints:

- **[The join key](docs/join-key.md)** — why word boundaries are explicit. Read this first.
- **[Briefs](docs/briefs.md)** — the rule book for the commonest words' outlines.
- **[The full suffix table](docs/suffixes.md)** — every standalone suffix (`+-G` "ing", `+-D`
  "ed", ...), the one rule they're derived from, and what isn't covered yet.
- **[Punctuation & commands](docs/punctuation.md)** and
  **[fingerspelling & misc keys](docs/fingerspelling.md)** — the reserved strokes pinned ahead of
  every word.
- **[The orthographic fallback](docs/fallback.md)** — what a stroke that misses the dictionary
  reads back as, and how that logic becomes keymux's WASM plugin, generated rather than hand-copied.

The numbers themselves (this page's "Where lyk stands" above, and the full breakdown at
[the bottom of this README](#full-benchmarks)) aren't a separate doc page — they're meant to be
seen, not looked up.

## Main features

- **Optimized for speed, not for being guessable**: a word's single-stroke outline comes from
  whichever source — a known brief, the phonetic rules, a brevity-ladder compression — actually
  costs the least to type, not from a fixed rule about which words are "allowed" to be arbitrary.
  Nothing about it needs to be derivable from the word's sound or spelling.
- **One outline per word, always**: a word has exactly one way to stroke it — no competing long
  form to also learn, and whatever that long form would have claimed is free for a different word
  instead.
- **An explicit join key**: continuation strokes are never mistaken for words, which is what lets
  suffixes be as short as a single key.
- **Measured, not asserted**: `build` and `score` print cost and match-rate against Plover and
  Lapwing on the same words, every time — see [the full benchmarks](#full-benchmarks).
- **A miss always resolves**: a stroke the dictionary doesn't have still reads back as something
  sensible, phonetically, as a real engine's last resort — see [the fallback page](docs/fallback.md).

## Use

```
cargo run --release -- fetch               # download the data into data/ (safe to re-run)
cargo run --release -- report              # what was loaded
cargo run --release -- score               # how often the rules match Plover or Lapwing
cargo run --release -- score --check-readme   # do this README's tables still match?
cargo run --release -- build               # briefs + rules -> out/lyk.json, with the cost
cargo run --release -- plugin              # generate keymux's WASM fallback plugin from the rules
cargo run --release -- deploy              # build + plugin, copied straight into keymux's live config
                                            #   (~/.config/keymux/steno/lyk by default; override with --to)
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

## Full benchmarks

Everything behind [the one overall number above](#where-lyk-stands), in full. On the 100,000 most
common words that Plover or Lapwing write and CMUdict pronounces, the rules write 99.9% of them,
and write the word the way Plover or Lapwing does (ignoring the join key and the `KWR` or star
they put on a continuation stroke, which the join key replaces) for:

<!-- lyk:score-table:start -->
| Syllables | Same as Plover or Lapwing | Plover | Lapwing | Keys/Stroke (Lyk) | Keys/Stroke (Plover) | Keys/Stroke (Lapwing) |
|---|---|---|---|---|---|---|
| 1 | 90% | 78% | 83% | 5.59 | 5.66 | 5.83 |
| 2 | 59% | 45% | 52% | 5.56 | 4.93 | 5.27 |
| 3 | 38% | 26% | 32% | 5.22 | 4.80 | 5.15 |
| 4+ | 23% | 17% | 18% | 5.38 | 4.97 | 5.22 |
<!-- lyk:score-table:end -->

"Keys/Stroke" above counts every distinct word once, regardless of how rare it is. The same
figure, with each word counted not once but as many times as it is actually said — so a handful
of very common words dominate it and a long tail of rare ones barely move it:

<!-- lyk:usage-table:start -->
| Syllables | Keys/Stroke (Lyk, by frequency) | Keys/Stroke (Plover, by frequency) | Keys/Stroke (Lapwing, by frequency) |
|---|---|---|---|
| 1 | 3.02 | 3.17 | 3.59 |
| 2 | 4.73 | 4.70 | 5.00 |
| 3 | 4.57 | 4.93 | 5.25 |
| 4+ | 4.93 | 5.24 | 5.50 |
<!-- lyk:usage-table:end -->

One-syllable words drop sharply under this weighting for every theory, not just Lyk, because a
handful of extremely common short words ("the", "a", "to", "of"...) dominate real typing far more
than the long tail of rare one-syllable words.

How common each syllable count actually is — first counting every distinct word once:

<!-- lyk:syllable-words-table:start -->
| Syllables | Share of distinct words |
|---|---|
| 1 | 28% |
| 2 | 41% |
| 3 | 22% |
| 4+ | 10% |
<!-- lyk:syllable-words-table:end -->

...and then weighted by how often each word is actually said, which is "how much of everything
anyone types has this many syllables", not "how many words of this length exist":

<!-- lyk:syllable-usage-table:start -->
| Syllables | Share of words actually said |
|---|---|
| 1 | 80% |
| 2 | 15% |
| 3 | 4% |
| 4+ | 1% |
<!-- lyk:syllable-usage-table:end -->

Most of what anyone actually types is one or two syllables, which is why [the overall number at
the top of this README](#where-lyk-stands) sits so much closer to the one- and two-syllable
frequency-weighted figures above than to the longer-word ones — it's these two tables together:
how much of everything typed is each syllable count, and how many keys a stroke takes at each
one.

Longer words differ mostly where Plover and Lapwing have a brief, and where Lyk writes a bare
`+-R` for an `-er` they write `*ER`. `mismatches --syllables N --after 1500` lists what is left.

`build` on the words all three dictionaries write, strokes and keys per word:

| | Strokes | Keys |
|---|---|---|
| Lyk | 1.016 | 3.435 |
| Plover | 1.039 | 3.717 |
| Lapwing | 1.041 | 4.114 |

By how common the word is (`build` prints this), keys per word:

| Words | Share of all words said | Lyk | Plover | Lapwing |
|---|---|---|---|---|
| top 100 | 58% | 2.24 | 2.48 | 2.91 |
| 100 to 1,000 | 28% | 4.60 | 4.81 | 5.17 |
| 1,000 to 5,000 | 10% | 5.54 | 6.14 | 6.42 |
| 5,000 to 20,000 | 4% | 6.81 | 7.47 | 7.88 |
| 20,000+ | 1% | 8.60 | 8.91 | 9.29 |

The top thousand words are briefs in all three, which is why the dictionaries are close: the
rules can only move the rest. Lyk now beats Plover across every frequency tier, including the
rarest words — every word's primary outline is settled by a single global optimization over every
legal candidate at once (briefs, the plain rules reading, brevity, the star ladder, all competing
on cost together), not a commonest-word-first pass that only ever revisits a word's own brief
attempt; see [briefs](docs/briefs.md) for how.

That is with the join key on every continuation stroke counted, and with a Plover outline counted
only when Lapwing has it too: Plover's `main.json` is full of misstrokes (`O` for *to*, `SP` for
*and*) that would otherwise make it look a third of a key cheaper than it is. Close to 4,000 words
end up with a star form because some other, more valuable word's candidate won the plain outline
instead (`site` and `sight`, `don` and `done`, but also any other collision the global optimizer
settles this way now, not just a sound-alike pair); `build` lists the commonest. About 300 of the
100,000 words can't be written by these rules yet, nearly all names and interjections (`hm`,
`schmidt`, `vlad`). On top of all that, close to 20,000 words also get a second, backup entry built
from an already-placed shorter word's own outline plus whatever sounds are left over (`build`'s own
outline plus a continuation stroke also reaches `building`, whether or not that agrees with
`building`'s own cheapest outline) — see [syllables & folding](docs/syllables.md) for the rule.

These numbers move whenever a brief or suffix is added — see [the docs](docs/README.md) for the
rule, and run `cargo run --release -- score --check-readme` before committing a rules change
to catch drift here automatically.
