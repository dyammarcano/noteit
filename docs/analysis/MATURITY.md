# noteit — Maturity Rating

Project type: Rust (Cargo, edition 2024) CLI. Assessed 2026-07-17.

> **Update — 2026-07-17 (post-hardening + plugin).** Most of the original
> Phase 1/2 route below has since landed; the scorecard rows are annotated
> inline with **[UPDATE]** where reality has moved. Summary of what changed:
> - **CI/CD (was F → now ~C):** `.github/workflows/ci.yml` exists and runs
>   fmt + `clippy --all-targets -D warnings` + `cargo test` + `cargo audit` on
>   push/PR. (Hosted-runner billing is an account-side gap, not a repo gap.)
> - **Code Quality (was C → now ~B):** the 2 clippy findings are fixed and a
>   `[lints.clippy] all = deny` gate is active in `Cargo.toml`; `cargo clippy
>   --all-targets -- -D warnings` is clean.
> - **Security (C):** `cargo audit` is now available (0.22.2) and wired into CI;
>   a local run scans 191 deps with no advisories.
> - **Stability (was C → now ~C+):** a git remote is configured and `master` is
>   pushed to GitHub; `LICENSE` (BSD-3) is present.
> - **Scope grew:** a plugin system was added (`src/plugin/*`, `noteit plugin
>   install|list|status|uninstall`), lifting the suite from 71 → **139 tests**.
>   `cli.rs` grew 372 → 550 lines and remains the least-covered module — the one
>   still-open Testing gap. Coverage numbers below are the last one-off
>   measurement and are pending a re-run.
>
> Re-run a full `/project:rating` audit to recompute the weighted score; it is
> expected to have moved up from 64.8 (Beta) toward the Beta/Production boundary.

**Overall stage: 2 — Beta** (weighted score ≈ 64.8 / 100, band 50–67)
**Confidence: Medium** — most signals were directly measured (test run, clippy, `cargo llvm-cov`, `cargo tree --duplicates`, greps, git log); a few (ops readiness, stability trend) are qualitative/estimated given the project's small size and single-contributor local history (no remote, no CI runs to sample).

Exit bar for the audit is stage 4 (Production) — this project sits two stages below that, which is expected for a ~1500-LOC greenfield tool with no CI/release infrastructure yet.

## Dimension scorecard

| # | Dimension (weight) | Stage/Grade | Evidence | Gap to next stage |
|---|---|---|---|---|
| 1 | Architecture & Boundaries (3) | B | Clean module split: `src/cli.rs`(372) `context.rs`(153) `render.rs`(115) `repoid.rs`(82) `store/{mod,contexts,notes,schema}.rs`; largest file 372 lines, no god-files; design captured in `docs/superpowers/specs/2026-07-17-noteit-design.md` and `docs/superpowers/plans/2026-07-17-noteit.md` | Add a lightweight `docs/adr/` or `ARCHITECTURE.md` with a diagram; formalize module boundaries as public interfaces |
| 2 | Testing & Coverage (5) | C | `cargo llvm-cov --summary-only`: **TOTAL 71.98% line cover, 63.50% region cover, 72.22% function cover** (measured). Per-file: `cli.rs` only **25.99% region / 29.60% line** cover, `main.rs` 0%, vs `store/contexts.rs` 86.22%/95.65%. 71 tests total, all passing (`cargo test`) | Cover `cli.rs` dispatch/arg paths (currently the weakest module) and wire coverage into CI so it's tracked, not one-off |
| 3 | CI/CD & Release (4) | F | `Test-Path .github\workflows` → `False`; no tags/releases; `git remote -v` → empty (no remote at all); `Cargo.toml` version `0.1.0` (dev snapshot) | Add a GitHub Actions workflow running build+test+clippy+coverage on every push; that alone moves this dimension from F to at least C |
| 4 | Security (4) | C | `cargo-audit` not installed → `error: no such command: audit` (no dep-vuln scanning available); `unsafe` count in `src/` = **0**; only 1 `.expect(` (`src/render.rs:21`, `.expect("ascii")`) and 0 `.unwrap()` in `src/` | Install/wire `cargo audit` (or `cargo deny`) into CI; audit the one `expect(` for a real failure mode |
| 5 | Documentation (2) | B | `README.md` present with rev-tag (`rev:003`), install + quick-tour walkthrough; `docs/superpowers/specs/` + `plans/` capture design decisions; no `ARCHITECTURE.md`/`CHANGELOG.md`/formal ADR log | Add `docs/ARCHITECTURE.md` and start a `CHANGELOG.md` once releases begin |
| 6 | Operational Readiness (4) | C | CLI tool (no server, so health/metrics don't directly apply); errors surfaced via `thiserror`-based error types (`Cargo.toml:19`); user-facing status via `eprintln!` (`src/cli.rs:251`), not structured logging; single SQLite file store, no config layering beyond repo-bound path resolution | Add basic `--verbose`/log-level control and document the on-disk DB location/config surface |
| 7 | Code Quality & Tech Debt (3) | C | `cargo clippy --all-targets` → 2 real warnings: collapsible `if` (`src/cli.rs:248`), `map_or` → `is_none_or` simplification (`src/repoid.rs:77`); plus 4 dead-code warnings for unused test helpers in `tests/common/mod.rs` (`plain_dir`, `git_root_sha`, `path_of` ×2); no `TODO`/`FIXME`/`HACK` found in `src/`; no `clippy.toml`/`rustfmt.toml` present | Fix the 2 clippy findings and prune/gate unused test helpers; add `clippy.toml` + CI `-D warnings` gate |
| 8 | Dependency & Supply-chain Health (3) | C | Only 3 direct deps (`gix 0.85`, `rusqlite 0.40` bundled, `thiserror 2.0`) + `tempfile` dev-dep, but `cargo tree` shows **542 lines** (large transitive tree, driven almost entirely by `gix`'s internal crate family); `cargo tree --duplicates` shows **3 duplicate `hashbrown` versions** (0.14.5, 0.16.1, 0.17.1) pulled in via `dashmap`/`clru`/`rusqlite`/gix internals; no `cargo audit` available to check CVE status | Run `cargo audit`/`cargo deny` once installed; evaluate whether `gix`'s default-feature transitive bloat (explicitly called out as required in `Cargo.toml:14`) is worth revisiting |
| 9 | Stability & Change Management (3) | C | `git log --oneline` → **36 commits**, no remote configured (`git remote -v` empty) — single-machine, pre-collaboration history; version pinned at `0.1.0`; no deprecation policy needed yet at this stage but also none documented | Push to a remote, tag a first release, and start a `docs/BACKLOG.md`-driven change log (a `BACKLOG.md` already exists at `docs/BACKLOG.md`) |
| 10 | Correctness & Robustness (4) | B | `src/`: **0** `.unwrap()`, **1** `.expect(` (`render.rs:21`), **0** `panic!`, **0** `unsafe` — clean error propagation via `thiserror`; all 71 tests pass including idempotent-migration and FTS-edge-case tests (`store/notes.rs`: `migrations_are_idempotent`, `search_with_unbalanced_quote_does_not_error`); single-threaded CLI process against one SQLite connection, so no data-race surface to probe with `-race`-equivalent tooling | Add a fuzz/property test pass on the FTS query sanitizer (`sanitize_fts_query`) and CLI arg parser for defensive-input hardening |

Weights sum to 35. Weighted score = Σ(grade_points × weight) / 35 ≈ **64.8** → **Stage 2 (Beta)**.

## Weakest dimensions (ranked by stage, not yet by leverage)

1. **CI/CD & Release — F** (no workflow, no releases, no remote)
2. **Testing & Coverage — C**, weight 5 (highest-weighted dimension; `cli.rs` at 29.6% line coverage is the concrete gap)
3. **Security / Dependency Health — C/C** (no `cargo audit` installed anywhere to even check CVE exposure; 3 duplicate `hashbrown` versions in the tree)

## Improvement route (Stabilize → Harden → Mature) to reach stage 4

**The one highest-leverage move: stand up a CI workflow (`.github/workflows/ci.yml`) running `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo llvm-cov`.**
This single change is the dominant node in the blocks/unblocks graph: it (a) turns the CI/CD dimension from F to at least C immediately, (b) makes the already-measured 72% coverage number continuously enforced instead of one-off, (c) gates the 2 outstanding clippy findings from regressing, and (d) is the prerequisite plumbing for adding `cargo audit`/`cargo deny` and for cutting a first tagged release. Nothing else on this list can be verified as durable without it.

### Phase 1 — Stabilize (this week)
- Add `.github/workflows/ci.yml`: build, `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo llvm-cov --summary-only`. *Unblocks: CI dimension, coverage tracking, quality gate.*
- Fix the 2 clippy findings (`src/cli.rs:248` collapsible-if, `src/repoid.rs:77` map_or→is_none_or) and either use or `#[cfg(test)]`-gate/remove the 4 dead test helpers in `tests/common/mod.rs`. *Unblocks: Code Quality dimension; effort S.*
- Add a `LICENSE` file (BSD-3-Clause per project convention) — currently absent, which is a real gap for any future distribution. *Effort S.*

### Phase 2 — Harden (next)
- Install and wire `cargo audit` (or `cargo deny`) into the new CI workflow; resolve any findings. *Unblocks: Security + Dependency Health dimensions; depends on Phase 1's CI existing.*
- Add integration/CLI-argument tests targeting `src/cli.rs` (currently 29.6% line coverage, the weakest module) to lift it toward the store-layer's ~80–95% bar. *Unblocks: Testing dimension's highest-weight gap.*
- Add a fuzz/property test for `sanitize_fts_query` and CLI parsing to harden Correctness against malformed input. *Effort M.*

### Phase 3 — Mature (once green)
- Push to a remote, tag `v0.1.0`, start `CHANGELOG.md`, and add `docs/ARCHITECTURE.md` (a diagram of `cli → context/store → sqlite+FTS5`). *Unblocks: Release, Documentation, Stability dimensions.*
- Revisit the `gix`-driven transitive dependency bloat (542-line tree, 3 duplicate `hashbrown` versions) now that CI/audit tooling exists to evaluate any change safely.

Re-run this rating after Phase 1 lands — CI alone should move CI/CD from F to C+ and lock in the existing 72% coverage number as a durable, continuously-checked signal rather than a point-in-time one.
