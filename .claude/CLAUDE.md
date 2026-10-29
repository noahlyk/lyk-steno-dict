# lyk-steno-theory

**Docs outrank code here.** The user has said this explicitly and repeatedly: *"DOCS ARE THE MOST
IMPORTANT THING theory is good but the most important thing is a theory you can implement, I want
100% exhaustive documentation of rules and things I can learn to actually be able to type my
theory."*

Any change to `rules/*.toml` or `src/*.rs` that changes what a word strokes as, what a brief does,
or how the generator picks between candidates **must** land with the matching doc update in the
*same commit* — not as a follow-up. `docs/theory.md` has gone stale multiple times from exactly
this gap: a rule changed in code before the docs caught up.

`cargo run --release -- score --check-readme` only catches numeric drift in `README.md`'s tables.
It does **not** catch prose drift — a rule described in `docs/theory.md` that no longer matches
what the code does. There's no automated check for that; it has to be done by hand, every time, as
part of the same change.

Start at [`docs/README.md`](../docs/README.md) to learn the theory — read in the order it lays
out, not at random.
