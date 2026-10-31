# Briefs

[← back to the table of contents](README.md) · [← back to the main README](../README.md)

**Briefs** (`rules/briefs.toml`). The commonest words keep the outlines people already
know, from Plover by default (`HE`, not `HAOE`) or Lapwing, per word. They are pinned before
anything else is generated, and nothing generated can take their strokes. A briefed word gets
exactly that one entry: no rule outline, no short form, no alternate pronunciation reachable
alongside it. There is only ever one way to stroke the word, faster to learn than picking the
cheapest of several, and whatever its rule outline or short forms would have claimed is never
reserved, so a later, less common word is free to take it instead — a brief can shorten a
completely unrelated word as a side effect, since words are settled once, commonest first,
with nothing undone to recheck afterwards. A brief is free to be a pure memorized shortcut with
no phonetic connection to its word at all — that's the point of a brief, and nothing about it
needs to be "learnable from the rules" the way a generated outline is. What a candidate may
never do is take only *part* of a key combination these same rules only ever use *together* for
one sound. Plover's own shortest outline is sometimes exactly that: a fragment of its own
theory's multi-key sound, not a clean shortcut (`good`'s Plover brief `KPWAOD` keeps the `P W`
of `TKPW`'s own "g" but drops the `T`, which these rules read back as "k", not "g" — a
different, wrong sound, not just an unrelated memorized one). That fragment is rejected, and
two repairs are generated alongside the original candidates to replace it: completing the
fragment into the whole, correct cluster, or dropping it outright, since a brief is always
free to skip a sound it isn't attempting at all. Dropping is usually fewer keys, so it's what
wins when it's available (`good`'s brief is `AOD` — no attempt at its "g" sound at all, rather
than the seven-key `TKPWAOD` completing it) — but either repair only exists because the word's
own gold dictionary pointed at that sound to begin with; nothing is invented from nothing.
Fewest keys then picks among whichever candidates, original or repaired, are left — except that
a *dropped* candidate (never a raw one) loses this tie-break if it lands exactly on the plain
outline of a different, commonly-known word that would otherwise have needed it (`fact` used to
drop its `K` entirely down to `TPAT`, exactly `fat`'s own plain outline, bumping `fat` onto the
starred `TPA*T` to make room). A rarer word losing its plain outline to a brief outright is this
whole mechanism's normal, accepted cost — that's what the star ladder is for — so this check
only looks at words inside the brief window itself (`briefs.count`) that are not themselves
landing a brief: a word briefed regardless of what it drops, like `a` pinned to `AEU`, was never
going to need its own plain reading (`A`) at all, so it does not count as a collision.

That fewest-keys ranking also decides what happens when the very best candidate is already
someone else's: words settle commonest-first, so a common word's top pick can already belong to
an even commoner one (`from`'s own best candidate, bare `R`, is already `are`'s). Rather than
giving up there and falling back to the word's full, multi-stroke phonetic outline, every
candidate is tried in the same fragment-safe order, and the first one nobody else already has
wins (`from` falls through `R` to land on `TPR` instead — three keys, not the six its own rules
outline would have cost). This only ever reaches further down its *own* ranked list; it never
reconsiders a word already settled, so it costs nothing to a commoner word and changes nothing
about which word wins a shared outline.

Fewest keys can still leave a flat tie between two of a word's own gold outlines — Plover's own
list for `best` has both `PW*ES` and `PWEFT`, costing exactly the same 5 keys. The star there
isn't disambiguating anything this word needs told apart; it's just an artifact of Plover's own
source, picked over the plain form by nothing more principled than list order. The star-free
candidate wins a tie like this (`best` lands on `PWEFT`, matching the `[coda]` rule for `-st`
rather than a coin flip) — a tie-break that only ever fires once every sharper measure above it
(length, fragment-safety, collision-avoidance, keys) has already come out equal, so it can never
turn a word away from a genuinely cheaper or safer candidate.

Generation keeps a running count of words whose final outline costs more than their own
plain-rules reading even though that reading is sitting completely free — a regression guard, not
an auto-fix, printed by `build` as "words stranded on a worse outline than their own, unclaimed
plain reading." This can genuinely happen: Plover and Lapwing occasionally mark a one-off
candidate with a star, or spend an extra key, for a reason of their own theory's that has nothing
to do with this one (`wow`'s Plover brief is `WO*U`; the exact same keys with no star, `WOU`, read
this word just as well and cost one key less, and sat completely unclaimed). Fixing a case like
that by hand is exactly what `[word]` in `rules/briefs.toml` is for — a one-word, looked-at-by-a-
human override, the same mechanism already used for every other judgment call this file lists.

A blanket, automatic version of that fix — "use whichever candidate is cheaper, always" — was
tried and reverted. Two things ruled it out. First, it isn't safe to compare a brief only against
the word's own plain-rules reading: a different pronunciation of the same written word can parse
to something shorter with no relation to what the brief is actually for (`a`'s Plover brief
`AEU` "lost" to a schwa-pronunciation reading of the same word that has nothing to do with the
stressed vowel `AEU` is written for). Narrowing the comparison to only strip a candidate's *own*
star and check if that exact text is free avoids that specific trap, but still doesn't avoid the
second problem: this generator settles every word once, commonest-first, with nothing revisited —
so changing even one early word's outline reshuffles which outlines are free for every word
after it, and that reshuffle reaches much further than the word being fixed (confirmed directly:
even the narrow, star-only version touched outlines for several hundred other words, some of them
worse off than before, nowhere near the word it was meant to help). That ripple is this whole
mechanism's normal, accepted cost when a *human* decides a specific word's brief should change —
it's the same cost adding any new `[word]` entry already has — but it is not something to trigger
automatically from a blanket rule with no one looking at what it actually moves. The regression
guard's job is to surface candidates for that human look, not to resolve them on its own.

The window itself (`count`) is a default for words nobody has looked at individually, not a
ceiling: a word named explicitly in `[word]` gets a brief regardless of rank, the same way a
two-key-or-fewer outline already does regardless of rank. That is only done when every one of
three things holds, checked mechanically rather than by eye: Plover's own outline saves at
least three whole keys over what the rules generate for the word (a one- or two-key saving this
far down the frequency list was rejected even where it existed, since it barely moves the
frequency-weighted cost the rare tier is judged on); that outline's first and last consonants
are a whole cluster toward the word's real first and last sound, never a fragment of one (the
same check a plain `plover`/`lapwing` source above already runs through); and claiming it does
not take a stroke a commoner word already has by the rules (checked by rebuilding — a rarer
word losing its own outline to a star form, the ordinary cost of adding any brief at all, is
fine; a commoner one losing its outline because a rarer word's hand-picked brief grabbed it
first would not be). `rules/briefs.toml` lists each word this has been done for and what it
saves.

Next: **[phonetic rules](phonetic-rules.md)** — how a word with no brief gets its stroke instead.
