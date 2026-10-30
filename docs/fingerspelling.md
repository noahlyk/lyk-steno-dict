# Fingerspelling & misc keys ("Emily's")

[← back to the table of contents](README.md) · [← back to the main README](../README.md)

The other half of **[reserved strokes](reserved-strokes.md)**:
`rules/reserved/emilys-modifiers.py`/`.json` (a copy of the deployed `04-lyk-emilys-modifiers`),
forked from Lapwing's "Emily's Modifier Dictionary." It's theory-independent — plain Plover
fingerspelling plus a chorded Ctrl/Alt/Super/Shift ending — so it needed no changes to work the
same way under lyk. `.py` is the source; `.json` (5,475 entries) is generated from it by
`regenerate.py` and is what `generate()` actually reserves strokes from.

## Why every entry ends the same way

Every single lookup this dictionary can answer ends in the same four keys struck together, `LGTS`
(spelled `-LGTS`, the right hand's bottom-row keys). That ending exists purely so this dictionary
never fires by accident: without it, an ordinary word's own outline could collide with a modifier
chord. A lookup also always needs at least one of Shift/Alt/Super/Ctrl held (see below) — a bare
letter or symbol with no modifier isn't this dictionary's job at all.

## Modifiers

Four keys on the right hand, any combination of them, map onto the four OS modifiers:

| Key | Modifier |
|---|---|
| `R` | Shift |
| `B` | Alt |
| `P` | Super |
| `F` | Ctrl |

They combine (`KPWR*EFPBLGTS` is Ctrl+Super+Alt+Up) and wrap around whatever character or key the
rest of the stroke names, innermost first: `{^}{#control(alt(up))}{^}`.

## Fingerspelling the alphabet

Each letter is its own steno chord, the same ones Plover's own fingerspelling dictionary uses
(`A` = `a`, `PW` = `b`, `KR` = `c`, `TK` = `d`, `TKPW` = `g`, `SKWR` = `j`, `STKPW` = `z`, ...),
struck with a modifier and the `LGTS` ender: `PW-RLGTS` is Shift+B, `KR-RLGTS` is Shift+C. A few
letters have a second, "magnum"-style chord as well (`AOEU` or `EU` both read as `i`; `SKWRAEU` or
`SKWR` both read as `j`; `STKPWHR` or `STKPW` both read as `z`). This same alphabet, stripped of
the `LGTS` ender, seeds the letter table **[spelling mode](spelling-mode.md)** reads a
no-pronunciation word back from — a different purpose (prose, not an OS keybinding), so it keeps
only each letter's primary chord.

## Numbers and function keys

The left hand's bottom row (`R`, `W`, `K`, `S`) counts in binary when struck with the vowel `AO`
and no modifier letter reused for a digit value: `R`=1, `W`=2, `K`=4, `S`=8, combined by addition
(so `RWKS` together would be 15, though only 0–9 are valid). `AORLGTS` alone (no bottom-row keys)
is Shift+0; `RAORLGTS` is Shift+1; `WAORLGTS` is Shift+2. Holding `T` and `P` together as well
turns the same digit into a function key: `TPAORLGTS` is Shift+F0 (Plover's own zero-indexed F-key
numbering).

## Symbols and special keys

A star in the stroke (`*`) switches the same chord shape over to a lookup table of symbols and
special keys instead of letters or digits, with up to four variants per chord selected by whether
`A` and/or `O` is also down:

| Chord | Variants (no vowel / `A` / `O` / `A`+`O`) |
|---|---|
| `TR` | Tab / Backspace / Delete / Escape |
| `KPWR` | Up / Left / Right / Down |
| `TKPWR` | Page Up / Home / End / Page Down |
| `TKWR` | Audio Play / Audio Prev / Audio Next / Audio Stop |
| `TKW` | Audio Mute / Audio Lower Volume / Audio Raise Volume |
| `TPH` | `(` / `<` / `[` / `{` |
| `KWR` | `)` / `>` / `]` / `}` |
| `H` | `*` / — / — / `×` |

...and a long tail of one-variant typable symbols on their own chord: `TK` → `!`, `TP` → `"`,
`KPWH` → `$`, `TKPW` → `%`, `T` → `'`, `K` → `.`, `KP` → `/`, `KW` → `;`, `TPW` → `?`, `TW` → `\`,
`WR` → `_`, `P` → `` ` ``, and more — each reachable the same way, a modifier plus the chord plus
`LGTS` (`K*FLGTS` is Ctrl+`.`, `TPW*FLGTS` is Ctrl+`?`).

Next: **[the fallback](fallback.md)** — what happens once a stroke misses every dictionary above,
reserved or generated.
