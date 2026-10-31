# Spelling mode: when a word has no sound to go on

[← back to the table of contents](README.md) · [← back to the main README](../README.md)

[The fallback](fallback.md) reads a missed stroke back into sounds — a reasonable last resort for
any word the rules *can* generate from, since it has a real pronunciation to work from. Some words
have no pronunciation on record at all: `stenography` isn't in CMUdict, so the generator has
nothing to run its onset/vowel/coda tables against, and the phonetic fallback has nothing better
to do than guess at sounds nobody struck. Spelling mode is a second, deliberate way to type a word
like that: not a guess, every letter struck explicitly.

## The marker

A spelling-mode run starts with `~` instead of `+`: `Stroke::SPELL`, a second virtual bit (the
25th) alongside [the join key](join-key.md)'s own 24th. It means "read this stroke, and every one
after it that carries the ordinary join key, as a run of letter chords, not sounds" — the same
`+`-joined-run mechanism [the fallback](fallback.md#as-a-real-steno-engines-last-resort) already
uses to chain a multi-stroke phonetic word into one reading, reused here for a different purpose
rather than reinvented. Only the first stroke of the run carries `~`; the rest look exactly like
any other continuation, `+` and all.

## The letter table

`rules/spelling.toml` gives every letter one bare chord — `A` for "a", `PW` for "b", `KR` for "c",
and so on through `STKPW` for "z" — seeded from the alphabet `rules/reserved/emilys-modifiers.py`
already ships for OS-level fingerspelling (see
**[fingerspelling & misc keys](fingerspelling.md)**), but without that file's `-LGTS` ender
requirement. That requirement exists there only so a bare letter chord isn't mistaken for an
ordinary word in Plover's own fingerspelling dictionary; spelling mode has no such ambiguity, since
the `~`/`+` marker bits already say unambiguously "this run is spelled, not sounded." Each letter
gets exactly one chord — the "magnum" alternates `rules/reserved/emilys-modifiers.py` also lists
(`AOEU` beside `EU` for "i") are a convenience for that file's own purpose, not needed here.

## How a run decodes

`Rules::resolve` checks the first stroke of an outline for the spell bit before doing anything
else; if it's set, the whole outline is handed to `Rules::resolve_spelling` instead of the normal
onset/vowel/coda read-back, which strips the spell bit (first stroke) or join bit (every stroke
after it) from each stroke's keys and looks the bare chord up in the letter table, one letter per
stroke, concatenated in order. There's no ambiguity to resolve the way a missed phonetic stroke
has — no ten-way vowel-key collision, no ranking a plain sound over a spelling variant — because
each letter was struck explicitly rather than inferred from a sound.

`stenography` spelled out is eleven strokes, one letter each:

```
~S/+T/+E/+TPH/+O/+TKPW/+R/+A/+P/+H/+KWH
```

```
cargo run --release -- resolve -- "~S/+T/+E/+TPH/+O/+TKPW/+R/+A/+P/+H/+KWH"
```

reads back `stenography`, letter for letter — compare that to the garbled guess the ordinary
phonetic fallback would produce for a stroke sequence that happened to share some of those keys,
since it has no way to know the word wasn't meant to be sounded out at all.

`cargo run -- plugin` generates the exact same letter table and dispatch into the deployed WASM
plugin, the same way it already does for the phonetic tables (see
[fallback.md](fallback.md#keeping-the-plugin-honest)) — nothing about spelling mode is a
second, hand-maintained copy of this rule either.

## Going the other way: `Rules::spell_word`

Everything above reads a spelling-mode outline back into a word — useful once you already know
you're going to spell something out, but no help deciding *how* to spell it: hand-writing
`~S/+T/+E/+TPH/+O/+TKPW/+R/+A/+P/+H/+KWH` for `stenography` letter by letter, by eye, against
`rules/spelling.toml`, isn't something a learner should have to do. `Rules::spell_word` is the
exact inverse of `resolve_spelling`: given the word, it looks each letter up in the same table and
produces that same outline directly, `~` on the first stroke and `+` on every one after it — `None`
for any word with a character the table has no chord for (anything outside plain a-z). This is what
a lookup tool should fall back to once a word has no pronunciation at all for the phonetic rules to
run from, rather than reporting the word as simply untypeable.

## Not yet wired to a physical key

Unlike the join key, `~` has no physical key bound to it in the live keymux config yet. keymux's
own engine is mid-refactor (the same `Layout`/`extra_keys` machinery the join key now runs
through) in a separate, ongoing change, and wiring a second extra key into a real config is that
change's job to finish, not this one's. Until then, spelling mode is fully implemented and
testable through `cargo run -- resolve` and the generated plugin, but not yet something you can
actually strike at a keyboard — the mechanism is proven ahead of the hardware catching up to it,
not the other way around.

This is the last chapter of the theory itself. Next: **[where lyk stands](../README.md#where-lyk-stands)** —
the benchmarks, a measured result of everything above rather than a rule of its own.
