# noteit — Maturity Rating

**Project type:** Rust (Cargo, edition 2024) CLI · **Assessed:** 2026-07-18
**Stage: 4 — Production (numeric) — but effectively a strong Release-Candidate**
**Weighted score: 91.4 / 100** · **Confidence: Medium**

> **Honesty caveat (read first).** The weighted score lands at 91.4, which maps
> to Stage 4 on the score ladder — but two *Production-defining* criteria are
> **unmet**: (1) there is **no measured test-coverage number** (`cargo llvm-cov`
> is infeasible locally — the instrumented rebuild of the bundled SQLite times
> out), and (2) **CI has never run green** (GitHub Actions has no hosted runner
> assigned — account-side billing). The high score reflects genuinely strong
> *code-level* signals (security, correctness, deps, docs); it does **not**
> reflect production battle-testing, which a ~2-week-old solo greenfield simply
> hasn't had. Treat real-world readiness as **high Release-Candidate** until the
> two gating criteria are satisfied. Confidence is Medium because the
> highest-weighted dimension (Testing) rests on an unverifiable coverage signal.

## Delta since last assessment (2026-07-17)

Prior: **Stage 2 — Beta, 64.8.** Every dimension improved; +26.6 points, +2 stages
(numeric). Drivers: CI added + clippy-`deny` gate + `cargo audit` (CI/CD **F→B**,
Code-Quality **C→A-**, Security **C→A**), the plugin system + integration/property
tests (Testing **C→B**, Correctness **B→A**), a full docs set + ADRs + man +
completions (Docs **B→A**), `v0.1.0` released + consolidated backlog (Stability
**C→B**), env config + degradation ladder (Ops **C→B+**), pinned deps + audit
(Deps **C→A**), and the `cli.rs` split (Arch **B→A-**).

## Dimension scorecard

| # | Dimension (wt) | Grade | Evidence (measured unless noted) |
|---|---|---|---|
| 1 | Architecture (3) | A- | Acyclic layers (`store/*` leaf; `cli→context/render/store`; `plugin` self-contained per ADR-0004); no god-file (largest src ~382 L); ADR-0001..0005. Gap: `cli/parse.rs` names `crate::plugin::{PluginCmd,HostSel}`. |
| 2 | Testing (5) | B | 160 tests — integration (`assert_cmd`) + property fuzz (`tests/property.rs`) + unit; fast/deterministic. **Coverage N/A** — `cargo llvm-cov` infeasible (instrumented bundled-SQLite rebuild >10 min; prior ~51 min); CI coverage job is `continue-on-error` + never sampled. No mutation testing. |
| 3 | CI/CD (4) | B | `ci.yml`: fmt + `clippy -D warnings` + test + `cargo audit` (blocking) + coverage (non-blocking). `v0.1.0` tagged + GitHub Release. **Never observed green** (no hosted runner — billing). No automated release workflow. |
| 4 | Security (4) | A | `cargo audit` = 0 advisories / 191 deps (blocking CI job). 0 `unsafe` in prod (6 total, all `#[cfg(test)]` `set_var`). Parameterized SQL (`params!` ×28) + `sanitize_fts_query`. `edit_in_editor` spawns `$EDITOR` as arg, no shell. |
| 5 | Documentation (2) | A | Full managed set: README (rev:006), CHANGELOG, LICENSE, AGENTS (rev:003), ARCHITECTURE (rev:001, mermaid), ADR-0001..0005, BACKLOG (rev:007), man page, completions. 59 `//!` in `plugin/`+`cli`. Gap: 9 core modules (`main`, `lib`, `context`, `repoid`, `render`, `store/*`) have no `//!` header. |
| 6 | Operational-Readiness (4) | B+ | thiserror enums + never-lose-a-note degradation ladder (`context.rs`); env config (`NOTEIT_DB`/`QUIET`/`PLUGIN_ROOT`); WAL + `busy_timeout(5s)`; stdout=data / stderr=notices; documented exit codes; deploy via `cargo install` + man + completions. Gap: printf `eprintln`, no verbosity levels (minor for a CLI). Server signals (health/metrics/shutdown) N/A. |
| 7 | Code-Quality (3) | A- | `clippy all=deny` clean; 0 suppressions; 1 prose TODO; no god-files. Gaps: `installed_file_count` (`command.rs:156`) ≡ `count_files_under` (`hosts.rs:207`) byte-identical dup; `pub trait Status` (`host.rs`) defined+exported, never impl'd (dead, superseded by `Doctor`). |
| 8 | Dependencies (3) | A | 4 pinned direct deps + `Cargo.lock` (191 resolved); 0 advisories. 3 `hashbrown` versions — all transitive via `gix` (accepted/documented; `Cargo.toml` mandates gix default features). Gap: no `cargo-deny` license gate. |
| 9 | Stability (3) | B | Append-only DB migrations (code-enforced `schema.rs:5`, regression-tested); consolidated BACKLOG; `v0.1.0` + CHANGELOG. Gaps: pre-1.0, no CLI deprecation policy; solo contributor; greenfield churn (89 commits/30 d — expected for age). |
| 10 | Correctness (4) | A | 0 prod `unwrap` (1 provably-infallible `expect` in `render.rs:21`); never-lose-a-note via `.keep()` on editor-failure paths; atomic tmp+rename installs; property tests; single-threaded (no race surface). Minor: best-effort `let _ = remove_dir_all`. |

**Rollup:** A=100, A-≈94, B+≈88, B=82, C=65. Weighted Σ = (94·3 + 82·5 + 82·4 +
100·4 + 100·2 + 88·4 + 94·3 + 100·3 + 82·3 + 100·4) / 35 = 3200 / 35 = **91.4**.

### Audit-process note

Three dimension auditors (Stability, Ops, Documentation) reported `CHANGELOG.md`,
`completions/*`, and `man/noteit.1` as *missing* — a **false negative**: their
`Glob` ran against the session's stale worktree (branch tip predating those
files) while their absolute-path `Read`s saw the current tree. Direct
verification (`git ls-files` @ `15aadca`) confirms **all exist on disk and in
git**. Those findings were discarded; the grades above reflect the corrected
picture (this lifted Ops from B to B+ and removed a false Stability weakness).

## Ranked weak points (by leverage, not severity)

1. **CI never run green** (CI/CD w4) — configured pipeline, zero passing runs.
2. **Coverage unmeasurable** (Testing w5) — the ceiling on the heaviest dimension.
3. No automated release workflow (CI/CD w4, Stability w3).
4. No `cargo-deny` license/advisory gate (Security w4, Deps w3).
5. `parse.rs` coupled to concrete plugin types (Arch w3).
6. Dup file-counter + dead `Status` trait (Code-Quality w3).
7. 9 core modules lack `//!` headers (Docs w2).
8. No verbosity levels / structured logging (Ops w4).

## Improvement route (Stabilize → Harden → Mature)

### Phase 1 — Stabilize (clear the two gating criteria)
- **Resolve GitHub Actions runner/billing → first green run.** Effort S. Validates test + audit gates and lets the coverage job sample. *First action:* enable the runner in repo Settings, re-run the workflow, capture the run URL.
- **Make coverage measurable — break the bundled-SQLite instrumentation timeout.** Effort M. Add a coverage build using system libsqlite3 (`rusqlite` without `bundled`) so `cargo llvm-cov` finishes; flip the CI coverage job to blocking once a baseline lands. *First action:* add a `coverage-syslib` feature, run `cargo llvm-cov --features coverage-syslib --summary-only`, record the first %.
- **Log the swallowed `remove_dir_all` cleanup error.** Effort S.

### Phase 2 — Harden
- **Add `cargo-deny`** (license + advisory + bans, codifying the accepted hashbrown dup). Effort S — closes Security + Deps in one move.
- **Automated release workflow** (tag-triggered, reuse CI gate). Effort M — after CI is green.
- **Decouple `parse.rs` from plugin internals** via a cli-layer command type. Effort M.
- **Verbosity levels / structured logging** honoring `NOTEIT_QUIET`. Effort M.

### Phase 3 — Mature
- **`//!` headers for the 9 core modules.** Effort S.
- **Dedup the file-counter + delete the dead `Status` trait.** Effort S.
- **CLI stability / deprecation ADR** (pre-1.0 stance). Effort S.

## Route execution (2026-07-18, same day)

Most code-actionable route items were executed immediately after this rating:
- **Done:** coverage-feasibility system-SQLite path (#2), `cargo-deny` gate (#3),
  dedup file-counter + remove dead `Status` trait (#4), `//!` headers for 9 core
  modules (#5), CLI-stability ADR-0006 (#7), decouple `parse.rs` from plugin
  internals (#8), tag-triggered release workflow (#9), consolidated notice
  helper (#10).
- **Corrected:** #6 (swallowed `remove_dir_all`) was a **false finding** — all
  such sites are `#[cfg(test)]` cleanup; the one prod uninstall uses a proper
  `match`. Not actionable.
- **Remaining (yours):** #1 — resolve GitHub Actions runner/billing so CI runs
  green. This is the one thing below, and it is an account-side action.

A re-rating after CI runs green + a coverage number is captured should clear the
two Production-defining criteria and lift confidence from Medium.

## The one thing

**Get GitHub Actions to run green on a hosted runner (resolve the account-side
billing/runner assignment).** It's the single highest-leverage move: the pipeline
is already fully authored, so one external unblock converts a
configured-but-unvalidated CI into a validated one and cascades into Testing
(reproducible green + coverage sampling), Security (live audit gate), and release
automation. It is an *account action, not a code change* — and it does **not** by
itself yield a coverage number, so pair it with the coverage-feasibility fix
(system-SQLite build). Together those two clear both unmet Production criteria.
