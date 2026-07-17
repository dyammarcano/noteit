# noteit — Design

**Date:** 2026-07-17
**Status:** Approved
**Target:** `D:\rust\noteit` (greenfield — empty scaffold: `Cargo.toml` with no deps, edition 2024, hello-world `main.rs`)

## Purpose

A Rust CLI for capturing ideas that auto-binds each note to the directory context it was written in. Run `noteit` inside a repo and you see that repo's notes; run it anywhere else and you see that directory's notes. A global view spans everything.

The problem it solves: ideas about a project occur while you are *in* the project, but general-purpose note tools make you file them manually. Manual filing is the step people skip, so the note is lost. noteit removes the step.

## Core premise

**A note binds to a repository's identity, not its location.** Clone the repo to another drive, rename the directory, move it — the notes follow. This is the feature the whole design serves; if it does not hold, noteit is a worse text file.

## Repo identity

### Algorithm

The repo id is the **lexicographically smallest parentless (root) commit SHA reachable from HEAD**, prefixed `urn:noteit:v1:`.

Adopted from lensr, verified by direct source scan:

- `D:\lensr\apps\lensr_git\csrc\git_repo_id.c:38-74` — `compute_root()`: resolve HEAD, revision-walk, keep commits where `commit->parents == NULL`, select min by `strcmp(hex, best) < 0`.
- `D:\lensr\apps\lensr_git\csrc\git_repo_id.c:84-114` — `lensr_git_project_root()`: forces SHA-1, discovers the git dir without chdir, rejects shallow, then `compute_root`.
- `D:\lensr\apps\lensr_git\src\repoid.rs:24-45` — validates all-ASCII-hex, prefixes the URN.

The id derives from the repo's own history, which is why it is location-independent.

### Implementation: `gix`, not linked git C

lensr obtains this by statically linking git's own C source (`build.rs` merges `libgit.a` into the binary; mingw-only; machine-local hardcoded default paths). It is also a `[[bin]]` package with **no `lib.rs`**, so nothing can depend on it.

noteit re-implements the algorithm over the `gix` crate. **Parity is defined by the SHA payload, not the implementation** — a `gix` port is contract-compatible as long as min-root-hex selection and shallow-rejection match.

### Namespace

noteit uses `urn:noteit:v1:`, not `urn:lensr:v1:`. The SHA payload is identical, so the SHA is the cross-tool join key if that is ever wanted; the prefix is ours because noteit is not lensr.

### Known and accepted: HEAD-relative instability

The id is HEAD-relative. A branch not containing the usual root (an orphan branch) computes a **different** id. This is accepted, not fixed. Notes store their resolved context at capture time and the id is **never recomputed for an existing note**, so notes stay where they landed.

## Context model

### Two kinds, one concept

A context is a bucket notes hang off. It is keyed either by repo id or by absolute path. These are the same concept differing only in key derivation.

- **repo context** — key is the URN; notes additionally record `subpath` (relative to repo root).
- **path context** — key is the absolute path. For directories that are not repos, or repos whose id is not yet available.

### Resolution ladder

Evaluated once per run, before verb dispatch, for every command including capture:

| Condition | Result |
|---|---|
| `project_id(cwd)` → `Ok(id)` | repo context; `subpath` = cwd relative to repo root |
| `Err(NoCommits)` / `Err(NoHead)` | path context |
| `Err(Shallow)` | path context, warn once (recoverable via `git fetch --unshallow`) |
| `Err(NotARepo)` | path context — no upward walk |
| unexpected error / panic | path context; print the error; never crash on capture |

`repoid::project_id(dir) -> Result<RepoId, RepoIdError>` returns an **enum** error, deliberately diverging from lensr, which fails open to `None` (`repoid.rs:24`) and collapses every failure into one indistinguishable case. noteit cannot: `Shallow` warns and is recoverable via `git fetch --unshallow`, while `NoCommits` and `NotARepo` are silent normal states. Distinct behavior requires distinct variants.

### Adoption

When a directory with path contexts at or under it gains a repo id, those contexts fold into the repo context.

**All path contexts adopt — there is no permanent path context.** Every rung of the ladder is provisional, because any directory can become a repo: `NoCommits` gains a commit, `Shallow` gains history via `--unshallow`, and a `NotARepo` directory gains a `git init`. Distinguishing "adoptable" from "permanent" path contexts would strand the `NotARepo → git init` case, which is the single most common way a project starts. The error variants drive *messaging*, not adoptability.

- **Trigger:** automatic on any run, **announced** — `adopted 7 notes from 3 paths into noteit`. Automatic because manual adoption taxes every new project; announced because it moves data between scopes and a wrong fold must not be invisible.
- **Scope:** all path contexts at or under the repo root. **Skip any path whose own `project_id` differs** (submodule guard) — otherwise a submodule's notes are swallowed by the parent.
- **Mechanism:** single transaction; `UPDATE notes SET context_id=?, subpath=?` — notes never change tables, so note identity survives. Subpath is derived from the path context's stored absolute path relative to the repo root.
- **Idempotent:** re-running after a crash is safe.
- **Audited:** each fold writes an `adoptions` row (`from_context_id`, `to_context_id`, note ids, timestamp), enabling a future `adopt --undo`. Cheap now, impossible to reconstruct later.

Live example: `D:\rust\noteit` is a git repo with zero commits, so noteit's own first notes exercise this path.

## Schema

```sql
contexts(
  id INTEGER PRIMARY KEY,
  kind TEXT NOT NULL CHECK(kind IN ('repo','path')),
  key TEXT NOT NULL,              -- repo urn, or absolute path
  display_name TEXT NOT NULL,
  name_overridden INTEGER NOT NULL DEFAULT 0,
  root_path TEXT NOT NULL,        -- repo root, or the path itself
  shallow_warned INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  UNIQUE(kind, key)
)

notes(
  id INTEGER PRIMARY KEY,
  context_id INTEGER NOT NULL REFERENCES contexts(id),
  subpath TEXT NOT NULL,          -- '.', 'src', ... relative to root_path
  body TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'open' CHECK(status IN ('open','done')),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
)

tags(id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE)
note_tags(note_id REFERENCES notes(id), tag_id REFERENCES tags(id), PRIMARY KEY(note_id, tag_id))

adoptions(id, from_context_id, to_context_id, note_ids TEXT, adopted_at)
```

Index on `notes(context_id, subpath)`.

**FTS5** external-content table over `notes.body`, kept current by insert/update/delete triggers. Search scopes by joining back to `contexts`.

**Tags** live in a table rather than a delimited string, so `--tag` is an index hit and tag listing needs no parsing. `#tag` is parsed out of the body at capture time and stored **both ways** — the body keeps them for display fidelity, the table drives queries.

**Migrations** via `PRAGMA user_version`, applied in order at startup. No external migration tool.

### Rejected alternatives

- **Two tables (`repo_contexts` / `path_contexts`)** — `notes.context_id` could not be a foreign key to either, every query becomes a UNION, and adoption rewrites the pointer's *meaning*. Type safety bought with hand-rolled polymorphism.
- **Denormalize onto notes** — the display-name override has no single home: rename means an UPDATE across every note, and a new note re-derives the basename and silently reverts it.

## Project display names

lensr derives **no** project name — the scan confirmed no name derivation and no language detection anywhere. noteit must supply this itself, because the grouped `--global` view needs a heading and `urn:noteit:v1:a3f9c2…` is not one.

**Source: repo root directory basename, user-overridable** via `noteit project rename <name>`. The override is stored on the context row and always wins.

The name is **display-only**. It never keys anything, so a rename can never split or lose notes — the repo id remains the identity. That is what makes an occasionally-wrong default safe.

### Rejected alternatives

- **Remote URL last segment** — accurate when a remote exists, but many repos have none (this one has no commits, let alone a remote), so it needs the basename fallback anyway: two mechanisms for one job.
- **Manifest `name` field** — would name this project **`app`**, since `D:\rust\noteit\Cargo.toml` declares `name = "app"` while the directory is `noteit`. Also forces a per-ecosystem parser and is ambiguous in monorepos.

Neither alternative can name a **path context** at all, since a non-repo directory has neither a remote nor necessarily a manifest.

## CLI surface

```
noteit "fix the FTS5 tokenizer"    # capture
noteit                             # list current context
noteit add "search"                # capture verb-colliding text
noteit new                         # capture via $EDITOR
noteit search <query> [--global]
noteit done <id> | noteit open <id>
noteit list --global [--flat] [--tag x] [--all] [--limit N]
noteit project rename <name>
```

**Ambiguity rule:** a first argument matching a **known verb** dispatches that verb; anything else is note text. The verb list is small, closed, and known at parse time, so the rule is unambiguous despite looking magical. `noteit add` is the escape hatch for capturing text that collides with a verb. Implemented via `clap` external-subcommand fallback or a manual pre-parse.

- `--all` on list, because open-only is the sane default once status exists.
- `done` / `open` take the **short display ids** printed by `list`, not raw rowids.
- **Global view:** grouped by project by default; `--flat` for a chronological timeline.
- **Output cap:** list caps at 50 rows; `--limit N` overrides, `--limit 0` for all. Prints `… 340 more (--limit 0 for all)`. `--global --flat` can return thousands of rows and silent truncation would read as completeness.

## Module layout

`lib.rs` + thin `main.rs` — **not** because noteit needs to be a library today, but because lensr's repo-id logic is good and unreusable purely because it is locked in a `[[bin]]` with no `lib.rs`. We are re-implementing it for exactly that reason.

| Module | Responsibility | Depends on |
|---|---|---|
| `repoid` | Root-commit walk → URN. Shallow → error. | `gix` |
| `context` | cwd → resolved context. Owns the fallback ladder and adoption. | `repoid`, `store` |
| `store` | SQLite: schema, migrations, CRUD, FTS5, tags. | `rusqlite` |
| `cli` | Verb dispatch, ambiguity rule, `$EDITOR`. | `clap` |
| `render` | Grouped/flat output, short ids. | — |

**Data flow, every run:** `cwd → context::resolve() → (maybe adopt) → dispatch verb → store → render`. Exactly one code path decides where notes live.

## Storage location

Single SQLite DB at `%USERPROFILE%\noteit.db`. All contexts share one file.

## Error handling

**Governing rule: a note tool must never lose a note.** Every failure degrades to a working capture except the two below.

**Hard failures** — refuse rather than degrade:

- **DB open/migrate failure.** Corrupt DB or failed migration stops the run. Do **not** create a second DB; do **not** skip the migration. A migration failure rolls back and reports the `user_version` it choked on.
- **Concurrent writes.** WAL mode with `busy_timeout`. Two shells capturing at once is normal and must not interleave into corruption.

## Testing

Effort follows risk: `repoid` and adoption can lose data; the rest is comparatively boring CRUD.

**`repoid`** — fixture repos built by real git operations in `tempfile` dirs. Each case maps to a specific scan claim, so a `gix`/git-C divergence surfaces as a test failure rather than missing notes:

- Single-root repo → stable id; **id identical after cloning to a different path** (without this test, nothing verifies the core premise).
- Multi-root (grafted) repo → smallest root, matching lensr's `strcmp` selection.
- Orphan branch → a **different** id, asserted deliberately, documenting known behavior so it is not "fixed" later.
- `git init`, zero commits → `Err(NoCommits)` — not a panic, not `NotARepo`.
- Shallow clone → `Err(Shallow)`.
- Plain directory → `Err(NotARepo)`.

**`context` + adoption** — integration tests on a temp DB: N-path fold with subpaths preserved; submodule-skip guard; idempotency (run twice, assert identical state); transaction rollback on mid-fold failure.

**`store`** — CRUD round-trip; FTS5 scoped results; tag queries; a migration test opening a `user_version = 0` DB and walking it forward (cheap now, expensive once user DBs exist in the wild).

**`cli`** — the ambiguity rule's three cases: `noteit search` dispatches, `noteit add "search"` captures, `noteit "search this"` captures.

**`render`** — snapshot tests on grouped and flat output.

**Not tested:** `$EDITOR` spawning (needs a real terminal — manual), `gix` itself.

## Out of scope for v1

No upward repo walk; no sync; no encryption; no attachments; no note deletion beyond `done` status (notes are append-mostly). Real deletion is a backlog item.
