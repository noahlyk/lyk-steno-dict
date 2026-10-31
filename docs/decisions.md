# Decisions

[← back to the table of contents](README.md) · [← back to the main README](../README.md)

This is not a learner's chapter — it's a maintainer's log. The chapters in the table of contents
explain how the theory works *now*; this explains why a run of real decisions this session landed
the way they did, so nobody (human or AI) reverts one by accident without knowing it was
deliberate. Each entry: what changed, the concrete problem that forced it, and what regresses if
undone. In commit order.

**Join key renamed `^` → `+` (`38f11c8`).** Pure notation, no behavior change — `^` collided
visually with nothing in particular, `+` reads better next to Plover's own `*`. If this gets
reverted on its own, every doc, every `rules/*.toml` entry, and the plugin template's own parser
all have to move together or the dictionary and the deployed plugin disagree on notation.

**Backup entries: a shorter word's own outline, plus whatever's left over (`f8687f2`, later
rewritten by `ada24ff`).** `build` → `PWEULD`, `building` → the unrelated-looking fused `PWEULGD`,
because every word's outline was derived from its own pronunciation alone — nothing ever checked
"is there already a shorter placed word to build this one from instead." Fixed with a second,
additive pass: for every word, try every split of its own sounds against three composition kinds
(a trailing reduced syllable as a suffix continuation, a leading consonant cluster folded onto a
placed word's first stroke, two whole placed words back to back), and add every safe match as an
extra entry — `build`'s `PWEULD` plus `+-G` also reaches `building`, without touching `building`'s
own primary outline at all. Runs to a fixed point so a backup can chain through another backup:
`green`+`house`→`greenhouse`, then `greenhouse`+`+-Z`→`greenhouses`, with neither needing its own
primary outline to look like that. See [syllables & folding](syllables.md) for the full mechanism.
**Never revert this to "first match wins"** — the whole point of trying every split and running to
a fixed point was that the first-match version missed chained compounds and left real words
(`buildings`) without a sane way in. Deliberately *not* extended to two-syllable suffixes (`-ism`,
`-ity`, `-ary`, `-ical`, `-logy`, `-cy`, `-ify`) — the real derivation needs a consonant-migration
pattern (how the stem's own ending consonant shifts before the suffix lands) too irregular to
guess safely without real design work, not just effort. Don't paper over that gap with a guess.

**One-stroke-per-word, stated as policy (`12927ec`).** A word gets *exactly one* single-stroke
outline — no `NOT`/`OT`/`PHOT`-style competition for the same word — but unlimited `+`-joined
multi-stroke backups, judged on predictability (does it decompose into pieces a learner already
knows) rather than brevity. This was always true of the single-stroke machinery by construction;
what changed is making it an explicit, written invariant so a future change doesn't casually add a
second single-stroke way to reach a word and call it a feature. If you're ever tempted to add an
alternate single-stroke spelling for a word "because it's reasonable too" — don't; that's the
`NOT`/`OT`/`PHOT` problem by definition.

**The `cat`/`captain` fragment bug (`57d99d7`).** `cat` was losing its own natural `KAT` to
`captain` — a word that's actually *less* frequent in the corpus's own ranking here, but got
there first because `matches_real_consonants`/`drop_fragments` only checked a word's first and
last syllable when deciding whether a sound counted as a "fragment" to drop. `cat`'s own `K-A-T`
happened to overlap bit-for-bit with a coda cluster from an *earlier* syllable of `captain` that
gets dropped entirely, so the fragment check falsely flagged `cat`'s legitimate, complete sound as
someone else's leftover. Fixed by checking a syllable's own full structure, not just the
word's endpoints. If this regresses, it won't look like "captain changed" — it'll look like some
unrelated short, common word losing its natural outline to a longer word that merely happens to
contain its sounds as a sub-fragment. Watch for that shape of bug, not just this one instance.

**Shrimple-style spelling mode (`5f1dd71`).** Some words (`stenography`) have no CMUdict
pronunciation at all, so the rules engine has nothing to generate a stroke from — the phonetic
fallback used to garble these since it only knows how to read keys back as *sounds*, not letters.
Added a second marker bit (`~`, alongside the join key's `+`) that switches a run into letter-exact
spelling: each stroke after the marker reads back as one literal letter from `rules/spelling.toml`,
not a guessed sound. This reuses the join key's own "whole run handed to the plugin at once"
mechanism — no new keymux engine state needed. See [spelling mode](spelling-mode.md).

**Suffix VIP fallback (`ff01ea4`).** A suffix stroke (`+-D`, `+-G`, …) only read as its real
suffix text when some specific *combined* outline happened to already be a literal dictionary
entry — struck after a stem with no matching entry, it read as a bare, vowelless consonant guess
instead (`+-D` → `"d"`, not `"ed"`). This defeated the entire point of giving suffixes their own
reserved strokes in the first place: a suffix is supposed to be struck-and-trust, unconditionally,
the same way reserved commands and punctuation are. Fixed by having the phonetic fallback
(`Rules::resolve`, mirrored in the generated plugin) check `rules/suffixes.toml`'s own chords
*first*, for any stroke after the first that carries the join key, before ever falling through to
a generic phonetic guess. **This must stay unconditional.** Reintroducing any "only if it's a known
combination" gate here reopens the exact bug — a suffix silently working sometimes and not others
depending on what it's attached to, which is the single complaint this fix exists to kill.

**Continuation stem reads its real word, not raw phonemes (`f44bfe2`).** Even with the suffix fix
above, a continuation run whose *stem* (the first stroke) wasn't backed by a literal combined
dictionary entry still got re-derived from scratch as raw phonemes — including stems that are
themselves perfectly good, already-placed words. `SKP` is "and"'s own brief; nothing about
`S-K-P` spells "and" phonetically, so `SKP`+`+-R` read back as `"sper"` (`SKP` decoded as raw
onset consonants, with "-er" correctly appended to the wrong base entirely). Fixed by having
`resolve` check an already-placed single-stroke word first for the *first* stroke of an unmatched
run, using its real text instead of re-guessing: `SKP/+-R` now reads `"ander"`, `PWHR/+-D` reads
`"builded"` instead of `"mred"`. Scoped to a single-stroke stem for now — a multi-stroke placed
word as a run's own stem is a known, not-yet-covered extension, not a decision to leave it out
forever. See [the fallback](fallback.md) for the exact mechanics.

**keymux's own plugin-fallback layout bug (keymux commit `5a4460f`, sibling repo).** Found while
chasing a live report that `+` sometimes "made a separate word." Root cause lived in keymux, not
lyk: `Chain::lookup` rendered the stroke window it hands a plugin's fallback export using a
hardcoded default 23-key Plover layout, not the `Layout` the chain was actually built with. Any
extra key a theory defines — lyk's `+`, or any other theory's own join/spelling/whatever key — has
no name in that default layout, so the free-standing `render_strokes` call silently dropped it
before the plugin ever saw it: the plugin had no way to tell a continuation stroke from a fresh one
once dictionary lookup missed, because the one bit that said so was already gone from the text by
the time it got there. Fixed by having `Chain` keep the `Layout` it loaded with and render with
that instead of the hardcoded default. **This fix is generic, not a lyk accommodation** — it
affects any theory with any extra key, which was the explicit condition on making it at all (see
[the join key](join-key.md)'s closing note). It landed on a detached HEAD in the keymux repo and
still needs to be placed on a real branch — if that commit gets lost, every theory with a custom
extra key (not just lyk) silently regresses to this exact bug the moment its dictionary misses.

**CLI default corpus size matched to what's actually deployed (`1f26529`).** `quote`, `resolve`,
`lookup`, and the standalone `plugin` command all defaulted to a smaller word count (20k-30k) than
`build`/`deploy`'s 100k. A mid-frequency word only earns its full brevity treatment once the
generator has placed enough higher-ranked words to know what's still free, so a smaller ad-hoc
build can show a strictly worse outline than what's actually shipped — confirmed concretely with
`quitters`: a 2-stroke backup composition at the old 30k default, a clean 1-stroke `KWEURTZ`
(matching Lapwing's own stroke count) at the real 100k the deployed dictionary uses. Not an engine
bug — a diagnostic tool quietly demonstrating a weaker, un-shipped build of itself. Fixed by
matching every CLI default to 100k. **If these defaults drift apart again, every stand-alone
`quote`/`resolve`/`lookup` run becomes a false demo of lyk's own quality**, understating exactly
the terseness comparisons the whole project is judged by.

**The "stranded outline" investigation turned out smaller than it looked (`ae725e4`).** `take`
sitting on a starred `TA*EUBG` while the plain `TAEUBG` sat unclaimed was the motivating report -
and it was a false alarm. `TA*EUBG` isn't `take`'s own primary at all: it's a `Backup` entry
(`T` + the rare word `ake`'s own starred outline, itself a genuine, permanent collision against
`make`), and `generate::cheapest` already correctly picks `take`'s real primary, the clean 5-key
brief `TAEBG` - the confusion was entirely the next entry's bug (`lookup`/`explain` showing the
wrong thing). A real, much smaller issue *did* turn up while checking: ten words (`wow`, `honest`,
`bottle`, `public`, `hiding`, `gasps`, `flowers`, `grunting`, `honey`, `bloody`) sit on a brief
that costs more than their own plain-rules reading, which is sitting completely free. An automatic
fix was tried - first a blanket "use whichever is cheaper," then a narrower version that only
strips a candidate's own gratuitous star - and **both were reverted**. The blanket version
relitigated briefs as foundational as `a`'s own `AEU` against an unrelated pronunciation's shorter
reading; even the narrow, star-only version rippled into several hundred other words' outlines,
some worse off, because this generator settles every word once, commonest-first, with nothing
revisited - changing even one early word reshuffles everything after it. That ripple is an
accepted cost for a human adding a `[word]` override, not for code deciding on its own. What
shipped instead: a standing count, printed by `build` as "words stranded on a worse outline than
their own, unclaimed plain reading," surfacing exactly these ten for a maintainer to fix by hand
with `[word]` the normal way. **Don't revive the automatic version** without first finding a
comparison that can't reach past the specific word it's meant to help - "cheaper" alone isn't it.

**`lookup`/`explain` now read the real dictionary (`62a2dae`).** Both called the raw phonetic
candidate computation directly (`setup.candidates(...)`), bypassing `generate()`'s full collision
resolution, briefs, and backup entries entirely, and never marking continuation strokes with the
join key either - `lookup quitters` showed a garbled, unjoined `KWEUT/-RZ` even though the real,
deployed dictionary has the clean, single-stroke `KWEURTZ`. This is also what made the `take`
investigation above take as long as it did: the tool being used to check the generator's output
was itself the thing lying. Fixed by routing `lookup`'s word/phrase path through
`generate::generate` + `generate::cheapest`, same as `quote`/`resolve` already do, falling back to
the raw candidate only for a word that never made it into the generated dictionary at all.
`explain` keeps its existing `ours` line exactly as it was on purpose - it's a deliberately
rules-only view, for seeing how the theory derives a word step by step - and gains a new `deployed`
line next to it showing the real answer, rather than conflating the two. **If `ours` and
`deployed` ever get merged back into one line, this exact confusion comes back**: a rules-only
derivation is not the same question as "what does this word actually stroke as," and both answers
are worth keeping visible, not just one.

**Spelling mode gained an encoder (`b6e1107`, `36efc64`).** It could only ever decode: `resolve`
read a hand-written `~`/`+` letter-chord outline back into a word, but nothing went the other way
- a word with no pronunciation at all (`stenography`, not in CMUdict) had to be spelled out by
hand, letter by letter, against `rules/spelling.toml`, by eye, before `resolve` could even confirm
it. `Rules::spell_word` is `resolve_spelling`'s exact inverse: given the word, it produces that
outline directly. Wired into `quote`'s fallback chain, then into `lookup`'s `reverse_word` and
`explain`'s no-pronunciation branch once the concurrent `lookup`/`explain` accuracy fix above
landed and touched exactly those two functions - deliberately sequenced to avoid editing the same
code at the same time as that fix. **Does not need porting to keymux**: keymux only ever decodes
strokes someone actually presses (already true for spelling mode, via the generated plugin's
letter table), it never needs to compute what to press for a word - that's purely a lyk-side
authoring/lookup concern.

## Two rules that override any of the above

These were stated repeatedly and are not up for silent renegotiation by a future change:

**No semantic, fuzzy, or embedding-based matching anywhere in dictionary generation.** Only exact
phonetic containment (is one word's pronunciation exactly a prefix/suffix of another's) and exact
spelling-boundary tie-breaks. Phonetic containment has a symbolic ground truth already sitting in
the pronunciation data — there's nothing to approximate, so a learned similarity score would only
add a miscalibrated threshold and a new source of false positives that exact matching can't
produce by construction. It would also break the project's central requirement, that a learner can
read a finite rule and predict a stroke: "related because this exact sound sequence matches" is
checkable by eye forever; "related because a trained vector's similarity exceeded some cutoff" is
neither learnable nor stable. If a future change wants embeddings or cosine similarity for
*anything* in this pipeline, that is a reversal of an explicit, considered decision, not a neutral
addition — raise it as one.

**Exactly one single-stroke outline per word; unlimited, predictable `+`-joined multi-stroke
backups.** Restated here because it's the invariant every backup-generation decision above exists
to protect without ever touching. See the one-stroke-per-word entry above and
[the table of contents](README.md)'s own opening paragraph for the canonical wording.
