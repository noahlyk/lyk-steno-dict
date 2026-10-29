# Plan: find more words worth a deliberate brief

Repo: `/home/fib/code/noahlyk/lyk-steno-theory`, branch `main`. Commit and push directly.

## Why

The user asked: "include pretty much every epic brief from Plover for top 100 words that you
like, and also check Plover's two key 1 stroke outlines and see if those are worth keeping too,
with some rules if to keep or to not keep."

Half of this already happened as a side effect of other work this session (see "Already true"
below) — what's left is the actual *search*: going looking for words outside the current
auto-brief window that deserve one, which nobody has done yet.

## Already true — don't re-derive this, just build on it

- `rules/briefs.toml`'s `count = 1500`: every word ranked in the top 1,500 by frequency already
  gets a brief automatically, default-sourced from Plover's own outline for it (`default =
  "plover"`), unless overridden per-word in the `[word]` table.
- `TINY_BRIEF_KEYS = 2` (in `src/generate.rs`): **any** word, however rare, whose Plover or
  Lapwing outline is 2 keys or fewer, also gets briefed — this is already "Plover's two-key
  outlines are worth keeping", just automatic and unconditional, not a judgment call per word.
- The brief *selection* mechanism (`best_brief`/`matches_real_consonants`/`drop_fragments` in
  `src/rules.rs` and `src/generate.rs`) was reworked this session so that whichever candidate gets
  picked is phonetically sound (full key clusters, never half of one) or a cleanly dropped sound,
  never a confusing fragment. That part doesn't need redoing.

So "every epic brief from Plover for the top 100" is *already* happening by construction — the
open question is what's being missed **outside** the top-1,500/2-key window: words ranked, say,
1,500–20,000 where Plover has a genuinely short (3–4 key) outline that's a deliberate, well-known
trick, not just what the rules would generate anyway, and isn't automatically caught by either
existing gate.

## The actual task

1. Build the dictionary and get a list of words ranked roughly 1,500–20,000 (check
   `src/data.rs`/`src/analysis.rs` for how frequency rank is already exposed; `cargo run --release
   -- build` and `mismatches`/`score` already print cost breakdowns by frequency tier — reuse that
   machinery rather than writing a new one).
2. For each such word, compare its current (rule-generated) outline's key count against Plover's
   own outline's key count. Flag words where Plover is meaningfully shorter (say, 2+ keys
   shorter, or any case where Plover's outline is 3 keys or fewer) — these are the "is this a
   brief that was hand-picked by Plover's own designers, not something that falls out of
   frequency" candidates.
3. **Write down an explicit keep/reject rule**, since the user specifically asked for one, not ad
   hoc judgment calls:
   - Reject if adopting it would collide with a word that already has that stroke (check `taken`/
     rebuild and look for collision warnings — `cargo run --release -- build`'s own output already
     surfaces this).
   - Reject if the saving is marginal (e.g. one key) on a word rare enough that the saving barely
     moves the frequency-weighted cost — this is exactly the kind of judgment call rule 1 of
     `docs/theory.md` and this session's brief-correctness work already made for the common tier;
     extend the same reasoning down into the rarer tier rather than inventing a new standard.
   - Keep if it's a genuinely large saving (3+ keys) on a word common enough in the 1,500–20,000
     band to matter, and the Plover outline is a *whole* key/cluster (not a fragment — reuse
     `matches_real_consonants`'s logic, already in `src/rules.rs`, to check this mechanically
     rather than by eye).
4. Add the ones that pass as explicit `[word]` overrides in `rules/briefs.toml`, each with a
   one-line comment following the file's existing style (see `want = "W-T"`, `and = "SKP"` for the
   comment convention — explain *why*, not just *what*).

## Verification

Same as every other change to this repo this session:
```
cargo build --release
cargo test --release
cargo run --release -- build
cargo run --release -- score --check-readme    # regenerate and paste back any table it flags as stale,
                                                # including the two hand-typed ones near the bottom of
                                                # README.md that --check-readme doesn't cover itself
cargo clippy --release --all-targets
```

Report the actual before/after numbers honestly (`cargo run --release -- score`), the way every
other commit in this repo's recent history does — don't round favorably, and say plainly if the
net effect is smaller than hoped (adding briefs this far down the frequency list has rapidly
diminishing returns on the *weighted* headline number, even if it's a real win for the specific
words typed).

## Docs

If this changes how the brief *window* is described (not just which specific words are briefed),
update `docs/theory.md` rule 1/2 (whichever rule number briefs ended up at — check the current
numbering, it's been renumbered more than once this session) to describe the keep/reject rule you
wrote, the same way the fragment-vs-dropped-sound rule got written up there.

## Deliverable

Commit(s) on `main`, pushed. Regenerate and redeploy to the live config:
```
cargo run --release -- build
cargo run --release -- plugin
cp out/lyk.json ~/.config/keymux/steno/lyk/02-lyk-words.json
cp out/lyk-orthographic-fallback.rs ~/.config/keymux/steno/lyk/05-lyk-orthographic-fallback.rs
```
