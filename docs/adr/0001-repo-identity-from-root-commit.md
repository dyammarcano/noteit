# ADR-0001: Repo identity from the root commit SHA

- **Status:** Accepted
- **Date:** 2026-07-17

## Context

noteit binds notes to the repository you run it in. A note taken in a repo must
still resolve to that repo after it is cloned to a new path, renamed, or moved —
so the identity key cannot be the filesystem path or the remote URL (both
change). It must be intrinsic to the repository's history.

## Decision

Identify a repo by the **lexicographically-smallest parentless root-commit SHA**
reachable from `HEAD`, formatted as `urn:noteit:v1:<sha>`. The SHA is read with
the `gix` crate (`src/repoid.rs`). Choosing the smallest root deterministically
handles histories with multiple root commits (e.g. merged unrelated histories).

`RepoIdError` is a rich enum (`NoCommits`, `NotARepo`, `Shallow`, …) — never
collapsed to `Option` — because `src/context.rs` branches on the specific
variant to decide whether to warn, fall back to a path context, or both.

## Consequences

- Notes survive clones/renames; the same repo always resolves to the same id.
- A shallow clone can't see its root commit, so it degrades to a path context
  with a warning until `git fetch --unshallow` (then it self-adopts).
- The `urn:noteit:v1:` prefix leaves room for a `v2` algorithm without
  reinterpreting existing ids.
- `gix` must keep default features (see ADR-0002 note in AGENTS.md); slimming
  them breaks the build on gix 0.85.
