# Roadmap

[← back to the table of contents](README.md) · [← back to the main README](../README.md)

Not a changelog, not shipped — a working plan, written up after a round of live testing, Discord
feedback, and reading other theory-builders' write-ups. Nothing here is built yet; each item gets
its own commit when it lands.

## 1. Correctness first — done

Several pieces assumed missing earlier in this plan turned out to already exist in the deployed
config (`~/.config/keymux/steno/lyk/`), and testing them surfaced one real, structural bug —
fixed, not just found:

- **Fingerspelling, misc keys, OS shortcuts** (`04-lyk-emilys-modifiers.py`/`.json`) — full A-Z
  fingerspelling, Tab/Backspace/Delete/Escape, arrows, page nav, media keys, typable symbols with
  shift variants, and Shift/Alt/Super/Ctrl chording for any key. Forked from Lapwing's "Emily's
  Modifier Dictionary," theory-independent. Tested, working.
- **Punctuation** (`03-lyk-punctuation.json`) — periods, commas, question marks, exclamation,
  colon, semicolon, quotes, parens, dashes, and a long tail of symbol strokes, forked from Plover's
  own. Tested — found that `TP-BG` (`!`, Lapwing's own convention) was silently unreachable: the
  generated word dictionary loads at *higher* priority than punctuation in a real keymux config, so
  a word the generator independently picked for the same stroke ("factory") shadowed it. Not a
  one-off: the generator had no idea punctuation, commands, or fingerspelling strokes existed at
  all, so this could resurface for any word on any future regeneration. Fixed structurally, not
  patched: `rules/reserved/` now holds a copy of the deployed commands, punctuation, and
  fingerspelling dictionaries, and `generate()` reserves every stroke in them before a single word
  is assigned an outline — the same mechanism briefs already use to reserve their own strokes.
  40 strokes total were affected; cost impact negligible (keys/word moved from 3.945 to 3.950).
- **Quotes** (`cargo run -- quote`) — already pulls live from `zenquotes.io`, already strokes the
  result next to Plover/Lapwing. Still worth an end-to-end pass through the live daemon on a real
  quote to confirm punctuation/caps/apostrophes/fallback all resolve cleanly together, not just
  that the command runs — not yet done.
- **Reverse lookup** (`cargo run -- explain WORD`) — already goes word → stroke(s) across all three
  theories. Functionally there; a dedicated `lookup` alias with a terser output is a nice-to-have,
  not a gap. (Separately, a much bigger reverse-lookup redesign is in progress elsewhere — see
  keymux's own planning for that, not this repo.)
- **The AO/UH fallback tie-break**: fixed — see `3822e12`. The orthographic fallback's best guess
  for a bare `AO` stroke is now "oo" (`UW.oo`), not "uh" (`UH`), since the latter was the rarer of
  three colliding readings despite "winning" by an accidental plain-vs-variant tie-break rule.
- **Brief outlines that split a two-key consonant in half** (`good`, `day`, `work`, and 245 of the
  1500 auto-briefed words total): fixed — see `93251ef`. A brief now has to read back as the
  word's real first and last sound under lyk's own rules, not just be the fewest keys. Real
  tradeoff, not free: the headline number moved from 3.49 to 3.73 keys/stroke, now behind Plover's
  3.51 instead of ahead of it — see [the main README](../README.md#where-lyk-stands) for the full
  explanation, written up there rather than hidden.

## 2. Dictionary growth

- **Suffixes** (`rules/suffixes.toml`) — done: `-tive`, `-ic`/`-ical`, `-al`, `-logy`, `-ity`/`-ety`,
  `-cy`/`-sy`, `-ify` (two-stroke, like `-ism`). `-ious`/`-eous` needed no new entry — it's already
  the existing "-y" stroke followed by the existing "-ous" stroke. `-graphy`/`-ography` stayed out:
  the schwa right before it takes a spelling-dependent vowel no fixed standalone stroke can know in
  advance (see `docs/suffixes.md`'s own "Not yet covered" for why). Plus prefix strokes on the same
  "fixed stroke that glues onto the next one" pattern `ex-`/`dis-` already use: `un-`, `mis-`,
  `over-`, `under-`, `inter-` (`rules/plover.toml`). `re-` and `pre-` stayed out too — they already
  have their own carve-out ([[spelled]]'s `IH.e`/`IY.e` entries) that deliberately keeps their
  reduced vowel, which a fixed prefix would silently override. `non-` stayed out: its sound is a
  vowel between two copies of the same consonant (`N AA N`), so there is no second onset chord to
  combine with `un-`'s own the way `under-`/`inter-` combine two different ones.
- **Briefs** (`rules/briefs.toml`): raise `count` past 1500 and hand-check the next tier; the
  `work`/`wore`/`working`/`worrying`/`wok` collision mess from the Discord log is a concrete sign
  these need explicit briefs instead of improvised rule codas.
- **Compound suffix strokes** (lower priority): one continuation stroke for a common suffix *pair*
  (e.g. `-er`+`-s` → "-ers" in one stroke instead of two) — the one portable idea from tarot's
  Discord chart. Our `^` key already makes single suffixes cheap; this is a smaller win than
  filling out single-suffix coverage first, so it comes after Plan 2's main suffix work, not before.

## 3. Theory architecture — ideas from reading onyx's write-ups

Read `onyx` (+ its `/conflicts` and `/regular-outline-grammars` pages), `onyx-syllabic-splitting`,
and `generated-dictionaries`. Short verdict: **lyk's architecture already matches their own
conclusions more than it needs new ones from them.** Their taxonomy names word-boundary conflicts,
proper-noun conflicts, chord/stenonym conflicts, and homophonic conflicts, each with a structural
fix rather than a memorization patch — and lyk already has the fix they recommend for the one that
matters most: a dedicated join key for word-boundary conflicts (their "option 1," which they rate
above the vowel-dropping alternative Phoenix uses — exactly the alternative tarot said "doesn't go
all the way," which checks out: it causes forget/forgot-style collisions lyk's join key avoids by
construction).

**Conflict-taxonomy audit — done, 2026-10-29:**
- **Word-boundary** — solved by construction, the join key (rule 1 of this section).
- **Proper-noun** — handled, via command strokes, not a dedicated outline shape: the deployed
  punctuation dictionary (`03-lyk-punctuation.json`) already carries `KPA`/`KPA*` → `{-|}`
  (capitalize the next word), fired before a name the same way Plover's own convention works.
  lyk itself generates no proper-noun outlines of its own — names type as whatever their ordinary
  phonetic outline would be, capitalized by that stroke. Not a gap, just worth having written down.
- **Chord/stenonym** (two different words landing on the identical outline) — handled by the star
  ladder (`theory.md` rule 6): commonest word keeps the plain outline, the rest take a star on the
  first stroke, then the last, then two star strokes. ~1,200 words currently use a star form
  (`README.md`'s full benchmarks section). Working as designed, not audited word-by-word here —
  no reason found to think it needs to be.
- **Homophonic** — handled orthographically, by spelling rules (`theory.md` rule 3: `sail`/`sale`,
  `ea`/`ee`, `oa`/`o-e`, `air`/`are`), exactly the structural fix the article's taxonomy
  recommends over leaving it to per-word judgment. Already the deliberate approach, confirmed.

All four hold up. The one genuine finding from this pass was unrelated to the taxonomy itself: the
orthographic fallback's bare-`AO` tie-break (section 1 above) was picking the least likely of three
colliding readings — fixed (see `3822e12`).

Two other ideas from the same reading, still worth keeping in mind:
- **Pseudo-strokes**: a named technique for splitting one hard-to-encode coda cluster into an
  onset-equivalent + coda-equivalent half within a single stroke. Relevant only if/when
  `[coda]`/`[coda_cluster]` in `rules/plover.toml` get expanded toward the denser cluster tables
  tarot's chart showed — a real but much bigger, separate project, not scheduled here.
- **One honest flag**: the onyx author tried a fully phonetic rule-based theory and gave up on it,
  specifically because homophone/unstressed-vowel resolution became "vibes" with no system behind
  it. Not a reason to change course — but the long tail of homophones and reduced-vowel collisions
  is exactly where lyk needs deliberate rules, not per-word judgment calls, as briefs run out and
  the rule-generated tail grows.

## 4. Docs — reorder into a sequential read

Right now `docs/` is a lookup table (`theory.md`, `suffixes.md`, `fallback.md`), not something
meant to be read start to finish. Restructure so reading every doc in order teaches the whole
theory, roughly:

1. Pitch & the join key (why word boundaries are explicit) — exists, in `theory.md` rule order
   already, just needs to be pulled out as its own first chapter
2. Briefs — one outline per word, how the common tier is picked
3. Phonetic rules — onset / vowel / coda, how a plain syllable gets its stroke
4. Spelling disambiguation — picking between ways to write the same sound
5. Prefixes with their own keys
6. Syllables & folding — how an unstressed syllable joins the stroke before it
7. Brevity steps — how a long word gets shorter
8. Suffixes — the standalone continuation-stroke table
9. Punctuation & commands — now documented, since it turned out to already exist
10. Fingerspelling & misc keys ("Emily's") — likewise
11. The fallback — what a miss resolves to
12. Benchmarks — where lyk stands, last, since it's a result, not a rule

Numbered filenames (`01-join-key.md`, `02-briefs.md`, ...) or an explicit "read in this order" list
on `docs/README.md` — either works, pick one when it's time to build this out.
