# The orthographic fallback: how a miss resolves

[← back to the table of contents](README.md) · [← back to the main README](../README.md)

Every phonetic table in [phonetic rules](phonetic-rules.md) also reads the other way. `Rules::resolve`
takes a stroke that missed the dictionary and reads its keys back into sounds, the same way they
would have been written, spelled plainly rather than in real orthography (there is no word on
record to say how it's actually spelled). Every key on this keyboard means something alone,
under this theory's own tables, and its vowel table covers all 15 non-empty combinations of the
vowel keys, so almost every stroke reads back as *something*.

**A vowelless stroke reads back with no invented vowel.** `-T` alone resolves to just `"t"` — no
guessed-at vowel sound glued on. A stroke with consonants but no vowel key down is a bare
continuation or skeleton stroke, valid either way, not an invitation to invent a sound nobody
pressed. (An earlier, separately hand-maintained copy of this logic inside keymux's WASM plugin
got this wrong for a while — see "Keeping the plugin honest" below for why that can't happen
again.)

**A true miss** — where the rules give up outright and the engine would fall back to a raw echo
of the keys — is only a stroke with nothing left once the star and join keys are set aside: no
onset, no vowel, no coda, because the star and join alone carry no sound.

**Vowel key spelling matters.** `AO` alone is lyk's vowel key for the "uh" sound; `AOU` (with a
`U`) is the one that means "oo". `-T/+AO` reads back as "tuh", not "too" — that's not a miss or a
bug, that's what the vowel key means. If you want "too", the stroke is `-T/+AOU`.

Two different sounds can use the exact same keys (`Z` and its silent-glide cluster `Z Y` are both
`STKPW`; a plain vowel and a spelling variant can be the same keys too), which the rules can't
tell apart from the stroke alone. The read-back always resolves the tie the same way, so the same
miss reads the same way every time — it's consistent, not necessarily "correct" in the sense of
matching what you meant.

## Try it

```
cargo run --release -- resolve STKPW        # a stroke not in the dictionary, read back
cargo run --release -- resolve -- "-T/+AO"  # "too" — see below for why, not "AOU"'s own "too"
cargo run --release -- resolve -- "-T/+AOU" # "too"
```

Bare `AO` is a three-way collision, not a clean pair: it's the stroke for the plain vowel of
"book"/"put" (`UH`), *and* the "-oo" spelling of "too"/"moon"/words ending "-oo" like "kangaroo"
(`UW.oo`), *and* the "-oa" spelling of "boat" (`OW.oa`). A real word's full pronunciation and
spelling tell these apart when the dictionary is built; a miss has no word to go on, so `resolve`
has to pick one reading for the stroke alone, every time, the same way. It picks `UW.oo` over the
other two — not because "book"'s vowel is rare in general, but because it's rare specifically as
an unreduced vowel landing on a *continuation* stroke, where "-oo" endings are common. See
`vowel_rank` in `src/rules.rs` for the exact tie-break and why.

## As a real steno engine's last resort

A dictionary built from these rules can give a real steno engine this as its last resort: look up
the stroke in the dictionary first, and only once that misses, run it through `resolve` for a
reading instead of a raw echo of the keys. In Plover, that's not a dictionary of entries but a
small dictionary *plugin*: a `StenoDictionary` whose `lookup` runs this same read-back in code,
loaded at the bottom of the dictionary stack so it only ever answers once every real entry has
already missed. keymux's own plugin system works the same way — a WASM module asked only once
every dictionary ahead of it in the chain has missed, and only ever for a single stroke (or a run
the keyboard's join key explicitly declared as one continuing word), never for a window of
otherwise-unrelated strokes the engine's own search happened to try.

## Keeping the plugin honest

```
cargo run --release -- plugin [--out out/lyk-orthographic-fallback.rs]
```

generates that plugin's `.rs` source **directly from the rule tables above** — the same
`onset`/`vowel`/`coda` data and the same "no vowel key down means no vowel sound" logic
`resolve` already uses, ported to plain `u32`s with the WASM `alloc`/`fallback` ABI kept as a
fixed, unchanged template around it. Nothing about the phonetic reading is hand-copied or
re-derived by eye anymore, which is what let the old hand-maintained copy drift and invent a
schwa ("uh") for a bare consonant that resolve itself never did. Regenerate and redeploy this
file after any change to the rule tables, rather than hand-editing the deployed plugin directly.

The generated plugin also exports `joins(bits) -> bool`, answering true whenever `Stroke::JOIN`
is set. keymux itself has no built-in idea of a "join key" — every stroke key, standard or a
theory's own extra one, is just a bit to the engine — so without this export the real-time
translator would never recognize `+` as a continuation and would treat every stroke as its own
word, even though direct dictionary lookups of an already-built multi-stroke entry (like
`WORBG/+-G`) would still work. This is why `cargo run -- plugin` must be regenerated and
redeployed any time `Stroke::JOIN`'s bit position changes, not just when a rule table does.

This is the last chapter of the theory itself. Next: **[where lyk stands](../README.md#where-lyk-stands)** —
the benchmarks, a measured result of everything above rather than a rule of its own.
