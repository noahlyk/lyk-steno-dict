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
| `+-G` | -ing | *walk-ing* |
| `+-D` | -ed | *walk-ed* |
| `+-S` | -s | *cat-s* (unvoiced) |
| `+-Z` | -s | *dog-s* (voiced; same written suffix as `+-S`, a second stroke for whichever sound the word actually ends in) |
| `+-L` | -ly | *slow-ly* |
| `+-R` | -er | *fast-er* |
| `+-PB` | -en | *wood-en* |
| `+-FT` | -est | *fast-est* |
| `+-PBS` | -ness | *dark-ness* |
| `+-PLT` | -ment | *pay-ment* |
| `+-GS` | -tion | *na-tion* |
| `+-BL` | -able | *read-able* |
| `+-FL` | -ful | *care-ful* (F, L) |
| `+-LS` | -less | *care-less* (L, S) |
| `+-PBT` | -ent | *differ-ent* (N, T — distinct from `-ment`'s `+-PLT`) |
| `+-LT` | -let | *book-let*, *pig-let* (L, T) |
| `+*T` | -th | *wid-th*, *warm-th*, *four-th* (the `*T` coda) |
| `+-FPL` | -some | *awe-some*, *lone-some* (the same S-M → `-FPL` cluster Plover's own F-insertion convention uses, e.g. `lesson` HR-FPB) |
| `+-BG` | -ic | *bas-ic*, *com-ic*, *mus-ic*, *tox-ic* (no onset of its own — the stem always keeps the consonant before it) |
| `+T-F` | -tive | *ac-tive*, *posi-tive* (T onset plus the same reduced IH + V `-ive` below already uses) |

## Suffixes that kept their vowel (dropping it would collide with a stroke above)

| Stroke | Suffix | Why the vowel stayed |
|---|---|---|
| `+US` | -ous | reduced to bare `S`, already `+-S`'s own stroke ("s") |
| `+EUFT` | -ist | reduced to bare `S T`, already `+-FT`'s own stroke ("est") |
| `+UPBS` | -ance/-ence | reduced to bare `N S`, already `+-PBS`'s own stroke ("ness") — the identical unstressed sound covers both spellings |
| `+UL` | -al | reduced to bare `L`, already `+-L`'s own stroke ("ly"); no onset of its own either (*national*'s own "-al", once "-tion" has eaten the N, is exactly this bare AH + L) |

## Suffixes with a real vowel that was never reducible

| Stroke | Suffix | Note |
|---|---|---|
| `+-F` | -ive | IH + V reduced to bare `-F`, unclaimed |
| `+AOEUZ` | -ize | AY is outside `[reduce]`, nothing to drop |
| `+SH-P` | -ship | SH + IH + P reduced to bare `SH-P`, unclaimed |
| `+HAOD` | -hood | UH is outside `[reduce]`, nothing to drop |
| `+WORD` | -ward | AO is outside `[reduce]`; happens to spell "WORD" — unintentional, but memorable |
| `+AOE` | -y | bare IY kept — only the `-ly` spelling of IY is in `[reduce]`, not plain IY |
| `+-PBLG` | -age | IH + JH reduced to bare `-PBLG`, the same cluster *village*'s own "-age" ending already uses |

## Two-syllable suffixes (two continuation strokes)

| Stroke | Suffix | Note |
|---|---|---|
| `+EUZ/+-PL` | -ism | IH + Z reduced (bare `Z`, already `+-Z`'s stroke), then AH + M reduced (bare `-PL`, unclaimed) |
| `+-R/+AOE` | -ary/-ery/-ory | phonetically exactly the already-claimed `-er` stroke followed by the already-claimed `-y` stroke, two syllables this table had already worked out alone |
| `+-BG/+UL` | -ical | exactly `-ic` then `-al` above, struck one after the other — the same two already-claimed pieces |
| `+OL/+SKWREU` | -logy | *bi-ology*, *ge-ology*, *psych-ology* — a genuinely stressed syllable (AA is never in `[reduce]`, so it keeps its vowel the way any stressed syllable does, coda L kept with it), then "-gy" (JH + the plain, never-reduced IY `+AOE` already uses for "-y") |
| `+U/+TEU` | -ity/-ety | *clar-ity*, *activ-ity*, *anxi-ety*, *soci-ety* — a bare AH with no consonants to drop (keeps its vowel out of necessity, not collision), then T + the "-y" vowel kept. Plain "-ty" with no leading schwa (*safe-ty*) is just the second stroke alone |
| `+U/+SEU` | -cy/-sy | *priva-cy*, *fanta-sy*, *poli-cy*, *democra-cy* — the same bare-AH first stroke as "-ity" (two different suffixes may share an opening stroke; only the whole outline has to stay distinct), then S + the "-y" vowel kept |
| `+U/+TPAOEU` | -ify/-fy | *clar-ify*, *simpl-ify*, *ident-ify* — the same bare-AH first stroke again, then F + AY kept (AY never reduces, same as "-ize") |

## Backup stem+suffix entries

This table isn't only a reference for striking a suffix on its own — a later pass in `generate()`
uses it, in spirit, to add a *second* dictionary entry wherever a word's own trailing syllable
could be struck as a standalone continuation stroke and the sounds in front of it exactly match
some other already-placed word's own pronunciation: `build`'s own outline plus `+-G` also reaches
*building*, whether or not that agrees with *building*'s own cheapest outline. See
**[syllables & folding](syllables.md)** for the full rule, both directions it runs in, and its
known limits. Only **single-stroke** suffixes above take part in this — the two-syllable table
just above (`-ism`, `-ary`/`-ery`/`-ory`, `-ical`, `-logy`, `-ity`/`-ety`, `-cy`/`-sy`,
`-ify`/`-fy`) isn't matched by it yet, since recognising a word's last *two* syllables together as
one of these needs its own check, not a straightforward extension of the one-syllable case.

## Not yet covered

- **`-ious`/`-eous`** (*curious*, *serious*, *various*, *obvious*, *previous*, *hideous*) already
  write with strokes this table has: the extra vowel before "-ous" is the same bare, never-reduced
  IY the "-y" row above uses, so the whole ending is just `+AOE/+US` struck in sequence — no new
  entry needed. Where there's no extra vowel (*gorgeous*, *courageous*, *religious*), it's plain
  "-ous", `+US`, on its own.
- **`-graphy`/`-ography`** (*biography*, *photography*, *geography*, *calligraphy*,
  *choreography*) is not here: the schwa right before "-graphy" (the "ra" of "biogra-phy") takes a
  vowel key that depends on incidental spelling elsewhere in the word (`AH` vs the spelled `AH.o`
  variant — confirmed by comparing the real generator's own continuation strokes for several of
  these words), not a fixed property of the suffix the way every other entry in this file is. A
  standalone stroke can't know that in advance, so this one stays per-word, through the normal
  rules, rather than a stroke here.

Next: **[punctuation & commands](punctuation.md)** — the reserved strokes pinned ahead of every
word, now with the actual content instead of just the structural mention.
