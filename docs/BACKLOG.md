# BACKLOG
<!-- rev:006 -->

Deferred work identified during implementation of `noteit`. Nothing here
blocks the current release; each item is tracked so it isn't lost.

## Shipped

- **FTS5 query sanitization** — done. `Store::search` now routes the query
  through `sanitize_fts_query`, which wraps each whitespace token as a quoted
  FTS5 phrase (doubling embedded `"`) and ANDs them; an empty query short-
  circuits to zero results before any SQL. `noteit search "foo` and bare
  `AND`/`OR`/`NOT` are now literal terms, never a SQL error.
- **`noteit adopt --undo`** — done. Reverses the most recent automatic
  adoption: recreates the folded path contexts, moves the notes back, and
  *pins* those contexts (`no_adopt = 1`, via a v2 migration) so automatic
  adoption never re-folds them on a later run.
- **`noteit delete <id>`** (hard delete) — done. Permanent, **context-scoped**
  deletion (you can only delete a note visible in the current context — safer
  than done/open's global-by-id for a destructive op). No interactive prompt;
  prints what was removed. Cleanup rides the `notes_ad` FTS trigger +
  `note_tags ON DELETE CASCADE`. The one authorized exception to
  "never lose a note".
- **Shallow-clone self-adoption after `--unshallow`** — verified, no code
  change needed. A shallow repo's own notes bind to a path context (keyed at
  the repo dir itself). Once the user runs `git fetch --unshallow`, the next
  in-dir run resolves to a Repo context, and `adopt_if_needed` correctly
  folds that path context in: `path_contexts_under` matches on `key = root`
  (not only descendants), and the submodule guard's canonicalized-root
  comparison passes since `repo_root(dir) == dir` post-unshallow. Regression
  test: `tests/adoption.rs::shallow_repo_self_adopts_after_unshallow`.
- **Single-query `list` / `search` / `--tag`** — done. The count+fetch double
  query is collapsed to one scan via `COUNT(*) OVER()`: the four read methods
  return `(rows, total)`, computing `total` in the same limited query.
  Behavior unchanged (same rows, order, truncation notice).

## Deferred by design (from the original spec's out-of-scope list)

- Upward repo walk (searching parent directories for a repo root when the
  cwd itself isn't one).
- Sync (multi-machine note synchronization).
- Encryption of the SQLite database at rest.
- Attachments (non-text note content).

## Investigated — accepted, no action

- **Duplicate `hashbrown` versions (0.14.5 / 0.16.1 / 0.17.1).** All three are
  100% transitive through `gix`'s own crate family — `hashbrown 0.14.5` via
  `dashmap → gix-tempfile`, `0.16.1` via `clru → gix-pack`, `0.17.1` via
  `gix-hashtable`. noteit has **zero direct pull** on any of them, and
  `Cargo.toml` mandates `gix` default features, so this can only resolve when
  `gix` aligns its internal dependency versions upstream. Not actionable from
  this repo; re-check when bumping `gix`.
- **Dependency vulnerabilities.** `cargo audit` is clean (191 deps, 0
  advisories) and runs as a blocking CI job. No `audit.toml` ignore-list needed.
