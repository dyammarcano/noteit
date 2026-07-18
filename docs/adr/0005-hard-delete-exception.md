# ADR-0005: Hard delete as the one note-losing exception

- **Status:** Accepted
- **Date:** 2026-07-17

## Context

noteit's core safety invariant is **never lose a note**: every failure mode in
repo detection degrades to a working capture, and only a DB open/migrate failure
stops the program. But users legitimately need to remove a note they no longer
want. A destructive operation has to coexist with that invariant.

## Decision

`noteit delete <id>` is a **permanent, context-scoped** hard delete — the single
authorized exception to "never lose a note". Safeguards:
- **Context-scoped:** you can only delete a note visible in the current context
  (stricter than `done`/`open`, which are global-by-id), so you can't delete
  what you can't see.
- **Explicit + visible:** no interactive prompt (the CLI must never hang), but it
  prints exactly what was removed (`deleted <id>: <snippet>`).
- Cleanup rides existing machinery: the `notes_ad` FTS trigger and
  `note_tags ON DELETE CASCADE`.

## Consequences

- Users get real deletion without weakening the capture-always guarantee.
- The scoping inconsistency with `done`/`open` is accepted; retrofitting those is
  a separate backlog item.
- No soft-delete/undo for `delete` (unlike adoption's `--undo`); the id-scoping
  and printed snippet are the mitigations.
