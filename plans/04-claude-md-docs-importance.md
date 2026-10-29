# Plan: write `.claude/CLAUDE.md` — docs are the most important thing in this repo

Repo: `/home/fib/code/noahlyk/lyk-steno-theory`, branch `main`. Commit and push directly.

## Why

The user's own words, verbatim: **"please even write down how important it is to maintain docs
in .claude/ so you never forget how important it is to maintain docs for the lyk-steno-theory
repo."** This is a direct request for a standing instruction, not a one-off doc edit — it belongs
in `.claude/CLAUDE.md` specifically because that file is loaded into context automatically for
every future session working in this repo, which is the whole point: a future agent (fresh
session, no memory of this conversation) needs to see this before it starts working, not discover
it by accident.

This file doesn't exist yet. `ls .claude/` currently returns nothing — there is no `.claude/`
directory in this repo at all. Create it.

## What to write

This is small — don't over-engineer it into a sprawling project-instructions doc. The single most
important thing a `.claude/CLAUDE.md` in this repo needs to say:

- **Docs are more important than the code, in this specific repo.** The user has said so
  explicitly and repeatedly across a full working session. Every change to `rules/*.toml` or
  `src/*.rs` that changes what a word strokes as, what a brief does, or how the generator picks
  between candidates **must** come with the matching doc update in the same commit — not as a
  follow-up, not "later." `docs/theory.md` in particular has gone stale multiple times this
  session (a brief-selection rule changed in code three separate times before the docs caught up
  each time) — this is the exact failure mode to prevent going forward.
- `cargo run --release -- score --check-readme` catches *numeric* drift in `README.md`'s tables
  automatically — it does **not** catch prose drift (a rule described one way in `docs/theory.md`
  that no longer matches what the code actually does). There is no automated check for this; it
  has to be done by hand, every time, as part of the same change.
- Point to `docs/README.md` as the entry point for a human (or agent) trying to learn the theory —
  if `plans/03-docs-restructure.md` has landed by the time this is written, mention that the docs
  are meant to be read in the sequential order `docs/README.md` lays out, not browsed at random.
- Keep it short. This is a standing reminder, not a tutorial — the tutorial is `docs/`.

## Format

Standard `CLAUDE.md` conventions: plain markdown, no special frontmatter required. A few short
paragraphs or a tight bullet list is plenty — a few hundred words at most. Write it so a future
agent reads it in a few seconds before starting work, not something that itself needs maintaining
as a second source of truth (don't duplicate the theory's actual content here — that's what
`docs/` is for; this file is about the *obligation* to keep `docs/` right, not a copy of it).

## Verification

None needed beyond confirming the file exists at `.claude/CLAUDE.md`, is valid markdown, and reads
clearly. Not a code change.

## Deliverable

One commit on `main`, pushed: `.claude/CLAUDE.md` created.
