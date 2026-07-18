# ADR-0006: CLI stability & deprecation stance (pre-1.0)

- **Status:** Accepted
- **Date:** 2026-07-18

## Context

noteit is at `v0.1.0`. Its user-facing contract is the CLI: the verb set
(`VERBS`), flags, exit codes, and the stdout/stderr output split. As features
land (e.g. `export`, `plugin`, config env vars), users and any scripting on top
of noteit need to know what they can rely on and how breaking changes will be
handled — but a pre-1.0 project also needs room to change its mind.

## Decision

While noteit is **pre-1.0 (`0.x`)**:

- **No hard CLI-stability guarantee.** Verb names, flags, and output format may
  change between `0.x` minor versions. Every such change is recorded in
  `CHANGELOG.md` under the release.
- **What is already stable in practice** and will not change lightly: the
  ambiguity rule (first arg is a verb XOR note text; `add` escape hatch — see
  ADR-0002), exit-code meanings (`0` success / `1` not-found / `2` usage), and
  the stdout=data / stderr=diagnostics split.
- **The database schema is stricter:** migrations are append-only and never
  edited (see ADR-0003) — on-disk data compatibility is preserved even while the
  CLI surface evolves.
- **When possible, deprecate rather than break:** keep an old flag/verb working
  alongside the new one for at least one `0.x` minor, emitting a stderr
  deprecation notice, before removal.

At **`1.0`** the CLI surface (verbs, flags, exit codes, output contract) becomes
a stability boundary: breaking changes require a major version bump and a
deprecation window.

## Consequences

- Contributors know what they may change freely now (CLI surface) vs what they
  must not (schema, the invariants above).
- Users get an honest expectation: script against `0.x` at your own risk, but
  the core invariants and your data are safe.
- A future `1.0` ADR will formalize the deprecation window and supersede this
  one.
