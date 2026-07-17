# Spec — single-query list/search/tag

**Date:** 2026-07-17 · **Item:** autonomy backlog #2 · **Status:** Approved (autonomous)

## Purpose

`list`, `search`, and `list --tag` each run TWO queries per invocation: one
unlimited to compute `total` (for the truncation notice) and one limited to
fetch the rows shown. Collapse each into ONE query using `COUNT(*) OVER()`.
Purely a query-count reduction — **no observable behavior change**.

## Fork (settled — charter Decision Log)

Change the four read methods to return `(rows, total)` rather than a bare
`Vec`, computing `total` via a `COUNT(*) OVER()` window column in the same
limited query. Chosen over separate `count_*` methods (still two queries).

## API changes (`src/store/notes.rs`)

```rust
pub fn list_notes(&self, context_id, subpath, include_done, limit)
    -> Result<(Vec<Note>, usize), StoreError>
pub fn list_all_notes(&self, include_done, limit)
    -> Result<(Vec<(Context, Note)>, usize), StoreError>
pub fn search(&self, query, context_id, limit)
    -> Result<(Vec<(Context, Note)>, usize), StoreError>
pub fn notes_by_tag(&self, tag, context_id, include_done, limit)
    -> Result<(Vec<(Context, Note)>, usize), StoreError>
```

Each SELECT gains `COUNT(*) OVER() AS total` as a trailing column. Window
functions are evaluated over all rows matching WHERE/JOIN **before** `LIMIT`,
so `total` is the true match count. Read `total` from the first row; if zero
rows are returned, `total = 0`. The row mappers (`row_to_note` / joined
context+note mappers) ignore the extra trailing `total` column (read it
separately via the row, or select it first and offset the mappers — keep the
existing offsets stable by appending `total` LAST and reading it by explicit
index).

Implementation note: with `COUNT(*) OVER()` appended last, `row.get(N)` for the
total uses the final column index. Keep `row_to_note`/`row_to_context` offsets
unchanged (they read the leading columns); read `total` from the known last
index inside the `query_map` closure (capture it into a local on the first row,
or collect rows and pull total from any).

## Call sites (`src/cli.rs run_core`)

Replace each `let total = store.X(.., None)?.len(); let rows = store.X(.., limit)?;`
pair with a single `let (rows, total) = store.X(.., limit)?;`. The `--global`
branches that sort/truncate in Rust: the DB `total` is the full match count;
keep passing it to the render truncation logic unchanged.

## Tests

- Update existing `store.rs` tests that call these four methods to destructure
  `(vec, total)` and assert on `vec` as before, PLUS at least one assertion per
  method that `total` reflects the FULL match count even when `limit` truncates
  (e.g. insert 5 notes, `list_notes(.., Some(2))` → rows.len()==2, total==5).
- `run.rs`: existing truncation-notice tests must still pass (the notice is
  driven by `total` vs shown). Add one asserting the notice count is right when
  a limit truncates a real query.

## Out of scope

Changing `render_*` signatures, changing pagination semantics, indexing.
