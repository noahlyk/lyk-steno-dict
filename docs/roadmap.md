# Roadmap

[← back to the table of contents](README.md) · [← back to the main README](../README.md)

Not a changelog, not shipped — a working plan, written up after a round of live testing, Discord
feedback, and reading other theory-builders' write-ups. Nothing here is built yet; each item gets
its own commit when it lands.

## 1. Correctness first — verify what already exists before building more

Several pieces assumed missing earlier in this plan turned out to already exist in the deployed
config (`~/.config/keymux/steno/lyk/`), just unverified:

- **Fingerspelling, misc keys, OS shortcuts** (`04-lyk-emilys-modifiers.py`/`.json`) — full A-Z
  fingerspelling, Tab/Backspace/Delete/Escape, arrows, page nav, media keys, typable symbols with
  shift variants, and Shift/Alt/Super/Ctrl chording for any key. Forked from Lapwing's "Emily's
  Modifier Dictionary," theory-independent. **Needs testing on the live daemon, not building.**
- **Punctuation** (`03-lyk-punctuation.json`) — periods, commas, question marks, exclamation,
  colon, semicolon, quotes, parens, dashes, and a long tail of symbol strokes, forked from Plover's
  own. **Needs testing, not building.**
- **Quotes** (`cargo run -- quote`) — already pulls live from `zenquotes.io`, already strokes the
  result next to Plover/Lapwing. **Needs an end-to-end correctness pass**: run real quotes through
  the live daemon and confirm punctuation, capitalization, apostrophes, and fallback all resolve
  cleanly, not just that the command runs.
- **Reverse lookup** (`cargo run -- explain WORD`) — already goes word → stroke(s) across all three
  theories. Functionally there; a dedicated `lookup` alias with a terser output is a nice-to-have,
  not a gap.
- **Regenerate and redeploy**: `build` (dictionary) and `plugin` (WASM fallback) against current
  `rules/`, diff against what's actually deployed, push anything stale.
- **Resolve the open plugin question**: the theory-faithful generated plugin resolves `-T/^AO` to
  "tuh" (matching `rules.rs`'s real AO-vs-AOU distinction) instead of "too" (the old hand-written
  plugin's bug, never corrected). Still undeployed pending a decision — needs one before `plugin`
  output replaces the hand-copied file for good.

## 2. Dictionary growth

- **Suffixes** (`rules/suffixes.toml`): `-tive`, `-ic`/`-ical`, `-al`/`-ial`/`-ual`, `-ty`/`-ity`,
  `-logy`/`-graphy`/`-ography`, `-ious`/`-eous` (folded into existing `-ous`), `-cy`/`-sy`, `-ify`
  (two-stroke, like `-ism`). Plus prefix strokes on the same "fixed stroke that glues onto the next
  one" pattern `ex-`/`dis-` already use: `un-`, `re-`, `pre-`, `non-`, `over-`, `under-`, `mis-`,
  `inter-`.
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

What's actually worth taking:
- **Audit lyk against their 4-way conflict taxonomy explicitly**, as a checklist: word-boundary
  (solved), proper-noun (confirm we have a dedicated capitalization stroke), chord/stenonym (the
  ~1,200 star-form collisions `build` already reports — audit a sample), homophonic (currently
  orthographic via spelling rules — confirm that stays deliberate, not ad hoc, as the dictionary
  grows).
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
