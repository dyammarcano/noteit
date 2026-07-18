# Changelog

All notable changes to `noteit` are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

_Nothing yet._

## [0.1.0] - 2026-07-18

First tagged release.

### Added

- **Note capture bound to git repo identity.** Notes bind to a repository's
  first-commit SHA (`urn:noteit:v1:<sha>`), so they follow the repo across
  clones and renames. Outside a repo, notes bind to the directory path.
- **Core CLI:** `noteit <text>` / `add`, bare `noteit` list, `new` ($EDITOR),
  `search` (FTS5), `list` (`--global`/`--flat`/`--tag`/`--all`/`--limit`),
  `done`/`open`, `project rename`, `delete <id>` (hard delete), `adopt --undo`.
- **`noteit export`** — dump every note (with its context) as JSON for backup.
- **Path-context adoption.** When a repo appears over a previously path-bound
  directory, its notes are automatically folded into the repo-context (announced
  on stderr); `adopt --undo` reverses the most recent fold and pins it.
- **Plugin system.** `noteit plugin install|list|status|doctor|uninstall --host
  <claude|codex|gemini|all>` installs noteit's bundled assets (a skill, the
  `/note` `/notes` `/note-search` commands, and a `note-keeper` agent) into a
  host-specific tree under `$HOME/.<host>/plugins/noteit/`. `doctor` runs
  host-side health checks. Built on a std-only plugin-host contract ported from
  an internal Go package (`src/plugin/`).
- **Configuration via env:** `NOTEIT_DB` (database path), `NOTEIT_QUIET`
  (suppress informational stderr notices), `NOTEIT_PLUGIN_ROOT` (plugin install
  root).
- **Shell completions** for bash, zsh, fish, and PowerShell (`completions/`);
  a **man page** (`man/noteit.1`).
- **FTS5 query sanitization** so malformed queries (unbalanced quotes, bare
  `AND`/`OR`) become literal terms instead of SQL errors.
- **CI** (`.github/workflows/ci.yml`): fmt, `clippy --all-targets -D warnings`,
  tests, `cargo audit`, and an informational coverage job.

### Performance

- `list`/`search`/`--tag` collapse their count+fetch double query into a single
  scan via `COUNT(*) OVER()`.

### Documentation

- `docs/ARCHITECTURE.md` (module + flow diagrams) and `docs/adr/` (ADR-0001…0005).

### Notes

- Storage is a single SQLite DB (WAL) at `%USERPROFILE%`/`$HOME`; schema
  migrations are append-only (v1, v2). Notes → stdout, diagnostics → stderr.

[Unreleased]: https://github.com/dyammarcano/noteit/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/dyammarcano/noteit/releases/tag/v0.1.0
