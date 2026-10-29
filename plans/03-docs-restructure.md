# Plan: restructure `docs/` into a sequential, start-to-finish read

Repo: `/home/fib/code/noahlyk/lyk-steno-theory`, branch `main`. Commit and push directly.

## Why — read this part first, it's not optional context

The user's own words: **"DOCS ARE THE MOST IMPORTANT THING theory is good but the most important
thing is a theory you can implement, I want 100% exhaustive documentation of rules and things I
can learn to actually be able to type my theory."** This is the single highest-priority standing
value for this repo, ranked explicitly above the code itself. Treat this plan accordingly — don't
rush it, don't leave anything half-restructured, and don't let it drift out of sync with the code
again afterward (see `plans/04-claude-md-docs-importance.md`, a short companion file that should
exist alongside this one and makes the same point as a permanent instruction).

## Current state

`docs/` is a lookup table, not something meant to be read start to finish:
- `README.md` — table of contents, one row per page, no reading order implied
- `theory.md` — the rule book (already *internally* in a sensible rule-1-through-N order; this is
  the file to pull apart, not throw away)
- `suffixes.md` — a reference table over `rules/suffixes.toml`
- `fallback.md` — what a miss resolves to
- `roadmap.md` — a working plan, not theory; not part of the reading order at all

The main `README.md` (repo root, not `docs/README.md`) also has its own structure: pitch →
"Where lyk stands" (one number) → Documentation (links into `docs/`) → Main features → Use (CLI) →
Full benchmarks. That structure is **not** being changed by this plan — only `docs/`'s own internal
organization is in scope.

## The target structure

A previous planning pass (`docs/roadmap.md`, section 4, written before this restructuring was
scheduled) already worked out the intended chapter order. Use it as the starting point, verify it
against `theory.md`'s actual current rule numbering before committing to it (the rule numbers have
shifted at least twice this session — reserved-strokes was inserted as rule 1, pushing everything
else down):

1. **Pitch & the join key** — why word boundaries are explicit, what the `^` key buys. Currently
   theory.md's intro framing plus whichever rule covers the join key itself.
2. **Reserved strokes** — commands/punctuation/fingerspelling are pinned before anything else.
   Currently theory.md's rule 1.
3. **Briefs** — one outline per word, how the common tier is picked, the brief-correctness rule
   (full cluster or fully dropped, never a fragment). Currently theory.md's rule 2.
4. **Phonetic rules** — onset / vowel / coda, how a plain syllable gets its stroke. Currently rule 3.
5. **Spelling disambiguation** — picking between ways to write the same sound. Currently rule 4ish
   (verify against the live file).
6. **Prefixes with their own keys**.
7. **Syllables & folding** — how an unstressed syllable joins the stroke before it.
8. **Brevity steps** — how a long word gets shorter (fold/skeleton/shell/ends).
9. **Suffixes** — the standalone continuation-stroke table. Currently its own page, `suffixes.md`
   — decide whether it becomes chapter 9 directly or stays a separate page linked prominently at
   this point in the sequence (a large reference table may genuinely read better as its own page
   than inlined — use judgment, but the *reading order* must make it obvious this is where it
   belongs, even if the content lives in its own file).
10. **Punctuation & commands** — now that `rules/reserved/` exists and is documented, this
    deserves its own chapter rather than being folded into "reserved strokes" at the top; that
    first mention should be structural ("these are pinned first"), this chapter should be the
    actual content (what's in there, why those specific strokes).
11. **Fingerspelling & misc keys** ("Emily's") — likewise: exists, is deployed, is real, currently
    has no dedicated doc page explaining it at all. This plan should produce one.
12. **The fallback** — what a miss resolves to. Currently `fallback.md`.
13. **Benchmarks** — where lyk stands, last, since it's a measured result of everything above, not
    a rule someone needs to learn. Already lives on the main README, not `docs/` — this chapter is
    just a pointer/transition, not new content.

## What to actually do

Pick one of two concrete approaches and commit to it fully, don't half-do both:

- **(a) Numbered files**: `docs/01-join-key.md`, `docs/02-reserved-strokes.md`, etc. Pro: the
  reading order is visible in a file listing with zero prose needed. Con: renumbering later (if a
  chapter gets split or merged) means renaming files and fixing every link to them.
- **(b) Unnumbered files, explicit order in `docs/README.md`**: keep roughly the current filenames
  (or close to them), but rewrite `docs/README.md` as a genuinely sequential table of contents —
  "read these in this order" — rather than today's unordered lookup table. Pro: no file-renaming
  churn later. Con: relies on someone actually reading the TOC in order rather than the filesystem
  making it obvious.

Given this repo's existing convention (filenames like `theory.md`, `suffixes.md`, `fallback.md` —
descriptive, not numbered) and that cross-links between docs pages already use filenames directly
(`[the theory page](theory.md)` appears throughout `README.md`, `fallback.md`, etc.), **approach
(b) is very likely less disruptive** — but verify how many existing links would break under
approach (a) (`grep -rn '\.md)' docs/ README.md`) before deciding; if it's a small, mechanical
number of links to fix, (a) is still on the table and has a real legibility advantage.

Either way:
- Split `theory.md`'s rules out into their own chapter files/sections per the structure above —
  don't just leave it as one monolithic file with a reordered intro.
- Write two **new** pages that don't exist yet: one for punctuation/commands, one for
  fingerspelling/misc keys ("Emily's"). Source material: the deployed config at
  `~/.config/keymux/steno/lyk/` (now also version-controlled under `rules/reserved/` in this repo
  — `rules/reserved/punctuation.json`, `rules/reserved/commands.json`,
  `rules/reserved/emilys-modifiers.py`/`.json`). Document what's actually in them (the strokes,
  grouped sensibly — don't just dump the raw JSON), not a restatement of "these exist."
- Rewrite `docs/README.md` as the actual table of contents for this order, dropping the current
  unordered-lookup-table framing.
- Update every cross-link between docs pages and from the main `README.md` into `docs/` to match
  whatever the final file layout is.
- `docs/roadmap.md` is explicitly **not** part of the reading order (it's a working plan, not
  theory) — leave it where it is, just make sure `docs/README.md`'s new TOC doesn't imply it's
  chapter N of the theory.

## Verification

This is a docs-only change, but don't skip the sanity pass:
```
cargo build --release          # confirm nothing in src/ references docs/ paths that would break (unlikely but check)
cargo test --release
cargo run --release -- score --check-readme   # confirm the main README's auto-checked tables are untouched
```
Then manually click/follow every link in the new `docs/README.md` and the main `README.md`'s
"Documentation" section to confirm nothing 404s.

## Deliverable

Commit(s) on `main`, pushed. This is purely a repo change — nothing to rebuild or redeploy to the
live config afterward.
