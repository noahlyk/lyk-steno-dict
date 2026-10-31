# Briefs

[← back to the table of contents](README.md) · [← back to the main README](../README.md)

**Briefs** (`rules/briefs.toml`). A brief is a word's outline taken from Plover's or Lapwing's own
gold dictionary instead of derived fresh from lyk's own phonetic rules — `he` as `HE`, not `HAOE`.
It is free to be a pure memorized shortcut with no phonetic connection to its word at all — that's
the point of a brief, and nothing about it needs to be "learnable from the rules" the way a
generated outline is.

**Every word's primary outline is settled by one global optimization, not a sequence of passes.**
`generate()` enumerates every legal candidate for every word at once — both gold dictionaries'
listed outlines, lyk's own plain rules reading, a spelling-told-apart variant for a homophone group,
every brevity-ladder shortening, and the full star ladder on top of all of that — and then solves
for the assignment that minimizes the whole dictionary's total cost, weighted by how often each word
is actually said. A brief, lyk's own rules-derived reading, and a brevity-shortened form all compete
in the *same* pool for the *same* word; whichever is cheapest for the word, weighed against every
other word's own stake in the texts it might also want, is what wins. This is new: before, a common
word's own brief was pinned the moment it was found, with nothing else — not lyk's own plain
reading, not a later brevity treatment — ever getting a chance to compete against it, which is the
confirmed reason the dictionary used to track Plover more closely than it needed to for words like
`takes` (Plover's own listed brief, `TAEUBGS`, used to win uncontested, even though lyk's own
shorter spelling-variant reading, `TAEBGS` — the same reduction `take`'s own brief, `TAEBG`, already
happened to use — was never offered as an alternative).

**A `[word]` literal override is the one exception, and still forces exactly one candidate.** A
source of `"plover"` or `"lapwing"` in `[word]` only narrows which gold dictionary a word's own
brief candidates are merged from — the word still competes on cost like anything else. A literal
outline (`up = "-P"`, `make = "PHAEUBG"`, and the rest `rules/briefs.toml` lists this way) is
different: a human looked at this specific word and picked its outline by hand, and that decision is
never reopened to competition against the word's own other candidates, let alone another word's.
It is resolved first, the same way a reserved outline or a standalone suffix always has been, so
every other word's own candidate pool already sees it as spoken for from the start. A collision
between a literal override and something already pinned (a reserved stroke, the suffix table, "I",
or another override) is still a hard error, worth fixing in `rules/briefs.toml`, never something to
fall back from silently.

**What a candidate may never do is take only *part* of a key combination these same rules only ever
use *together* for one sound** — and this check now applies to every raw gold-dictionary candidate
offered at all, not just the ones worth ranking once a word already had a brief, which is new rigor
over how this used to work. Plover's own shortest outline is sometimes exactly a fragment of its own
theory's multi-key sound, not a clean shortcut (`good`'s Plover brief `KPWAOD` keeps the `P W` of
`TKPW`'s own "g" but drops the `T`, which these rules read back as "k", not "g" — a different, wrong
sound, not just an unrelated memorized one). A raw candidate that fails this check is never offered
at all; instead, up to two repairs are generated from it and offered as separate candidates of their
own, since a brief is always free to skip a sound it isn't attempting at all: completing the
fragment into the whole, correct cluster, or dropping it outright. Dropping is usually fewer keys,
so it tends to win when both are legal and free — but either repair only exists because the word's
own gold dictionary pointed at that sound to begin with; nothing is invented from nothing. See
[`docs/decisions.md`](decisions.md) for what this blanket rigor actually changed once applied to
the real dictionary: six words (`on`, `from`, `no`, `then`, `day`, `which`) used to carry a literal
`[word]` override that was itself a hand-picked exploit of the old, narrower version of this check —
a half-held chord a *literal* override could bypass entirely, since a literal override was never run
through `matches_real_consonants` at all. Those six overrides are gone; what the dictionary
generates for them unprompted now is each word's own correct, full phonetic reading, exactly the
fallback each override's own comment already named as the alternative to its exploit.

**Cost, for ranking candidates against each other, is strokes first, then keys, then physical
ergonomics as the final tie-break** — the same priority `shortest`/the star ladder always encoded
informally, now a single explicit function (`candidate_cost`, `src/generate.rs`) applied uniformly
everywhere a candidate's intrinsic cost matters. Fewer strokes always wins outright; among
equal-stroke candidates, fewer keys wins; among candidates tied on both, the physically more
comfortable one wins (see [`src/ergonomics.rs`](../src/ergonomics.rs)). This replaces an older,
narrower tie-break that preferred a star-free candidate specifically when two of a word's own gold
outlines tied on every other measure (Plover's own list for `best` has both `PW*ES` and `PWEFT`,
costing exactly the same 5 keys, with the star there just a list-order artifact of Plover's own
source) — ergonomics now settles that same kind of tie on the two outlines' actual physical shape
instead, which can occasionally prefer the starred form if it is genuinely the more comfortable of
the two to press, not just whichever one happens to carry no star.

**Frequency only ever enters at the very end, as the solver's own arbitration weight, never as part
of a candidate's own cost.** Two words considering the exact same outline agree on how much it
costs; what decides who actually gets it is how much each word's own total result is worth, weighed
against every other word's — `setup.count[word]`, how many times the word is actually said in the
frequency corpus, exactly the same field `build`'s own "weighted by how often it is said" report
already uses. This makes the solver's objective literally that same headline number: the global
optimum it finds *is* the average keys per word, weighted by frequency, that `build`/`score` already
report — not a proxy for it. See [`docs/decisions.md`](decisions.md) for why this has to be the raw
usage count and not a simpler stand-in like frequency rank: an earlier attempt used `common - rank`,
which looked "strictly frequency-ordered" but collapsed almost all of its own dynamic range among
reasonably common words, letting a rare word's merely-somewhat-worse fallback occasionally outbid a
far commoner word's much better option.

**A word losing a contested outline to a more valuable word is not a failure - it is the whole
mechanism working**, the direct descendant of the star ladder's old job. Every word keeps its own
full ladder of fallbacks (the plain reading, a star on the first stroke, a star on the last, then
one and two extra star strokes) as real, always-present candidates, so losing a contested text never
means falling all the way back to a word's long multi-stroke phonetic spelling-out when a cheap
ladder step was sitting right there. Generation also keeps a running count of words whose final
outline costs more than their own plain-rules reading even though that reading is sitting completely
free — a regression guard, not an auto-fix, printed by `build` as "words stranded on a worse outline
than their own, unclaimed plain reading," and backed by `src/quality.rs`'s own test holding that
count to zero. A true global optimum structurally cannot leave a strictly cheaper, completely free
outline on the table for a word that could have had it for nothing — swapping to it would always
lower the total cost — so this count is expected to stay at zero for a real, structural reason now,
not because every individual case was hand-patched the way it used to be. The guard still runs
anyway: see [`docs/decisions.md`](decisions.md) for why a confirmed-impossible failure mode is not
a reason to delete the test that watches for it.

**Where a Hall-violation really is unavoidable — more mutually exclusive words than legal outlines
to go around, a true multi-way homophone or near-homophone cluster tighter than its own star ladder
can resolve — the solver sacrifices whichever word has the least to lose, not whichever one a raw
search happens to leave out.** Every word's own candidate pool includes one more option purely for
this: a private "no match" fallback, costed far above any of that word's own real candidates but
still scaled by that word's own frequency weight, so the comparison between two words' own stakes in
a shared, scarce cluster of outlines stays apples to apples. This case is rare, genuinely
unavoidable when it happens, and is exactly the kind of unresolved result `build`'s own "words the
rules could not place" count already reports — not a new failure mode, the same one an unbreakable
spelling tie already produced before this global solver existed, just reached by a different path.

Next: **[phonetic rules](phonetic-rules.md)** — how a word with no brief gets its stroke instead.
