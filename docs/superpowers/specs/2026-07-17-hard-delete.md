# Spec — `noteit delete <id>` (hard delete)

**Date:** 2026-07-17 · **Item:** autonomy backlog #1 · **Status:** Approved (autonomous)

## Purpose

Add a permanent delete. Today `done`/`open` only toggle status (soft retire);
there is no way to remove a note. This is the one authorized exception to the
"never lose a note" invariant — it removes a note the user *explicitly names*.

## Surface

```
noteit delete <id>
```
`<id>` is the short base36 id shown by `list`/`search` (same ids `done`/`open`
take). On success prints `deleted <id>: <body first line, truncated>` to
stdout, exit 0. If no such note is visible in the current context, prints
`no note with id <id>` to stderr, exit 1. Invalid id → exit 2 (matches the
`done`/`open` convention).

## Behavior (settled forks — see docs/AUTONOMY.md Decision Log)

- **Context-scoped:** deletes only a note whose `context_id` equals the current
  resolved context. You can only delete what you can see. (done/open are global
  by rowid — that inconsistency is accepted; a separate backlog Minor.)
- **No interactive prompt** (CLI must never hang). Immediate on the explicit
  `delete` verb; prints what was removed so the action is visible.
- **Cleanup is automatic:** the `notes_ad` AFTER-DELETE trigger removes the FTS
  row; `note_tags ON DELETE CASCADE` (with `foreign_keys=ON`, already set)
  removes tag links. Orphan `tags` rows are left (harmless).
- **Adoptions audit** needs no change: `note_ids` is CSV TEXT (not a FK), and
  undo already tolerates missing ids.

## Store API

```rust
/// Delete a note by id, but only if it belongs to `context_id`.
/// Returns the deleted note's body if a row was removed, else None.
pub fn delete_note(&self, id: i64, context_id: i64) -> Result<Option<String>, StoreError>
```
Implementation: `DELETE FROM notes WHERE id = ?1 AND context_id = ?2 RETURNING body`
(SQLite ≥3.35 supports RETURNING; bundled rusqlite 0.40 is new enough), read the
returned body via `query_row`, map `QueryReturnedNoRows` → `Ok(None)`.

## CLI wiring

- Add `"delete"` to `VERBS` (the `every_verb_in_VERBS_has_a_match_arm` test
  enforces a matching arm).
- `Invocation::Delete { id: String }`; `parse` arm mirroring `done`/`open`
  (error `DeleteNeedsId` if no id).
- In `run_core`: parse the short id (`render::parse_short_id`); `None` → stderr
  + `Ok(2)`. Call `store.delete_note(rowid, ctx.id)`; `Some(body)` → print
  `deleted {id}: {first line, ≤60 chars}` to `out`, `Ok(0)`; `None` → stderr
  `no note with id {id}`, `Ok(1)`.
- Update `HELP_TEXT` with the `delete` line.

## Tests

- `store.rs`: `delete_note_removes_a_note_and_returns_its_body`;
  `delete_note_is_context_scoped` (a note in another context is NOT deleted,
  returns None); `delete_note_also_clears_tags_and_fts` (after delete, a
  `--tag`/search for the note finds nothing, and `note_tags` has no rows for it).
- `run.rs`: `delete_success_exit_0`, `delete_not_found_exit_1`,
  `delete_invalid_id_exit_2`, and that a deleted note no longer appears in list.
- `cli.rs`: `delete_parses`, `delete_without_id_is_an_error`.

## Out of scope

Bulk delete, `delete --all`, undelete, pruning orphan tags, retrofitting
done/open to be context-scoped.
