# Suffix strokes

[← back to the table of contents](README.md) · [← back to the main README](../README.md)

Source of truth: [`rules/suffixes.toml`](../rules/suffixes.toml) — this page is a readable table
over that file, not a separate copy of the rule. Regenerate the dictionary (`cargo run --release
-- build`) after editing the TOML; this page is prose and isn't checked against it automatically.

## The rule

A suffix is a syllable like any other, so it's written by running its own sounds through the
exact tables `rules/plover.toml` already uses for any syllable (`[onset]`, `[vowel]`, `[coda]`,
`[coda_cluster]`, `[vowel_coda]`) — nothing in `rules/suffixes.toml` is a key combination invented
just for this file. Its vowel is then dropped, the same way `[reduce]` already drops a whole
word's own reduced final syllable when it folds onto a continuation stroke of its own, **whenever
that still tells the suffix apart from every other suffix stroke already claimed**. Bare
consonants read the same whether they came from a word's own ending or a suffix struck on its
own — that's the point of the rule.

When dropping the vowel would land on a stroke another suffix already claims, the vowel stays
instead, to tell them apart (see the "kept" rows below). A vowel outside `[reduce]` never had
anything to drop in the first place. A suffix that's genuinely two syllables strokes as two
continuations, each one still just a syllable through the same rule.

## Bare-consonant suffixes (the vowel dropped)

| Stroke | Suffix | Example |
|---|---|---|
| `^-G` | -ing | *walk-ing* |
| `^-D` | -ed | *walk-ed* |
| `^-S` | -s | *cat-s* (unvoiced) |
| `^-Z` | -s | *dog-s* (voiced; same written suffix as `^-S`, a second stroke for whichever sound the word actually ends in) |
| `^-L` | -ly | *slow-ly* |
| `^-R` | -er | *fast-er* |
| `^-PB` | -en | *wood-en* |
| `^-FT` | -est | *fast-est* |
| `^-PBS` | -ness | *dark-ness* |
| `^-PLT` | -ment | *pay-ment* |
| `^-GS` | -tion | *na-tion* |
| `^-BL` | -able | *read-able* |
| `^-FL` | -ful | *care-ful* (F, L) |
| `^-LS` | -less | *care-less* (L, S) |
| `^-PBT` | -ent | *differ-ent* (N, T — distinct from `-ment`'s `^-PLT`) |
| `^-LT` | -let | *book-let*, *pig-let* (L, T) |
| `^*T` | -th | *wid-th*, *warm-th*, *four-th* (the `*T` coda) |
| `^-FPL` | -some | *awe-some*, *lone-some* (the same S-M → `-FPL` cluster Plover's own F-insertion convention uses, e.g. `lesson` HR-FPB) |

## Suffixes that kept their vowel (dropping it would collide with a stroke above)

| Stroke | Suffix | Why the vowel stayed |
|---|---|---|
| `^US` | -ous | reduced to bare `S`, already `^-S`'s own stroke ("s") |
| `^EUFT` | -ist | reduced to bare `S T`, already `^-FT`'s own stroke ("est") |
| `^UPBS` | -ance/-ence | reduced to bare `N S`, already `^-PBS`'s own stroke ("ness") — the identical unstressed sound covers both spellings |

## Suffixes with a real vowel that was never reducible

| Stroke | Suffix | Note |
|---|---|---|
| `^-F` | -ive | IH + V reduced to bare `-F`, unclaimed |
| `^AOEUZ` | -ize | AY is outside `[reduce]`, nothing to drop |
| `^SH-P` | -ship | SH + IH + P reduced to bare `SH-P`, unclaimed |
| `^HAOD` | -hood | UH is outside `[reduce]`, nothing to drop |
| `^WORD` | -ward | AO is outside `[reduce]`; happens to spell "WORD" — unintentional, but memorable |
| `^AOE` | -y | bare IY kept — only the `-ly` spelling of IY is in `[reduce]`, not plain IY |
| `^-PBLG` | -age | IH + JH reduced to bare `-PBLG`, the same cluster *village*'s own "-age" ending already uses |

## Two-syllable suffixes (two continuation strokes)

| Stroke | Suffix | Note |
|---|---|---|
| `^EUZ/^-PL` | -ism | IH + Z reduced (bare `Z`, already `^-Z`'s stroke), then AH + M reduced (bare `-PL`, unclaimed) |
| `^-R/^AOE` | -ary/-ery/-ory | phonetically exactly the already-claimed `-er` stroke followed by the already-claimed `-y` stroke, two syllables this table had already worked out alone |

## Not yet covered

A suffix with a real vowel that never reduces away and isn't in the table above yet — `-tive`,
further `-ous` spelling variants, `-ic`/`-ical`, `-al`, `-logy`/`-graphy`, `-ity`/`-ety` — needs a
stroke built the way a stressed syllable is, not a bare continuation of consonants or a syllable
run through `[reduce]`. Further work; see [the theory page](theory.md) for how a stressed
syllable is normally handled.
