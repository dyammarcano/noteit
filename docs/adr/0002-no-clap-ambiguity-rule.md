# ADR-0002: No `clap`; hand-rolled ambiguity rule

- **Status:** Accepted
- **Date:** 2026-07-17

## Context

noteit's central UX rule: a first argument matching a known verb dispatches that
verb; **anything else is note text to capture**. `noteit search this` searches,
but `noteit fix the parser` captures a note. This "first-arg is a verb XOR note
text" rule is fundamentally at odds with how `clap` (or any conventional parser)
wants to model subcommands and positional args.

## Decision

No argument-parsing crate. Parsing lives in `src/cli.rs` as a hand-rolled
`parse()` over a closed `VERBS` list: if the first token is in `VERBS`, dispatch
that verb; otherwise the whole argv joins into a captured note. `add` is the
escape hatch for text that collides with a verb (`noteit add "search this"`).

## Consequences

- The ambiguity rule stays exact and legible instead of being bent around a
  parser's model.
- `--help`/`--version` are checked before the ambiguity rule so they're never
  captured as notes; a test guards this.
- A `VERBS`-vs-match-arm invariant must be kept in sync (guarded by a test and
  an `unreachable!`).
- We hand-roll flag parsing per verb (small surface); the tradeoff is accepted
  deliberately — do not introduce `clap` "to clean it up".
