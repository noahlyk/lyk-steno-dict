# Plan: make the capitalize/lowercase ("KPA") stroke family internally consistent

Scope: `rules/reserved/commands.json` in `/home/fib/code/noahlyk/lyk-steno-theory` (source of
truth, version-controlled), mirrored to the deployed
`~/.config/keymux/steno/lyk/01-lyk-commands.json`. No Rust code changes expected.

## Why

Earlier the same inconsistency was found and fixed for the spacing family: `and` sat on bare `SP`,
one character off from the deployed `S-P` (insert-space) stroke, with no way to tell them apart by
eye. That got fixed (see `git log --oneline | grep -i 'and\|SP'` for the commit — `and` now uses
`SKP`, `SP` itself now means "insert a space," consistent with the family it was colliding with).

The user flagged the **same category of problem** in the capitalize/lowercase family and it was
explicitly deferred: *"the KPA-family (KPA/KPA*/KPAD/KPA*L) star/suffix inconsistency you
flagged — I never got an answer, never touched it."* This plan is that deferred work.

## Current state — the full picture, already gathered, don't re-derive it

From `rules/reserved/commands.json` and `rules/reserved/punctuation.json`:

| Stroke | Does |
|---|---|
| `KPA` | capitalize next word (`{-|}`) |
| `KPA*` | capitalize next word + join (`{^}{-|}`) |
| `KPAD` | retroactively capitalize the word just typed (`{*-|}`) |
| `KPA*L` | lowercase next word (`{<}`) |

The problem: there's no consistent grammar across these four. The star means "+join" on `KPA*`
but doesn't appear at all on the retroactive variant (`KPAD` uses a trailing `D` for "retroactive"
instead), and `KPA*L` uses the star for something else again (just happens to also have one, not
because star means anything fixed here). Compare to the spacing family, which was made consistent
by giving the *bare* stroke the primary meaning and keeping the rest as recognizable variants on
it — the capitalize family doesn't have that structure at all right now; each of the four looks
like an unrelated fact to memorize.

## What this plan does NOT decide for you

This is **subjective, personal-preference design**, not a correctness question with one right
answer the way the dictionary-generation rules are. The previous session deliberately did not
redesign this alone and asked the user for direction instead — that's still the right call. **Do
not silently invent a new scheme and ship it.** Either:
(a) propose 2–3 concrete candidate schemes and ask the user to pick, or
(b) if this plan is being executed by an agent the user has explicitly told to "just decide,"
pick the most defensible one and say clearly why, the same level of justification this session
gave for e.g. the `vowel_rank` tie-break fix in `src/rules.rs`.

## A starting point for option (a) — concrete candidates, not prescriptive

One reasonable direction: **a trailing letter means "retroactive," a star means "with join," no
letter/no star is the plain forward case** — applied uniformly:
- `KPA` → capitalize next (unchanged)
- `KPA*` → capitalize next + join (unchanged, already fits)
- `KPAR` (or some other consistent "retroactive" marker, not reusing `D` for no mnemonic reason) →
  retroactively capitalize the word just typed
- `KPAL` (star dropped, since star now means "+join" specifically, not "lowercase") → lowercase
  next
- `KPA*L` → freed up; could become "lowercase next + join" for symmetry, or left unclaimed

Another direction: keep the existing four strokes exactly as they are (don't ask the user to
relearn four strokes they may already have muscle memory for from Lapwing/Plover), and instead
just **document** the actual pattern clearly in `docs/` (once `plans/03-docs-restructure.md` has a
punctuation/commands chapter, this family belongs there) so it reads as "yes, these are four
separate memorized facts, and here they are grouped together" rather than implying a consistency
that was never really designed in. Cheaper, no risk of breaking muscle memory, but doesn't solve
the stated complaint ("I need ALL of these outlines to be the same").

Present both directions to the user (or pick between them per the (a)/(b) rule above) before
touching the deployed config — getting this wrong means breaking strokes the user already has
muscle memory for, which is a worse outcome than leaving it alone.

## If a scheme is chosen and implemented

1. Edit `rules/reserved/commands.json` (source of truth in this repo).
2. Copy it to `~/.config/keymux/steno/lyk/01-lyk-commands.json` (the deployed config keymux
   actually reads) and `diff` the two files to confirm they match.
3. Validate the JSON (`python3 -c "import json; json.load(open('...'))"` — this repo's convention
   throughout the session for checking a hand-edited JSON dictionary didn't get corrupted).
4. Check every new/changed stroke doesn't collide with a real word in
   `~/.config/keymux/steno/lyk/02-lyk-words.json` — this exact bug (a word silently shadowing a
   command stroke) happened twice already this session (`TP-BG`/`!`, `SP`/`and`) before the
   generator was taught to reserve command/punctuation strokes permanently
   (`rules/reserved/*.json` feeds into `src/generate.rs`'s collision map at build time) — so a
   *new* stroke chosen here should also be added to `rules/reserved/commands.json` specifically
   (not just the deployed file) so the next `cargo run --release -- build` can never silently
   reclaim it for a word.
5. If step 4 requires it, rebuild: `cargo run --release -- build`, redeploy
   `out/lyk.json` → `02-lyk-words.json`.
6. Commit the `rules/reserved/commands.json` change to `lyk-steno-theory`, pushed.

## Deliverable

Either a short writeup presenting the options for the user to choose from (if going with (a)), or
a committed, deployed, verified change (if (b) applies) — not a silent redesign shipped without
the user's sign-off.
