# Plan: expand the suffix table

Repo: `/home/fib/code/noahlyk/lyk-steno-theory` (plain git checkout, branch `main`, remote
`origin` = `github.com/noahlyk/lyk-steno-theory`). Commit and push directly when done — no
worktree isolation needed here.

## Why

The user has asked twice for "WAY more suffixes" — `rules/suffixes.toml` currently has ~20
entries and hasn't grown since early in the session that produced this plan. Every suffix added
here is a continuation stroke any briefed or rule-generated word can pick up "for free" even if
the word-generation pass never happened to need it while building the dictionary — so this is
high-leverage, low-risk work: it only adds reachability, it doesn't change any existing word's
outline.

## The one rule (already established, don't reinvent it)

Read `docs/suffixes.md` in full before starting — it documents the rule this file follows:

> A suffix is a syllable like any other, so it's written by running its own sounds through the
> exact tables `rules/plover.toml` already uses for any syllable (`[onset]`, `[vowel]`, `[coda]`,
> `[coda_cluster]`, `[vowel_coda]`) — nothing in `rules/suffixes.toml` is a key combination
> invented just for this file. Its vowel is then dropped, the same way `[reduce]` already drops a
> whole word's own reduced final syllable when it folds onto a continuation stroke of its own,
> **whenever that still tells the suffix apart from every other suffix stroke already claimed**.

When dropping the vowel would collide with an existing suffix's own stroke, keep the vowel instead
(see the "kept" rows in `docs/suffixes.md` for examples: `^US` for "-ous", `^EUFT` for "-ist").
Every new entry needs the same kind of one-line justification comment the existing file uses —
see `rules/suffixes.toml`'s own comments for the style (explain which sound reduces to what bare
key combo, and why the vowel stays if it does).

## What to add — `docs/suffixes.md`'s own "Not yet covered" list

This is the starting list, already identified and written down by a previous pass, never
implemented:

- `-tive` (e.g. *active*, *sensitive*)
- further `-ous` spelling variants beyond what `^US` already covers (e.g. *-ious*, *-eous* — check
  whether these reduce to the exact same bare keys as the existing `^US`/"ous" entry, in which case
  they may not need a separate entry at all, just a note that the existing one already covers them)
- `-ic`/`-ical` (e.g. *electric*, *electrical*)
- `-al` (e.g. *national*) — note `-ial`/`-ual` may be the same reduced sound, check
- `-logy`/`-graphy`/`-ography` (e.g. *biology*, *photography*)
- `-ity`/`-ety` (e.g. *clarity*, *variety*)

Also worth checking while in here (not in the original list, but the same category of gap):
- `-cy`/`-sy` (e.g. *privacy*, *fantasy*)
- `-ify`/`-fy` (e.g. *clarify*, *simplify*) — almost certainly two syllables, follow the `-ism`
  two-stroke pattern already in the file (`^EUZ/^-PL`) rather than trying to force it into one

## Prefixes — a related, smaller gap in a different table

Not in `rules/suffixes.toml` at all — prefixes with their own keys live in `rules/plover.toml`'s
`[[prefix]]` array (TOML array-of-tables, not the suffix file). Current state, the whole table:

```toml
[[prefix]]
spelling = "ex"
sounds = ["IH0 K S", "EH0 K S", "IH0 G Z", "EH0 G Z", "EH1 K S", "EH2 K S", "EH1 G Z", "IH1 K S", "IH1 G Z"]
keys = "KP"

[[prefix]]
spelling = "dis"
sounds = ["D IH0 S", "D IH1 S", "D IH2 S", "D IH0 Z"]
keys = "STK"
```

Only two prefixes exist. Worth adding, each needing its own `sounds` list (every stressed/
unstressed ARPABET pronunciation variant CMUdict might give it, following the `ex-`/`dis-` entries'
own pattern) and a `keys` stroke that doesn't collide with anything: `un-`, `re-`, `pre-`, `non-`,
`over-`, `under-`, `mis-`, `inter-`. Read `src/rules.rs` for how `[[prefix]]` entries are consumed
(search for `prefixes:` and `Prefix`) before picking new `keys` values, and check each candidate
stroke against the built dictionary for collisions the same way the rest of this plan does (see
Verification below) before committing to it.

## Verification, every addition

```
cargo build --release
cargo test --release                              # expect the current count passing, no new failures
cargo run --release -- build                       # regenerates out/lyk.json, prints collision warnings
cargo run --release -- score --check-readme        # README's tables are auto-checked; if this fails,
                                                    #   regenerate and paste the corrected tables back in,
                                                    #   same pattern as every other commit this session
cargo clippy --release --all-targets               # compare warning count to before your change
```

Check each new suffix stroke doesn't collide with an existing word or suffix before committing it:
```
cargo run --release -- resolve <STROKE>
```
should say "not in the dictionary" (or show the suffix itself if you're testing it standalone) —
if it already resolves to some unrelated word, pick a different stroke or keep the vowel instead
of dropping it, per the rule above.

## Docs

`docs/suffixes.md` is the readable table over `rules/suffixes.toml` — **update it alongside every
addition**, matching its existing table structure (bare-consonant suffixes / kept-vowel suffixes /
real-vowel suffixes / two-syllable suffixes), and shrink or remove the "Not yet covered" section as
items get checked off. Don't let the prose drift from the TOML — this user has repeatedly said docs
are the most important thing in this repo, more important than the code itself.

## Deliverable

One commit (or a few logically-grouped ones) on `main`, pushed, with `rules/suffixes.toml`,
`rules/plover.toml` (prefixes), and `docs/suffixes.md` all updated together. Regenerate and
redeploy the dictionary and WASM plugin to the user's live config afterward:

```
cargo run --release -- build
cargo run --release -- plugin
cp out/lyk.json ~/.config/keymux/steno/lyk/02-lyk-words.json
cp out/lyk-orthographic-fallback.rs ~/.config/keymux/steno/lyk/05-lyk-orthographic-fallback.rs
```
Diff both copies against `out/` to confirm the deploy landed before finishing.
