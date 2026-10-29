# Handoff plans

Five outstanding asks from this session, written up for a fresh agent (or several, run in
parallel or in sequence) to pick up with no other context. Each file is self-contained — read the
one you're assigned, not the others, unless a plan explicitly points you at another.

| Plan | What | Touches |
|---|---|---|
| [01-suffix-expansion.md](01-suffix-expansion.md) | Grow `rules/suffixes.toml` — the user has asked twice for "WAY more suffixes" | `rules/suffixes.toml`, `rules/plover.toml` (prefixes), `docs/suffixes.md` |
| [02-brief-expansion.md](02-brief-expansion.md) | Find more words outside the auto-brief window worth a deliberate brief, with an explicit keep/reject rule | `rules/briefs.toml`, `docs/theory.md` |
| [03-docs-restructure.md](03-docs-restructure.md) | Reorder `docs/` into a sequential, start-to-finish read — currently a lookup table, not a tutorial | all of `docs/` |
| [04-claude-md-docs-importance.md](04-claude-md-docs-importance.md) | Write `.claude/CLAUDE.md` — a standing reminder that docs outrank code in this repo | new file, `.claude/CLAUDE.md` |
| [05-kpa-family-consistency.md](05-kpa-family-consistency.md) | Decide and (if approved) implement a consistent capitalize/lowercase stroke scheme | `rules/reserved/commands.json`, deployed config |

## Order, if running them one at a time

**01 and 02 can run in parallel with each other** (different files: `suffixes.toml` vs
`briefs.toml`), but **both should land before 03** (docs restructuring will want to reflect
whatever the final suffix/brief content looks like, and touching the same `docs/*.md` files
concurrently with 01/02's own doc updates risks a collision). **04 is independent and safe to run
any time**, including first. **05 is independent of all the others** (different file entirely,
`rules/reserved/commands.json`, not `rules/*.toml`) but needs a decision from the user before
anything gets written, not just an agent picking for them (see that plan's "What this plan does
NOT decide for you" section).

## Shared ground rules, all five plans

- Repo: `/home/fib/code/noahlyk/lyk-steno-theory`, branch `main`, plain checkout (not a worktree —
  no isolation step needed, just don't run two agents on overlapping files at the same time; see
  the ordering note above).
- If multiple agents run concurrently: `git status`/`git log --oneline -3` before starting and
  again immediately before committing, to catch a collision before it happens rather than after.
  This repo has had real collisions this session from exactly this kind of concurrent work — not
  hypothetical.
- Every plan that touches `rules/*.toml` or `src/*.rs` needs, before committing:
  `cargo build --release && cargo test --release && cargo run --release -- score --check-readme &&
  cargo clippy --release --all-targets` — and if `--check-readme` flags a stale table, paste the
  corrected table back into `README.md` rather than ignoring the failure.
- Report real numbers, not favorable rounding, when a change affects the dictionary's cost
  (`cargo run --release -- score`) — this repo's commit history is consistently honest about
  tradeoffs, including when a change makes something worse in exchange for something else; keep
  that up.
- Redeploy after any change to the generated dictionary or plugin:
  ```
  cargo run --release -- build
  cargo run --release -- plugin
  cp out/lyk.json ~/.config/keymux/steno/lyk/02-lyk-words.json
  cp out/lyk-orthographic-fallback.rs ~/.config/keymux/steno/lyk/05-lyk-orthographic-fallback.rs
  ```
  Diff both copies against `out/` afterward to confirm the deploy actually landed.
