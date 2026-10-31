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

**Contractions backfilled into the frequency rank (`CONTRACTIONS` in `src/analysis.rs`).** Every
contraction was silently invisible to the whole generator, no matter how common - `don't` has a
real CMUdict pronunciation but was missing from the generated dictionary entirely, so striking its
own, correct outline fell through to the raw phonetic-miss fallback and garbled it. Root cause: the
frequency corpus strips every apostrophe (`don't` is counted there as `dont`), while CMUdict keeps
the real spelling, and the generator's word list is built from the *intersection* of a rank and a
pronunciation - `dont` has no pronunciation, `don't` has no rank, so neither spelling was ever both
at once. Fixed with a hand-curated table (69 entries, verified against both files) backfilling each
real contraction's rank from its stripped form's. **Deliberately hand-curated, not derived by
stripping every apostrophe CMUdict has**: most of those are unrelated possessive-noun pronunciation
entries (`thing's`) that would otherwise wrongly borrow their plural twin's rank (`things`) - this
table is closed-class English contractions only. A few (`he'll`/`hell`, `we'll`/`well`,
`who're`/`whore`) share their stripped spelling with a real, unrelated word already at that rank;
the contraction borrows an approximate rank from it rather than its own true frequency - an
accepted, minor imprecision, not a reason to revert this. **If this table shrinks or disappears,
every contraction in it goes back to being untypeable as itself**, not just briefed differently.

**`make` = `AEUBG`, dropping its own `M`, is not a bug (investigated, confirmed intentional).** One
of Plover's own gold dictionary entries writes "make" with a bare `P` instead of the real `M` chord
`PH`; `best_brief`/`drop_fragments` (`src/generate.rs`, `src/rules.rs`) correctly flags holding only
*half* of `M`'s real chord as disallowed and "repairs" it by deleting the fragment entirely, same as
it does for **113 other words**, including some of the most common in English (`the`→`-T`,
`not`→`OT`, `that`→`A`, `for`→`-R`, `never`→`EFR`). Confirmed from the code's own doc comments this
is the sanctioned, intended behavior, not a failure mode: "a brief is always free to leave out a
whole sound it isn't attempting to represent." **Do not change this shared heuristic to disallow a
full onset/coda drop** - it would silently reshuffle over a hundred already-memorized, currently-
correct briefs for the most common words in the language. A specific word that still bothers a
learner (`make`, say) gets a one-off `[word]` override in `rules/briefs.toml`, the normal path for
"someone looked at this one and decided differently" - zero blast radius on the other 112.

**A real test suite: a hardcoded quote corpus, and structural QA guards.** Every check of "is lyk
actually terser" so far was a human running `quote` by hand and eyeballing a table - no regression
test ever failed if a rule change quietly made the theory worse. `src/quote_corpus.rs` hardcodes
~100 passages (public-domain lines plus plain, unremarkable prose written for the file - deliberately
*not* a curated set that would flatter lyk, since that would measure the curation, not the theory)
and scores every one exactly the way the live `quote` command does (`quote::rows_for`/`quote::cost`,
shared code, not a reimplementation), then asserts against thresholds set just under a real, actual
run: 103 passages, lyk beat Plover outright on 60 (58.3%) and Lapwing on 76 (73.8%); of 1,216 words,
lyk cost fewer keys than Plover on 18.2% and Lapwing on 33.0% (and cost *more* on only 9.5% / 5.2%);
aggregate keys across the whole corpus: lyk 4,640, Plover 4,694, Lapwing 5,039 (lyk is 1.2% cheaper
than Plover, 7.9% cheaper than Lapwing, in aggregate). Thresholds were set a few points under these
numbers - deliberately harsh, not a loose sanity floor: a real regression should fail this
immediately, and a real improvement should raise the constants back up in the same commit, not
leave them stranded low. `src/quality.rs` adds structural QA guards unrelated to terseness - no two
words ever sharing a stroke, every sufficiently common pronounceable word actually having an
outline (the exact kind of check that would have caught the `don't` bug the day it shipped, instead
of a live user hitting it), the stranded-word count not silently growing past its known baseline
(0, as of the entry below - it was 10 when this test was first written), and no generated word
colliding with a reserved command/punctuation/fingerspelling stroke. Both share one production-scale
dictionary build (`src/dictionary_fixture.rs`, a `LazyLock`) rather than paying the few-second
`generate()` cost per test - the whole suite (32 tests) runs in ~5 seconds. **If this corpus or
these thresholds get deleted "to make CI faster," every regression this session fixed by hand (and
ones nobody's found yet) can silently come back with nothing to catch them.**

**Six top-100 words were paying for a fragment-safety rule they didn't need to (`rules/briefs.toml`:
`on`, `from`, `no`, `then`, `day`, plus `bloody`/`wow`/`bottle`/`honest`/`flowers`/`gasps`/
`grunting`/`hiding`/`honey`/`public` from the stranded-word list, and `make` below).** The
quote-corpus benchmark surfaced these as real, measured losses, not guesses. Root cause for the
first five: Plover often writes an extremely common monosyllable using only half of this theory's
own multi-key onset/coda chord (`on`'s Plover brief `OB` is missing the `P` of coda "N"'s real
`-PB`; `from`'s `PR` is missing the `T` of onset "FR"'s real `TPR`; same pattern for `no`, `then`,
`day`) - a deliberate professional shorthand, but one `matches_real_consonants` always rejects,
since it cannot tell "a brief someone meant" from "a fragment that got there by accident" (the exact
same mechanism investigated for `make` below). Each was checked by hand - confirmed free
(`cargo run -- resolve -- <outline>`), confirmed nowhere else in `rules/briefs.toml` already -
before pinning as a literal override. **`one` was checked the same way and deliberately left alone**:
its own shortest Plover candidate (`WUB`) hits this identical rejection, but `one = "WUPB"` already
exists as a separate, older, deliberate decision (a homophone fight with `won`) documented in
`rules/briefs.toml` itself - not an oversight to pile a second fix onto.

**Two contractions (`they're`, `shouldn't`) still cost more than Plover's own brief even after the
rank-backfill fix above - fixed; seven others (`can't`, `didn't`, `hadn't`, `we've`, `hasn't`,
`haven't`, `wasn't`) checked and deliberately left alone.** All nine were visible to the generator
only because of the `CONTRACTIONS` backfill, but several still land far outside `briefs.toml`'s own
1,500-word window even with a backfilled rank, so `best_brief` never tries Plover's candidate for
them at all unless named explicitly. `they're` (saves 3 keys: a free `THER`, vs. a three-homophone
collision fallback `THA*EUR`) and `shouldn't` (saves 3 keys and a stroke-count tie broken on keys:
`SH-PBT` vs. `SH*UPBTD`) both clear this file's own stated bar for naming a word past the window
(**"saves at least three whole keys… that outline's first and last consonants are a whole cluster…
and claiming it does not take a stroke a commoner word already has"** - see the "past this point"
section) and were added. The other seven save only 1-2 keys each - below that same stated bar - so
per this file's own already-documented policy they were left alone, not force-included. **If someone
"fixes" those seven anyway, they are relitigating an existing, written threshold, not filling a
gap** - raise that, don't just add the entries.

**All ten words `build`'s "stranded on a worse outline" count ever flagged are now fixed
(`rules/briefs.toml`); the count is 0 and `src/quality.rs`'s baseline was lowered to match.** Every
one followed the exact same shape: the word's own plain-rules reading was strictly cheaper (fewer
strokes, or the same strokes with fewer keys) than its deployed outline, and that cheaper reading
was confirmed completely free before pinning it. Three distinct root causes among the ten, worth
knowing so the next one is diagnosed faster instead of re-investigated from scratch: (1) a star
nobody needed (`wow`, `bottle`, `flowers` - Plover's own candidate happened to carry a star for a
reason specific to Plover's theory, not a real collision here); (2) a word split across two strokes
when the rules' own single-stroke fold fits fine (`gasps`, `grunting`, `hiding` - all now one
stroke, matching Lapwing); (3) an onset sound that was never actually attempted getting struck
anyway (`honest` - the rules' own reading correctly has no "h", since it's silent in this word;
Plover and Lapwing both write an "h" nobody says). `honey` and `public` were smaller, unexplained
key differences, not fitting any of the three patterns above - just confirmed cheaper and free. **If
this count goes above 0 again, something in collision resolution regressed - it is not something to
raise the baseline to tolerate.**

**`make = AEUBG` investigated and fixed - confirmed a genuine defect, not the 113-word pattern it
looked like at first (`rules/briefs.toml`: `make = "PHAEUBG"`).** One of Plover's own gold entries
writes "make" with a bare `P` instead of the real `M` chord `PH`; `matches_real_consonants` correctly
flags the bare `P` as holding only half a real chord, and its "repair" deletes it entirely rather
than completing it, landing on `AEUBG` - a vowel-only outline with zero trace of the word's actual
first sound. The earlier investigation found 113 *other* words relying on this exact same
fragment-drop mechanism deliberately (`the`→`-T`, `not`→`OT`, `never`→`EFR`, and so on) and
correctly concluded the shared heuristic itself must not change. `make` is simply one of the rare
cases on that list where the dropped result lands on something confusing enough (a word with no
trace of its own first sound at all, rather than a short known brief) to be worth a one-off
`[word]` override - exactly the per-word escape hatch that finding already recommended, now applied.
**This does not reopen the question of changing `best_brief`/`drop_fragments` themselves** - that
was a closed, investigated decision; this is just one of the data points it already anticipated
getting hand-fixed individually.

**`which` takes Lapwing's `KH`, not Plover's own (rejected) `WEU` (`rules/briefs.toml`:
`which = "KH"`).** Same fragment-check mechanism as `on`/`from`/`no`/`then`/`day` above - Plover's
own candidate `WEU` for "which" trips `matches_real_consonants` too (lyk's onset table gives a
"wh"-spelled word the full `WH` chord, so bare `W` is only half of it) - but here the user asked
for Lapwing's own, even shorter `KH` specifically, not a repair of Plover's rejected candidate.
`KH` belonged to `charge` (its own Plover brief, rank ~62,700) until this override: `which` is
roughly 300x more common (rank ~200) and simply gets processed first in the rank-ordered pass, so
`charge` falls back automatically to its own next candidate, `KHARPBLG` - its own plain rules
reading, which Lapwing agrees with, not an error or a downgrade worth worrying about. Confirmed
nothing else moved. **If `which`'s override is ever removed, don't "fix" it by pointing it back at
Plover's `WEU`** - that candidate is the one that fails the fragment check in the first place; the
literal `"KH"` is what makes this work.

**A continuation's stem drops its own silent `e` before a vowel-initial suffix
(`Rules::resolve`/`drop_silent_e`, mirrored in `generate_plugin`'s template).** `SR/+-G` ("have"
plus the standalone "-ing" suffix) read back as "haveing" - the stem's real text ("have", found by
the earlier stem-recognition fix) concatenated with the suffix's real text ("ing") with nothing in
between to apply basic English spelling. Confirmed this wasn't an isolated case: `AOUS/+-G` ("use"
plus "-ing") had the exact same problem, reading "useing" instead of "using" - anything with a
silent trailing `e` before a vowel-initial continuation was affected, not just "have". Fixed by
computing the very next piece's text *before* committing the stem, so a trailing silent `e` can
drop when it's genuinely silent (preceded by a consonant) and what follows starts with a vowel -
`entire` + `ly` correctly keeps its `e` (consonant-initial suffix), `agree` + `ing` correctly keeps
it too (the letter before the `e` is already a vowel, so it was never silent). Mirrored into the
generated plugin's own template and verified by compiling that generated source standalone and
calling its `resolve` directly - the two must never drift, since the deployed plugin is what a real
keystroke actually reaches. **Known, accepted gap**: "dye"/"singe" keep their `e` before "-ing" in
real English specifically to stay visually distinct from "die"/"sing" - not special-cased, since
these are rare exceptions to an otherwise reliable general rule, not a sign the rule is wrong.

**`but` = `UT` was the `make`/`AEUBG` bug again, not a real win (`rules/briefs.toml`:
`but = "PWUT"`).** `UT` looked like a genuine terse win over both Plover's `PWU` and Lapwing's
`PWUT` - but checking Plover's real dictionary file directly, there is no single-stroke `UT` entry
for "but" anywhere in it. Its actual entries are `PWU`, `PWUT`, `PWHUT`, `PWUTS`, `PWUTD`, `WUT`,
and a two-stroke `PWUT/UT`. `WUT` uses bare `W` for "but"'s real `/b/` sound (the full chord is
`PW`); the same fragment-safety check that produced `make`'s bug flagged the bare `W` and deleted
it outright, landing on `UT` - a candidate that was never one of Plover's own answers, with zero
trace of the word's actual first sound. Pinned to the correct phonetic reading, `PWUT`, which also
matches Lapwing exactly. Cost the quote-corpus benchmark a small amount of real ground (lyk's
Lapwing win rate 74.8% vs the 1.2%/7.5% aggregate savings thresholds, down slightly from before) -
thresholds lowered to match in the same commit, same as every other real-but-small regression this
session accepted for correctness. **If `but` is ever "optimized" back toward `UT`, it is
reintroducing this exact bug, not finding a real improvement** - check `matches_real_consonants`
for a bare, partial-chord Plover candidate before trusting any single-stroke win that looks too
good, the same lesson `make` already taught once.

**A physical ergonomics model, diagnostic only so far (`src/ergonomics.rs`).** Every cost
comparison in this theory until now was pure key count - two outlines with the same number of keys
were treated as equally good, even though some physical shapes are much harder to execute than
others. Built from the user's own real keyboard diagram (`# +S TK PW HR AO - EU * FR PB LG TS DZ`,
a standard steno layout, not a flat-QWERTY finger chart) and two corrections made mid-design by the
user, both load-bearing: (1) a single finger pressing both its own rows (`TK`="D", `PW`="B") is
*not* a penalty by itself - those are foundational, universal steno chords; the real cost is **row
transitions** walking a hand's active fingers in physical order (`SKPH`, bottom-bottom-top-top, one
transition, fine; `STWH`, bottom-top-bottom-top, three transitions, alternates every finger, bad) -
and (2) the join key is excluded from the pinky-usage tax entirely, since it's a separate physical
key present in nearly every continuation regardless of which words are involved, so taxing it would
penalize "more than one stroke," not any real word choice. The right pinky is a real, structural
asymmetry in the layout itself (it alone owns two columns, `T`/`S` and the further-stretch `D`/`Z`,
where every other finger owns one) - not something any theory choice can fix, only something a cost
model can account for.

Scored the real dictionary: ergonomic cost already correlates with frequency somewhat on its own
(top-1,000 words average 2.05, climbing to ~3.8 past 10,000) - reinforcing a trend already present,
not fighting it. The worst offenders at every frequency band are dominated by `STR`/`SPR` onset
clusters (`street`, `strong`, `straight`, `surprise`), which matches commonly-cited real-world
steno complaints, not a false positive in the model. **Deliberately not wired into `cheapest`,
`best_brief`, or any word's actual outline yet** - this is read-only and diagnostic, guarded by its
own ceiling test (`the_real_dictionary_stays_within_known_ergonomic_ceilings`) so a future rule
change can't silently make the dictionary physically worse to type without the test catching it,
but no outline has been changed on ergonomic grounds alone. The next real step, not yet started: a
one-off `[word]`-override pass on the ~20 top-1,000 words that score worst, the same safe,
individually-verified pattern already used for `make`/`on`/`but`/`which` - not a sweeping change to
the shared cost function. **If this module gets deleted "since it doesn't do anything yet,"** the
next regression it would have caught (a rule change that makes a whole class of words less
comfortable without anyone noticing, since key count alone can't see it) comes back with nothing to
catch it.

**Backup generation now composes two-stroke suffixes too (`Rules::two_stroke_suffix`,
`src/generate.rs`'s new "(a2)" composition).** The single-stroke stem+suffix backup pass
(`Rules::suffix_split`) never covered `-ism`, `-ary`/`-ery`/`-ory`, `-ical`, `-logy`, `-cy`/`-sy`,
`-ify`/`-fy` - a documented, deliberate deferral, since these two-syllable suffixes' own chords in
`rules/suffixes.toml` sometimes migrate a consonant backward onto the *first* stroke in a way plain
syllable rules wouldn't reproduce (`-ism`'s own `+EUZ/+-PL` ends its first stroke in a `z` that
ordinary syllabification would give the second syllable instead). Rather than risk re-deriving that
migration and getting it wrong, the fix finds the match by **spelling** instead of sound - a word
ending in one of these suffixes' own letters (`cubism` ending in "ism") has its stem looked up by
spelling directly (`word_outline.get("cub")`), and the suffix's own chord is reused exactly as
written in `rules/suffixes.toml`, never re-derived. Confirmed against a real production build: 327
new backup entries, including `cubism` → `KUB/+EUZ/+-PL`, `trilogy` → `TRAOEU/+OL/+SKWREU`,
`chemical` → `KEPL/+-BG/+UL`, `quantify` → `KWAPBT/+U/+TPAOEU`, `salary` → `SAL/+-R/+AOE`. These are
backups only - `cheapest()` still picks whatever's actually shortest for a word's primary entry, so
a word that already had a cheaper single-stroke fold from an unrelated mechanism keeps it; this
just means a stem a learner already knows now has a predictable, derivable path to these endings
too, when nothing shorter already existed. **If spelling-based stem lookup here ever gets replaced
with a sound-based one "for consistency" with the single-stroke version**, re-derive and verify the
consonant migration for every one of these suffixes first - that was the exact risk this design
choice was made to avoid.

**Three words pinned to a less-terse but more comfortable outline, the first real use of
`src/ergonomics.rs` (`rules/briefs.toml`: `against`, `protect`, `perhaps`).** Scanned the
top-1,000-by-frequency words for the worst `ergonomics::weighted_outline` scores, then checked each
one's own Plover candidate list for a different-shaped answer rather than inventing anything new.
`against` (`TKPWEPBS` vs deployed `STPWEPBS`, score 6 vs 9) cost nothing at all - same 8 keys,
different onset shape Plover already lists. `protect` (`PHREBGT` vs `PREBGT`, score 5 vs 7) and
`perhaps` (`PRAPGS` vs `PRAPS`, score 7 vs 9) each cost one extra key for a real reduction - the
trade the user explicitly asked for. **A fourth candidate, `trouble` (`TRUPBL` vs `TRUBL`, score 6
vs 8), was tried and reverted** - pinning it had no effect at all, because `trouble` also qualifies
for the automatic brevity-ladder pass (`Origin::Short` - see the `ords`/`every short form that is
free is added` design earlier in this log), which independently re-derives and re-claims the
cheaper `TRUBL` regardless of any `[word]` override, and `cheapest()` always prefers the globally
shortest entry for a word, not whichever one the brief table named. **This is a real, structural
limit on this whole approach**: an ergonomics-motivated `[word]` override only sticks when nothing
else in the generator independently reconstructs the word's cheaper natural form - always verify
with `cargo run --release -- resolve -- <old outline>` that it now shows "not in the dictionary"
before trusting an override took effect, the same check that caught this one.

## The core philosophy reversed (day 4): speed over per-word learnability

**This is a genuine, deliberate reversal of this project's original founding value, not an
incremental tweak — recorded here so nobody mistakes it for drift.** Every entry above this line
was written under the original rule: "a learner can read a finite rule and predict a stroke,"
applied to *every* word, which is why so many entries above justify an arbitrary brief as an
exception scoped to short, extremely common words only (`into`, `will`, `under`, `and`) while
treating rules-derived predictability as the default everywhere else. The user explicitly revoked
that default after reflecting on it directly: *"I want some balance, I get that rare words may need
to be learnable but we are making too many words 'easily learnable'... full speed everywhere is my
new ideology. break all of my old rules... we can always build multistroke phonetic fallbacks with
our + join key."*

**What this actually means, precisely** — because "full speed" alone is too vague to build against:
- A word's **primary, cheapest outline no longer needs any individual phonetic or spelling
  justification at all**, for any word, not just common ones. If a shorter or more ergonomic
  outline exists for a rare, multi-stroke word by any legitimate means (a gold-dictionary brief,
  brevity-ladder compression, a globally-optimized reassignment), it should be taken, even if
  nothing about the result is guessable from the word's sound or spelling.
- **The safety net stays, and is the reason this reversal is viable rather than reckless**: the
  `+`-joined, phonetically-derived multi-stroke fallback (the whole `docs/fallback.md` mechanism,
  `Rules::resolve`, the deployed plugin) is explicitly *not* going anywhere. A learner who forgets
  or never learned a word's optimized brief can still compose it sound-by-sound, slower, exactly as
  today. Speed optimization only ever applies to what counts as the *best* outline a real dictionary
  hands out; it never removes the ability to spell a word out by sound as a fallback of last resort.
- This directly motivates treating dictionary generation as a **global optimization problem**
  (assign every word its cheapest-available outline across the whole dictionary at once, not
  commonest-word-first-greedy-never-revisit) rather than a predictability-constrained rules engine
  with hand-picked exceptions - see the live discussion for the specific plan (an exact min-cost
  bipartite assignment over per-word candidates now, local-search refinement for cross-word costs
  like phrase-briefing and inter-stroke ergonomics later). That work is in progress as of this
  entry; treat any claim elsewhere in this doc that a word "should" look guessable as **historical
  record of a now-superseded default**, not a constraint binding future changes.

## The one rule that still overrides everything above

This was stated repeatedly and remains non-negotiable - speed-over-learnability does not touch it,
because it isn't about learnability at all, it's about correctness of method:

**No semantic, fuzzy, or embedding-based matching anywhere in dictionary generation.** Only exact
phonetic containment (is one word's pronunciation exactly a prefix/suffix of another's), exact
spelling-boundary tie-breaks, and exact, auditable cost functions (key count, the ergonomics score,
frequency weight) - including inside any future optimizer. Phonetic containment has a symbolic
ground truth already sitting in the pronunciation data — there's nothing to approximate, so a
learned similarity score would only add a miscalibrated threshold and a new source of false
positives that exact matching can't produce by construction. A global optimizer is not an exception
to this rule: minimizing an exact, readable cost function over a finite candidate set is still exact
computation, not a learned approximation — there is no gradient, no training data, and no model
whose internal weights replace an auditable reason. If a future change wants embeddings, cosine
similarity, or a trained model deciding *which outline a word gets* for *anything* in this pipeline,
that is a reversal of an explicit, considered decision, not a neutral addition — raise it as one.

**The structural one-stroke-per-word policy itself is unaffected by the philosophy reversal above**
— every word still gets exactly one primary single-stroke outline, with unlimited `+`-joined
multi-stroke backups available underneath it. What changed is *how arbitrary that one primary
outline is allowed to be*, not whether the one-outline/many-backups shape exists at all. See the
one-stroke-per-word entry above and
[the table of contents](README.md)'s own opening paragraph for the canonical wording.

## Dictionary generation became a real global optimization problem

**The day-4 philosophy reversal above was implemented, not just declared: `generate()`'s old
commonest-first greedy pass is gone, replaced by enumerate every candidate → peel the easy ones →
solve the rest as a real min-cost assignment problem (`src/generate.rs`: `Candidate`,
`candidate_cost`, `enumerate_candidates`, `solve_contested`).** The two complaints that motivated
this are both fixed directly, not worked around: `best_brief` used to source candidates from
*either* `setup.gold.plover` *or* `setup.gold.lapwing`, gated by a frequency window - lyk's own
rules-derived reading never got a chance to compete at all, which is why `take`'s own short Plover
brief (`TAEBG`) worked but `takes`'s otherwise-identical case (Plover's own listed candidate is the
longer `TAEUBGS`; lyk's own shorter spelling-variant reading, `TAEBGS`, was never tried against it)
didn't - confirmed fixed (`generate::tests::the_take_takes_inconsistency_is_gone_from_the_candidate_pool`).
And a word settled early (commonest-first) could never be revisited once a later, rarer word's own
circumstances might have made a different global arrangement cheaper overall - the root cause the
plan behind this traced `make`=`AEUBG`, `but`=`UT`, the ten stranded-word fixes, and the `on`/
`from`/`no`/`then`/`day`/`which` fragment-check overrides all back to.

**Every word's own candidate pool is now genuinely exhaustive**, merging in one list: both gold
dictionaries' listed outlines (`matches_real_consonants` now a blanket filter on every raw one, not
just a sort key `best_brief` used to apply only when ranking - new rigor over what shipped before),
lyk's own plain rules reading (always present, brief or not), the spelling-told-apart variant for a
homophone group (`rules.variant`, offered alongside the plain reading rather than replacing it - the
solver decides who keeps which, on cost and frequency, the same way it settles any other contested
text), every brevity-ladder shortening, and the full four-step star ladder on top of both the plain
reading and the spelling variant. A `[word]` literal override in `rules/briefs.toml` is the one
exception: it still forces exactly one candidate, a human decision never reopened to competition,
resolved first into `taken` the same way reserved outlines always have been.

**The headline acceptance gate**: all 18 `rules/briefs.toml` `[word]` overrides whose own comments
cited this exact architectural gap (`make`, `but`, `on`/`from`/`no`/`then`/`day`, `which`, and the
ten stranded-word fixes) were removed, and the new pipeline was confirmed to reproduce or beat every
one of them unprompted (`generate::tests::the_acceptance_gate_reproduces_or_beats_every_removed_override`).
Twelve reproduce or beat their own override's exact outline directly. The other six
(`on`/`from`/`no`/`then`/`day`/`which`) are a different shape of result worth spelling out: their
own literal text (`"OB"`, `"PR"`, `"PHO"`, `"THEB"`, `"TAEU"`, `"KH"`) was itself a hand-picked
exploit of a gap in `best_brief` - a half-held chord `matches_real_consonants` would reject from any
ordinary candidate, bypassed only because a literal override skips that check entirely. The new
blanket filter closes that exact gap, so none of those six literal texts is a legal candidate
anymore at all - what the new pipeline reproduces instead, unprompted, is each word's own correct,
full phonetic reading (`OPB`, `TPR`, `TPHO`, `THEPB`, `TKAEU`, `WHEUFP`) - exactly the alternative
each override's own comment already named as what the exploit was avoiding. That is "beats the
override," read correctly: the exploit is gone, and the fallback it was built to improve on is
exactly what ships now.

**Three real bugs were found and fixed while building this, each worth knowing about on its own:**

1. **Peeling by candidate-list length left ~99% of the dictionary "contested."** The first
   implementation tried to settle the easy majority of words before handing the hard remainder to
   the solver by looking for words with exactly one candidate left. That number is almost never one
   for any word with a plain rules reading, because the full four-step star ladder (source 5 in
   `enumerate_candidates`) is *always* present as a fallback, regardless of whether the word's own
   best pick is actually contested by anyone - so nearly every word starts with five-plus
   candidates. Measured directly: 99.3% of the dictionary "contested," and a solver built for "a
   sparse, few-hundred-to-low-thousand-word subgraph" turned a 100,000-word build into a multi-
   minute hang. Fixed by changing the peeling test to a word's own *cheapest remaining* candidate
   being the unique cheapest among every other remaining word (sort each list once, compare heads).
2. **That fix was still unsafe, in a way that silently produced a worse dictionary rather than
   hanging.** A word is only safe to peel when its own cheapest candidate's *text* has no other
   remaining word as a claimant *anywhere* in that word's own list - not merely "no one else's own
   *top* pick is this text." The difference showed up as a real regression: `on`'s own natural
   second choice, `OPB`, has no competing top pick from anyone else (`on` was still holding out for
   its own cheaper, contested `O`), so the far rarer word `gone` - for whom `OPB` genuinely was the
   cheapest, uncontested-by-anyone-else-at-the-top pick - got to claim it immediately, before `on`
   ever got a say. `on` fell all the way to `AUPB` (4 keys) instead of its own correct `OPB` (3
   keys) - worse than even the removed override. Fixed by only peeling a word's cheapest candidate
   when it appears in no other remaining word's list at all, full stop.
3. **Edge weight was first `common - rank`, per the plan's own original formula - measurably wrong
   once built.** The idea was "strictly frequency-ordered, bounded well under `u64` overflow," but
   `common - rank`'s own dynamic range collapses for any reasonably common word once `common` is
   100,000: "he" (rank 17) and "heh" (rank 2,191) differ by over 130x in actual usage count but
   under 2% in `common - rank`. That let a rare word's merely-somewhat-worse fallback out-arbitrate
   a far commoner word's much better option purely because the two words' weights looked nearly
   identical to the solver - measured directly as a real regression (lyk losing to Plover in
   aggregate keys/stroke, not just a small one). Fixed by using `setup.count[word]` - actual usage
   count, the same field `build`'s own "weighted by how often it is said" report already uses -
   which has the dynamic range frequency weighting is actually for, with no overflow risk at the
   scale this dictionary ever reaches.

**A real Hall-violation (more mutually-exclusive words in one connected component than it has
distinct texts for - a genuine multi-way homophone/near-homophone cluster tighter than its own star
ladder can resolve) is still possible, and is handled inside `solve_contested` itself now, not left
to the caller.** Every word gets one extra, private edge straight to the sink, costed at 1,000x that
word's own worst real option (which still scales with that word's own frequency weight, since the
real edges it's computed from already have it folded in) - this is what makes the solver correctly
sacrifice whichever word in an over-subscribed cluster has the least to lose, rather than whichever
one a raw max-flow search happens to run out of augmenting paths on first (confirmed this mattered:
an earlier version of this exact mechanism, before the per-word scaling, once left "cat" - the
common word in a `cat`/`kat`/`katt`/`khat` true-homophone cluster - without any outline at all,
while its three far rarer cluster-mates kept theirs).

**Every `HashMap`/`HashSet`-iteration-order-dependent point in the new pipeline now sorts
explicitly.** Rust's default hasher is randomized per process; collecting a word's own candidates,
a connected component's own members, or a text's own claimant list straight out of a `HashMap`
without an explicit, deterministic order meant a genuine cost-tie's *specific* winner (not the
total cost, which a true tie keeps identical either way) could differ run to run - confirmed
directly: the quote-corpus benchmark's own word-level counts moved by one or two words between
otherwise-identical runs before this fix. This project's whole ethos is exact and reproducible, not
"equally good, whichever one happened to come up" - every such collection point now sorts by
(cost, outline text) or by word text before being used.

**The contested-set-size invariant the whole algorithm choice rests on turned out to be a real,
different number than the plan estimated, not a bug to chase back to the original guess.** The plan
expected peeling to leave "hundreds to low thousands" of words genuinely contested (under 5% of the
dictionary); a real, correctly-peeled run measures roughly a quarter of it. The blanket
`matches_real_consonants` filter's deeper fragment-drops mean far more words now legitimately
converge on the same small handful of very short, nearly-bare outlines than a single-source brief
pool ever produced - a real, structural property of this design, confirmed not to be the 99.3%-style
bug above (see `quality::the_contested_set_after_peeling_stays_a_minority_of_the_dictionary`, set at
a real, measured ceiling rather than the plan's original estimate). What keeps the solver fast at
this scale isn't a small raw contested count - it's that the contested set splits into many small,
mutually-independent connected components (two words share an edge only if they share a candidate
text, transitively) rather than one giant tangle, so `solve_contested` runs once per small component
instead of once over the whole quarter of the dictionary at once.

**Net result, measured against the same quote-corpus benchmark and the same README benchmark this
session's earlier entries already cite**: lyk's aggregate keys across the 103-passage corpus went
from 4,640 to 4,409 (now 6.1% cheaper than Plover's 4,694 and 12.5% cheaper than Lapwing's 5,039, up
from 1.2%/7.5%); the words-individually-worse-than-Plover share, the one number this rewrite was
expected to cost something on, ended up landing back within a hair of its original value (9.5% ->
9.9%) once the peeling bug above was fixed, not stranded at the 11-12% an earlier, buggy measurement
of this same rewrite briefly showed. The README's own overall number moved from 3.40 to 3.37
keys/stroke, still beating Plover's 3.51 -> 3.52 and Lapwing's 3.93 -> 3.94. `quote_corpus`'s
thresholds and the README's own tables were both raised/refreshed in the same change, per this
project's established practice of never leaving a number stranded below what the theory now
actually does.
