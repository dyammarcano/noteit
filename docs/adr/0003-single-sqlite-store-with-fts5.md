# ADR-0003: Single SQLite store with FTS5

- **Status:** Accepted
- **Date:** 2026-07-17

## Context

noteit needs durable local storage for notes and contexts, fast full-text
search, and safe concurrent access from more than one shell — without a server
or an external database.

## Decision

One SQLite database at `%USERPROFILE%\noteit.db` / `$HOME/noteit.db`, via
`rusqlite` with the **bundled** SQLite (no system dependency). WAL mode with a
busy timeout makes concurrent captures safe. Full-text search uses an **FTS5
external-content** table kept in sync with the `notes` table by triggers. Schema
evolves through **append-only** `PRAGMA user_version` migrations (v1, v2
shipped) — a shipped migration is never edited; new schema adds v3+.

## Consequences

- Zero-setup, single-file storage that's trivial to back up or delete.
- Bundled SQLite means no "install sqlite" step, at the cost of a longer
  (and, under coverage instrumentation, much longer) compile.
- FTS5 has its own query grammar; raw user queries are routed through
  `sanitize_fts_query` so malformed input becomes literal terms, never a SQL
  error (see `tests/property.rs`).
- Append-only migrations keep old databases forward-compatible.
