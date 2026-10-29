# The orthographic fallback: how a miss resolves

[← back to the table of contents](README.md) · [← back to the main README](../README.md)

Every phonetic table in [the theory](theory.md) (rule 2) also reads the other way. `Rules::resolve`
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
`U`) is the one that means "oo". `-T/^AO` reads back as "tuh", not "too" — that's not a miss or a
bug, that's what the vowel key means. If you want "too", the stroke is `-T/^AOU`.

Two different sounds can use the exact same keys (`Z` and its silent-glide cluster `Z Y` are both
`STKPW`; a plain vowel and a spelling variant can be the same keys too), which the rules can't
tell apart from the stroke alone. The read-back always resolves the tie the same way, so the same
miss reads the same way every time — it's consistent, not necessarily "correct" in the sense of
matching what you meant.

## Try it

```
cargo run --release -- resolve STKPW        # a stroke not in the dictionary, read back
cargo run --release -- resolve -- "-T/^AO"  # "tuh" — AO is "uh", not "oo"
cargo run --release -- resolve -- "-T/^AOU" # "too"
```

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
