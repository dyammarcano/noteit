# Autonomy Charter — noteit
<!-- rev:001 -->

Standing authority for autonomous execution (`/steps:autonomous`), granted
2026-07-17. The phased roadmap is complete; this charter governs driving the
remaining approved backlog hands-free.

## Envelope (granted once by the operator)

- **Scope:** the two actionable backlog items **plus `noteit delete <id>`**
  (hard delete, the most-requested deferred-by-design item). Order:
  1. `noteit delete <id>` — permanent note deletion.
  2. Single-query `list` / `search` / `--tag` (collapse the count+fetch double
     query, e.g. `COUNT(*) OVER()`).
  3. Shallow-submodule re-adopt after `git fetch --unshallow`.
  Then stop. `sync`, `encryption`, `attachments`, `upward repo walk` stay OUT
  of scope (genuinely deferred by design; pulling them in is a product call).
- **Design forks:** decide-and-log, keep going. Every fork gets a rationale in
  the item's spec ("Settled forks") and this charter's Decision Log.
- **Guardrails — allowed without asking:** merge verified branches to `master`;
  build/run the real binary to smoke-test; push merged work to the public
  `origin` (plain `git push`, gate-compliant).
- **Check-in cadence:** report at each item boundary.

## Guardrails — NEVER without explicit operator say-so

- Rewrite/delete git history, force-push, `reset --hard` shared refs.
- Delete operator data/files this work didn't create.
- Publish anywhere but the authorized `origin` (github.com/dyammarcano/noteit).
- Spend money / paid cloud services.
- Change the license or the project's safety invariants (below).
- Pull `sync`/`encryption`/`attachments`/`upward-walk` into scope.

## Project safety invariants (must survive every change)

- **Never lose a note.** Every failure degrades to a working capture; only DB
  open/migrate failure stops the program. (Hard delete is the ONE authorized
  exception — it removes a note the user explicitly named.)
- Repo-detection failures degrade to a path context; `RepoIdError` stays an
  enum, never fail-open-to-`None`.
- Single SQLite DB at `%USERPROFILE%`/`$HOME`; WAL; migrations append-only
  (v1/v2 shipped — never edit a shipped migration; add v3+ for new schema).
- FTS5 stays in sync via triggers; `#tag` stored both in body and table.
- Notes → stdout; warnings/adoption notices → stderr.
- Gates stay green: `cargo test`, `cargo fmt --check`, `cargo clippy
  --all-targets -- -D warnings` (clippy-deny gate is active), `cargo audit`.
- No AI attribution in commits; scripts-first (`.scripts/`), Windows/PowerShell.

## Per-item cycle

spec (+settled-forks) → self-review → plan → SDD execute (fresh implementer
per task, task review, fix loop) → whole-branch review → independent verify
(direct tool run: build+test+fmt+clippy; Codex agent unavailable in this env,
so a fresh Claude verifier or direct run is the green gate) → merge to master
→ smoke-test the real binary → docs checkoff → push → report at item boundary.

## Stop conditions

Genuine unresolvable blocker; a NEVER guardrail or unauthorized gated action
required; scope satisfied (3 items shipped). On stop → wrap-up summary.

## Decision Log (newest first)

_(forks settled autonomously appended here as work proceeds)_

### Item 1 — `noteit delete <id>` — forks settled 2026-07-17
- **Scoping:** delete is **scoped to the current resolved context** (only a
  note whose `context_id` matches the current context can be deleted). Safer
  than the global-by-rowid behavior of `done`/`open` for a destructive op —
  you can only delete what you can see. Inconsistency with done/open scoping
  is accepted; retrofitting done/open scoping stays a separate backlog Minor.
- **Confirmation:** no interactive prompt (the CLI is non-interactive by
  design and must never hang). Delete is immediate on the explicit `delete`
  verb, and prints what was removed (`deleted <id>: <body snippet>`) so the
  action is visible.
- **FTS/tags cleanup:** rely on the existing `notes_ad` AFTER-DELETE trigger
  (removes the FTS row) and `note_tags ON DELETE CASCADE` (requires
  `PRAGMA foreign_keys = ON`, already set at open). Orphaned `tags` rows are
  left in place (harmless; pruning them is out of scope).
- **Adoptions audit:** `adoptions.note_ids` is CSV TEXT, not a FK, and undo
  already tolerates missing ids (UPDATE matches 0 rows). Deleting a note needs
  no audit-table change.
