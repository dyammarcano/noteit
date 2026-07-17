# Hardening Phase 1 (Stabilize) — Report

## STATUS: DONE

## Commits
- H-01: `d222b8c` — fix(cli): preserve note text and use a secure temp file in $EDITOR flow (H-01)
- H-02: `5ae98a1` — fix(cli): report temp path on non-UTF-8 editor content (H-02)
- H-03: `947ccf0` — fix(cli): keep same-named projects contiguous in grouped --global (H-03)

## Test counts (baseline 71)
- After H-01: 72 passed (added `editor_nonzero_exit_preserves_typed_text` in new `tests/editor.rs`)
- After H-02: 73 passed (added `editor_invalid_utf8_surfaces_the_path` in `tests/editor.rs`)
- After H-03: 74 passed (added `render_grouped_keeps_same_named_contexts_separate` in `tests/render.rs`)

All suites green after each commit (`cargo test`), including doc-tests.

## Details

### H-01
- `edit_in_editor` (`src/cli.rs`) now uses `tempfile::Builder::new().suffix(".md").tempfile()?.into_temp_path()`
  instead of a predictable `noteit-<pid>.md` path in the OS temp dir.
- Promoted `tempfile = "3.27"` from `[dev-dependencies]` to `[dependencies]` in `Cargo.toml` (removed the now-redundant
  dev-dependencies entry).
- On a non-zero editor exit, the file is no longer deleted. The existing content is read back; if non-empty, the
  temp file is persisted via `TempPath::keep()` and the returned error message includes the preserved path, e.g.
  `"{editor} exited with {status}; your note text was preserved at {path}"`. Only a genuinely empty result is
  discarded silently (matching prior no-op behavior for an aborted, unedited note).
- On Windows, `NamedTempFile` (open file handle) could not be reopened by the external editor process
  (`ERROR_SHARING_VIOLATION`); switching to `into_temp_path()` immediately after creation closes our handle
  while keeping the same secure, unpredictable path and delete-on-drop semantics, which resolved this.
- New `tests/editor.rs` spawns a real fake-editor process (a generated `.cmd` script on Windows) that appends
  text to the file path it's given and exits with a chosen code. `editor_nonzero_exit_preserves_typed_text`
  confirms: (a) `edit_in_editor()` returns `Err`, (b) the error message contains a path, (c) that path exists on
  disk, and (d) its contents contain the exact text the fake editor wrote — proving no data loss on non-zero exit.
  This is a real-process test, not a fallback unit test of decision logic alone.

### H-02
- On a `read_to_string` failure (e.g. invalid UTF-8) after a zero-exit-code edit, the temp file is now persisted
  via `TempPath::keep()` and the error includes the path:
  `"could not read note from {path} (invalid UTF-8): {err}; the file was left in place"`.
- `editor_invalid_utf8_surfaces_the_path` test: fake editor writes `0xFF 0xFE` bytes and exits 0; asserts the
  error message contains a path and that the path still exists on disk.

### H-03
- Both `--global` list branches (tag-grouped at ~line 297 and the plain-grouped branch at ~line 322) now sort by
  `(display_name, ctx.id, created_at desc)` instead of `(display_name, created_at desc)`, guaranteeing rows for
  any two distinct contexts stay contiguous even when they share a `display_name`. The flat branch (`render_flat`)
  is untouched, per spec.
- New `render_grouped_keeps_same_named_contexts_separate` test in `tests/render.rs`: builds two `Context`s with
  identical `display_name` ("app") but different `id`s, interleaves rows, pre-sorts with the new key, and asserts
  `render_grouped` output contains exactly 2 un-indented "app" header lines (not 4, which would happen if rows were
  interleaved under the old sort key).

## Concerns
None. The real-process fake-editor approach worked on Windows for both H-01 and H-02 (no fallback to a
decision-logic-only unit test was needed). The one Windows-specific wrinkle (file sharing violation with
`NamedTempFile`'s open handle) was resolved by using `into_temp_path()`, which is a clean, idiomatic fix rather
than a workaround.
