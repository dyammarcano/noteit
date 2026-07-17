# BACKLOG
<!-- rev:002 -->

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

## Double-counted queries on `list` / `search` / `--tag`

Each of `list`, `search`, and `list --tag` runs one unlimited query to get an
accurate `total` count, then a second, limited query to fetch the rows
actually rendered — two scans per invocation. Acceptable for v1 given
expected note-table sizes, but worth collapsing into a single query (e.g.
`COUNT(*) OVER()`) if databases grow large enough for this to matter.

## Deferred by design (from the original spec's out-of-scope list)

- Upward repo walk (searching parent directories for a repo root when the
  cwd itself isn't one).
- Sync (multi-machine note synchronization).
- Encryption of the SQLite database at rest.
- Attachments (non-text note content).
- Real note deletion — `done`/`open` toggle status; there is no hard delete.

## Shallow-clone submodule adoption

A shallow nested repo (submodule) is correctly *not* adopted — the submodule
guard compares repo roots and skips it. But if the user later runs
`git fetch --unshallow` on that submodule, making it a full, identifiable
repo, its notes remain stuck in their original path context: there is no
re-check that would fold them in at that point. Minor, since it only affects
users who deliberately un-shallow a submodule after the fact.
