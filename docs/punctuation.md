# Punctuation & commands

[← back to the table of contents](README.md) · [← back to the main README](../README.md)

**[Reserved strokes](reserved-strokes.md)** explained *why* these are pinned ahead of every word.
This chapter is the actual content: what's in `rules/reserved/punctuation.json` and
`rules/reserved/commands.json` (copies of the deployed `03-lyk-punctuation.json` and
`01-lyk-commands.json`), grouped by what they do, not the raw JSON.

## Punctuation (`rules/reserved/punctuation.json`)

146 strokes, forked from Plover's own punctuation dictionary. Most marks have far more than one
stroke because Plover's convention gives each mark several chords for several different contexts —
struck plain, struck doubled, struck with a leading or trailing glue so it doesn't force an
unwanted space, struck after a quote or before a parenthesis, and so on. A handful of the busiest
marks, by stroke count:

| Mark | Strokes | A few of them |
|---|---|---|
| `.` | 32 | `-P`, `TP-P`, `P-FP`, `TPHR-FRPBLT` |
| `,` | 14 | `-RB`, `KW-BG`, `W-BG` |
| `?` | 11 | `H-F`, `STP`, `STPH*` |
| (word joiner, no space) | 5 | `SP`, `S-P`, `S*PS` |
| `!` | 5 | `TP-BG`, `SKHAPL`, `STKPWHR-FPLT` |
| `;` | 4 | `SP-PT`, `SKWR*RBGS` |
| `:` | 3 | `STP-FPLT`, `STPH-FPLT` |

Beyond the sentence-ending marks, the same file carries:

- **Quotes and apostrophes** — opening/closing double quotes (`KR*GS`/`KR-GS`), a possessive or
  contraction apostrophe (`-PBT`, `SKW*T`/`SKW-T`).
- **Parens and brackets** — `P*PB` opens `(`, `P*PBT` closes `)`.
- **Dashes and ellipses** — `TK-LS` for a plain dash, `HR-PS`/`SW-BS` for `...`.
- **Symbols that are their own stroke, not built from other punctuation** — `@` (`KWRAT`), `%`
  (`PHR*US`), `=` (`KWA*LS`), `™` (`TR*PL`), `°` (`TKPWRAO*E`), a cent sign (`KAO*UBD`), a currency
  sign (`PHR*SZ`), and a few more.
- **Capitalizing the next word** — `KPA` and its starred form `KPA*` both fire `{-|}`, Plover's own
  "capitalize the next word" directive, with no space inserted. This is how proper nouns are
  handled under this theory (see **[the roadmap](roadmap.md)**'s conflict-taxonomy note): lyk
  itself never generates a dedicated outline shape for a name — a name types as whatever its
  ordinary phonetic outline would be, capitalized by firing this stroke first.

## Commands (`rules/reserved/commands.json`)

41 strokes, smaller and more purpose-built than the punctuation file. Grouped by what they do:

| Group | Strokes | What they send |
|---|---|---|
| Cursor movement | `STPH-R`/`-B`/`-P`/`-G` | Left / Down / Up / Right |
| Word-at-a-time movement | `STPH-RB`/`-BG` | Ctrl+Left / Ctrl+Right |
| Select to start/end of line | `SH-FT`/`SR-RS` | Ctrl+Shift+Home / Ctrl+Shift+End |
| Alt-tabbing between apps | `TPW-R`/`TPW-G`, `TA*BT` | Alt+Left / Alt+Right / Alt+Tab Tab |
| Editing | `TK*EL`, `PW*FP`/`PW-FP`, `TA*B` | Delete, Backspace, Tab |
| Newline / paragraph break | `SKW-BGS`, `R*R`, `SKWRAEURBGS` | Return, or a blank-line paragraph break with the next word capitalized |
| Escape | `TPEFBG`, `EFBG` | Escape |
| App shortcuts via Super | `KPH*F`/`KPH*BG`/`KPH*T`/`KPH-BG` | Super+V / Super+K / Super+W / Super+C |
| App shortcuts via Ctrl | `KHR*F`/`KHR*BG`/`KHR*T`/`KHR-BG` | Ctrl+V / Ctrl+K / Ctrl+W / Ctrl+C |
| Capitalize the next word | `KPA*L`, `KPAD`, `KA*PD` | `{<}` / `{*-\|}` |
| Plover engine control | `PHRO*F`, `PHRO*PB`, `PHROLG` | Suspend / Resume / Toggle Plover |
| Add a translation | `TKUPT` | Plover's "add translation" dialog |
| The literal/raw stroke | `^`, `SP-LS` | `{*!}` — Plover's "don't autocorrect this one" |

Plover's own engine-control strokes (suspend/resume/toggle, add-translation) are here because
they're reserved the same way any other command is: a word can never silently take over the
stroke a steno engine itself depends on to keep working.

Next: **[fingerspelling & misc keys](fingerspelling.md)**.
