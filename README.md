# noteit
<!-- rev:001 -->

`noteit` is a command-line tool for capturing small ideas and notes that bind
themselves to the git repository (or plain directory) you were in when you
wrote them. Run `noteit` inside a repo and you see that repo's notes; run it
in some other directory and you see that directory's notes; run
`noteit list --global` to see everything, everywhere.

The point that makes it different from "just append to a text file": a note
binds to the repo's **identity**, not its location on disk. Clone the repo to
another drive, rename the checkout directory, move it entirely — the notes
follow. This has been verified end to end: notes captured in a plain
directory before `git init` were automatically adopted into the repo once it
got its first commit, and then followed a clone of that repo into a
differently-named directory.

## Install

```powershell
cargo install --path .
```

This builds and installs the `noteit` binary from this checkout.

## Quick tour

```
=== capture before git init (path context) ===
saved 1 to demo

=== git init + commit, then run again (expect adoption) ===
adopted 1 notes from 1 paths into demo
saved 2 to demo

=== list ===
[ ] 2  idea after the repo existed #rust
[ ] 1  idea before the repo existed

=== search ===
[ ] 1  demo          idea before the repo existed
[ ] 2  demo          idea after the repo existed #rust

=== global ===
demo
  [ ] 2  idea after the repo existed #rust
  [ ] 1  idea before the repo existed

=== verb-collision escape hatch ===
saved 3 to demo
[ ] 3  search
[ ] 2  idea after the repo existed #rust
[ ] 1  idea before the repo existed
```

Notes go to stdout; warnings and adoption notices (like `adopted 1 notes from
1 paths into demo` above) go to stderr, so scripting one doesn't pollute the
other.

## Commands

```
noteit <text>              capture a note in the current context
noteit                     list notes for the current context
noteit add <text>          capture text that collides with a verb
noteit new                 capture a longer note in $EDITOR
noteit search <query>      full-text search      [--global]
noteit list                list notes            [--global] [--flat] [--tag <t>] [--all] [--limit <n>]
noteit done <id>           mark a note done
noteit open <id>           reopen a note
noteit project rename <n>  rename the current project
noteit --help | --version
```

Notes:
- `list --global` shows every context, grouped by project name by default;
  `--flat` shows one flat feed instead. `--tag <t>` filters by tag; `--all`
  includes done notes (by default only open notes are shown).
- `--limit <n>` caps output; the default cap is 50. `--limit 0` means
  unlimited. When output is truncated you always get an explicit trailer
  like `… 12 more (--limit 0 for all)` — never a silent cutoff.
- Ids shown by `list`/`search` are short, compact base36 strings (e.g. `3`,
  `a1`). `done`/`open` take those same ids.
- `#tag` written in a note body stays visible in the note's text (so it reads
  naturally) and is also indexed separately, which is what `--tag` queries
  against.

## The ambiguity rule (read this first)

There is no `capture` verb. `noteit` decides what to do based on its first
argument alone:

> **If the first argument matches a known verb (`add`, `list`, `search`,
> `new`, `done`, `open`, `project`), that verb runs. Anything else is treated
> as note text to capture.**

So:
- `noteit fix the login bug` captures the note "fix the login bug".
- `noteit search this` **searches** for "this" — it does not capture a note
  reading "search this".
- If you genuinely want to capture text that happens to start with a verb
  word, use the escape hatch: `noteit add "search this"` captures the literal
  text "search this" as a note.

This is a deliberate trade-off: it makes everyday capture as short as
possible (no `capture`/`note`/`add` prefix needed for the common case) at the
cost of a small, fixed set of reserved first words. There is intentionally no
`clap` dependency — a standard argument parser fights this rule, so parsing
is hand-rolled specifically to keep the ambiguity resolution simple and
predictable.

## How context binding works

Every note is filed under a **context**, which is one of:

- **A repository context**, keyed by the repo's id: the lexicographically
  smallest parentless (root) commit SHA reachable from `HEAD`, prefixed
  `urn:noteit:v1:`. This is the same value you'd get from
  `git rev-list --max-parents=0 HEAD | sort | head -1`. Because the id is
  derived from repo history rather than the working-directory path, it
  survives clones, renames, and moving the checkout to another drive.
  Shallow clones are rejected for id purposes (truncated history has no true
  root commit to anchor on), and `noteit` falls back to a path context with a
  warning in that case.
- **A path context**, keyed by an absolute directory path, used whenever the
  current directory isn't part of an identifiable repo (no repo at all, or a
  shallow clone).

Each note also records its `subpath` — where inside the context root you were
standing when you captured it — so notes from different subdirectories of
the same repo still show up together under one project, with their relative
location preserved.

The repo id is computed once per invocation, relative to `HEAD` at capture
time; it is never recomputed for existing notes. This means an orphan branch
in the same repo computes a different id than the main branch — a known,
accepted limitation, not a bug.

Display names default to the directory's basename and are purely cosmetic —
`noteit project rename <name>` changes how a project is displayed without
ever changing what a note is keyed to.

## Adoption

If you write notes in a plain directory before it's a recognizable git repo
(before `git init`, or before the first commit exists), those notes are
filed under a path context. The next time `noteit` runs in that same
directory *after* it has become an identifiable repo, its path context(s) are
automatically folded into the new repo context, and the fold is **announced**
on stderr:

```
adopted 1 notes from 1 paths into demo
```

Adoption only ever moves notes into the resolved repo context — it never
touches an unrelated repo. A submodule guard skips any candidate whose own
repo root differs from the one being resolved, so a nested repo's notes are
never accidentally swallowed into its parent. Every fold is also recorded in
an `adoptions` audit table, capturing what was folded in, which is what would
let a future `noteit adopt --undo` reverse it (not implemented yet).

## Storage

All notes live in a single SQLite database:

- `%USERPROFILE%\noteit.db` on Windows
- `$HOME/noteit.db` on Linux/macOS

The database uses WAL mode with a busy timeout, so two shells capturing notes
at the same time is safe. Schema migrations are tracked via
`PRAGMA user_version`. Full-text search is powered by an FTS5 external-content
table kept in sync via triggers.
