# Syllables & folding

[← back to the table of contents](README.md) · [← back to the main README](../README.md)

**Several syllables.** An unstressed syllable with a reduced vowel is not written: its
consonants join the stroke before it, in steno order (*living* HREUFG, *special* SPERBL),
except that a lone `-er` or `-ly` may land out of order as in Plover (*better* PWERT,
*lately* HRAEULT, *color* KORL). When they don't fit, the stem keeps the consonant it owns
and the rest is a continuation stroke of consonants only on the right hand — this is the
**[join key](join-key.md)** at work: *ended* EPBD/+-D, *breaking* PWRAEUBG/+-G, *goodness*
TKPWAOD/+-PBS, *harder* HARD/+-R, *confidence* KOPB/+-FD/+-PBS. So the suffixes words fold onto
are a fixed set that never needs memorising apart from the rules, and `rules/suffixes.toml` gives
every one of them a stroke on its own too, Plover-suffix-dictionary style, so it can be added to
a stem that never happened to need it while the dictionary was built — see
**[the full suffix table](suffixes.md)** for every stroke this gives, old and new. Final `-ly` is
`+PHREU` after a cluster and a double `l` (*silly* SEUL/+EU); any other final `y` is `+EU`, and a
final `a` is a syllable of its own (*china* KHAOEU/+TPHA). Suffix clusters both dictionaries use
are rules: `-ment` -PLT, `-tion` -GS, `-ting` -GT, `-ding` -GD, and Plover's F for an S before
another consonant (*castle* KAFL, *lesson* HREFPB).

**Backup entries: a shorter word's own outline, plus whatever is left over.** This is where [the
design principle stated up front](README.md) — one single-stroke outline per word, but unlimited
multi-stroke ones judged on predictability rather than brevity — turns into actual generated
entries. The fold above only
ever happens to keep a word consistent with a shorter relative when cramming everything onto one
stroke isn't physically possible — *breaking* keeps *break*'s own `PWRAEUBG` only because `PWRAEUBG`
plus a second "k" has nowhere to go; *building* has no such excuse, so its own cheapest outline is
the fused `PWEULGD`, unrelated to *build*'s own `PWEULD`. A separate, later pass in `generate()`
fixes this on purpose instead of by accident: for every word, if its last syllable is a reduced
syllable that could be struck as a standalone continuation stroke on its own ([`Rules::suffix_split`
in `src/rules.rs`]), and the sounds left over exactly match some other already-placed word's own
pronunciation, that word's own real outline plus the continuation stroke is added as a *second*,
additional entry — `build`'s `PWEULD` plus `+-G` also reaches *building*, whether or not that agrees
with *building*'s own cheapest outline above. This never changes which outline wins as a word's own
primary (shown by `explain` and used everywhere else); it only ever adds an extra way in, so a
learner who knows the stem and the ending can always reach the word that way too.

The same pass also runs the other way, for a leading sound in front of a shorter word's own
*exact* pronunciation rather than a trailing one (`never` = the extra sound "N" in front of
*ever*'s own exact sounds `EFR`). Since the join key only ever continues a stroke forward, this
direction has to land on the *same* stroke as the matched word's own first one — the leading
sound's own keys folded onto it, the same way a prefix with its own fixed keys already does (see
[prefixes](prefixes.md)). When the leading sound's keys collide with one the matched word's first
stroke already uses, this can't rescue the word at all (`unclear` needs `un-`'s own `TPH`, which
shares an `H` with *clear*'s own "cl" onset cluster `KHR`) — skipped, not forced, a real structural
limit of a theory where nothing can precede a stroke, only follow one.

Neither direction requires the match to *mean* anything: `doctor`'s own sounds happen to be exactly
`docked`'s plus an "-er", and `crash`'s are exactly a "k" in front of `rash`'s — coincidences with
no relation to either word's own sense, reused anyway, the same way a brief never needed one either
(see [the main README](../README.md#where-lyk-stands)). When more than one already-placed word
ties on sound alone (`build` and `billed` are exact homophones), the word's own *spelling* breaks
the tie if it can — `building` starts with `build`, not `billed` — the same way spelling already
settles a tie between words that sound alike elsewhere in this theory (see
[spelling disambiguation](spelling.md)); if spelling doesn't settle it either, the word is left
alone rather than guessed at. A collision with an outline something else already has is skipped and
counted (`build`'s own report line), never a hard failure, since this is a best-effort pass over
the whole dictionary rather than an explicit, human-authored override.

Next: **[when two words sound the same](collisions.md)**.
