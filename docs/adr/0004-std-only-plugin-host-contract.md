# ADR-0004: Std-only ported plugin-host contract

- **Status:** Accepted
- **Date:** 2026-07-17

## Context

noteit ships as an installable plugin for AI hosts (Claude, Codex, Gemini). The
plugin-host contract (assets, host trait, atomic tree write, library-skill
synthesis) was ported from an internal Go package. noteit is a public repo with
a deliberately small dependency set, and the source is private.

## Decision

Port the core contract into `src/plugin/` **std-only, zero new crates**:
- a hand-rolled `<%.Field%>` template substituter (no `text/template` analog);
- explicit `AssetRegistry` / `HostRegistry` value types replacing Go's
  `init()`-populated globals;
- Go `interface`s → Rust `trait`s (`Host`, `Installer`, `Status`, `Doctor`,
  `TreeWriter`); `error` → `io::Result`.

All source-project-specific identifiers are **de-identified**; a guard test
(`de_identified_no_private_literals`) enforces none leak into rendered output.

## Consequences

- No new dependencies, no private identifiers in the public repo.
- The renderer is faithful but simpler than Go's engine (infallible except for an
  unterminated delimiter) — documented in `docs/port/PORT-PLAN.md`.
- The `Doctor`/`Status` traits are part of the contract; `Doctor` is now used by
  `noteit plugin doctor`, `Status` remains available for future use.
- Provenance is described generically; the private source project is never
  named anywhere in this public repository.
