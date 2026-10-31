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
