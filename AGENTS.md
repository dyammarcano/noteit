# AGENTS.md
<!-- rev:003 -->

Instructions for agents (and human contributors) working in this repo.
`noteit` is a Rust CLI that captures notes bound to a git repo's identity
(or, failing that, to a directory path). See `README.md` for user-facing
behavior first — this file is about building, testing, and the hard
constraints that must not be "optimized away".

## Build & test

```powershell
cargo build
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings   # clippy-deny gate is active
cargo audit                                 # dependency advisory scan
```

The test suite currently has ~160 tests across `tests/adoption.rs`,
`tests/cli.rs`, `tests/config.rs`, `tests/context.rs`, `tests/editor.rs`,
`tests/main_smoke.rs`, `tests/plugin.rs`, `tests/property.rs`,
`tests/render.rs`, `tests/repoid.rs`, `tests/run.rs`, and `tests/store.rs`,
plus unit tests in the library (including `src/plugin/*`). All gates above must
pass before committing; `[lints.clippy] all = deny` is enforced in
`Cargo.toml`.

Config is env-driven (no global flags, to keep the ambiguity rule clean):
`NOTEIT_DB` (database path), `NOTEIT_QUIET` (suppress stderr notices),
`NOTEIT_PLUGIN_ROOT` (plugin install root). `noteit export` dumps all notes as
JSON. CLI parsing lives in `src/cli/parse.rs`; runtime dispatch in
`src/cli/mod.rs`.

## Plugin surface

`src/plugin/*` is a std-only plugin-host contract (ported — see `docs/port/`)
plus noteit's own assets and host backends. It powers `noteit plugin
install|list|status|doctor|uninstall --host <claude|codex|gemini|all>`, which
renders bundled assets into `$HOME/.<host>/plugins/noteit/`
(`NOTEIT_PLUGIN_ROOT` overrides the home dir). Plugin ops are filesystem-only
and dispatched in `cli::run` *before* the DB opens.

## The `.scripts/` convention

Every git/cargo command in this repo is written to a script under
`.scripts/` and executed — never run inline. Naming:
`{NUM}-{LETTER}_{verb}_{target}.ps1`, e.g. `21-B_commit_docs.ps1`. `{NUM}` is
a zero-padded sequence number within a batch; `{LETTER}` is the batch tag,
derived from whatever already exists in `.scripts/` (never chosen at
random). As of this writing, batch `B` is in use through roughly `41-B`; a
new batch increments the letter rather than reusing `B`. `.scripts/` is
ephemeral (gitignored) and holds only executable scripts — never docs,
plans, or data.

## Hard constraints — do not "fix" these

### `gix` must keep its DEFAULT features

`Cargo.toml` depends on `gix = "0.85"` with no `--no-default-features`
tweaking. This is deliberate and has been empirically verified:
`--no-default-features --features max-performance-safe` does **not** compile
on gix 0.85 — `gix-hash 0.25.1` fails with 16 `E0004` "non-exhaustive
patterns" errors on its own `Kind` enum. Do not attempt to slim this
dependency's feature set to "optimize" build size or time; it breaks the
build.

### No `clap`

There is intentionally no argument-parsing crate dependency. `noteit`'s
central design rule — a first argument matching a known verb dispatches that
verb, anything else is note text to capture — actively fights how `clap` (or
any conventional parser) wants to work. Parsing in `src/cli.rs` is
hand-rolled by design; do not introduce `clap` "to clean it up".

### Errors are enums, never fail-open to `Option<T>`

`RepoIdError` (see `src/repoid.rs`) must keep its distinct variants —
notably `NoCommits`, `NotARepo`, and `Shallow` are semantically different
outcomes. The context-resolution ladder in `src/context.rs` branches on
which specific variant it got to decide whether to warn, fall back to a path
context, or (for `Shallow`) warn *and* fall back. Collapsing these into a
single `Option<RepoId>` or a generic error would break the shallow-clone
warning path and the adoption ladder's ability to distinguish "not a repo at
all" from "a repo, but not yet identifiable".

## Shared helpers — keep them in ONE place

- `now()` lives in `src/store/mod.rs`. Do not re-implement a timestamp
  helper in another module.
- `row_to_context` and the `SELECT_COLS`/`CTX_COLS` column-list constants
  live in `src/store/contexts.rs`. Any new query against the contexts table
  should reuse these rather than duplicating column lists inline.

## Never lose a note

Every failure mode in repo detection (not a repo, no commits yet, shallow
clone, git error) degrades gracefully to a path context — a note is always
captured. The only failures that should stop the program entirely are
opening the database or running its migrations; those are treated as hard
failures precisely because silently degrading there risks creating a second,
inconsistent database.

## Where to look for the design rationale

- `docs/ARCHITECTURE.md` — module map + capture/plugin flow diagrams (the
  current-state architecture reference).
- `docs/superpowers/specs/2026-07-17-noteit-design.md` — the original design
  spec (repo-id algorithm, schema, resolution ladder, adoption rules).
- `docs/superpowers/plans/2026-07-17-noteit.md` — the task-by-task
  implementation plan this codebase was built from.
- `.superpowers/sdd/task-*-report.md` — per-task implementation reports,
  including the smoke-test transcript proving adoption and cross-clone
  survival actually work.

Where the spec and the as-built code disagree (a few corrections were made
during implementation — e.g. `--help`/`--version` handling, exit-code
alignment between `Capture` and `New` on empty bodies, removal of a
redundant re-resolve after adoption), the code and its task reports are
authoritative; treat the spec as historical design intent, not a literal
current-state description.
