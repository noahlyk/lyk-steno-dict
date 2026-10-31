# Phonetic rules

[← back to the table of contents](README.md) · [← back to the main README](../README.md)

**Phonetic rules** (`rules/plover.toml`). Where each sound goes on the keyboard in Plover
theory: consonants at the start of a syllable, the vowel, consonants at the end, and the
clusters that fold (`NG K` is `*PBG`, `S T` is `-FT`). A vowel with an `R` of its own (`ER`,
`AR`) shares the key with a cluster that has it too: `serve` SEFRB, `march` PHAFRPB.

These are the base tables everything else in the theory sits on top of: a word's own stroke by
running its sounds through these tables — onset, vowel, coda, and the clusters above — is always
computed and always competes, brief or not (see **[briefs](briefs.md)**) — and the chapters that
follow this one (spelling, prefixes, syllables, brevity) are all refinements layered on top of this
same base, not replacements for it. `Rules::resolve` (see **[the fallback](fallback.md)**) reads
these same tables the other way, turning a stroke back into sounds.

Next: **[spelling disambiguation](spelling.md)** — picking between more than one way to write the
same sound.
