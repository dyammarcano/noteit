# BACKLOG
<!-- rev:001 -->

Deferred work identified during implementation of `noteit` v1. Nothing here
blocks the current release; each item is tracked so it isn't lost.

## FTS5 query sanitization

`src/store/notes.rs` passes the raw user query straight into
`notes_fts MATCH ?1`. A search like `noteit search "foo` (an unbalanced
quote) or a bare `AND` / `OR` / `NOT` token is valid input from the user's
point of view but is also FTS5 query-syntax, so it can hit FTS5's own parser
and surface a raw SQL syntax error instead of a friendly "no results" or an
escaped literal match. Not a crash, but a real UX gap — quoting/escaping the
query before it reaches `MATCH` (or catching the parser error and treating it
as zero results) is the fix.

## `noteit adopt --undo`

The `adoptions` table already captures everything needed to reconstruct an
undo: the folded context's identity (`from_key`, `from_root_path`,
`from_display_name`, `from_name_overridden`) plus which note ids moved. The
audit rows are written on every fold; the undo command itself is not
implemented yet.

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
