# Hardening Runbook — noteit (2026-07-17)
<!-- rev:001 -->

**Baseline stage:** 2/5 (Beta)   **Target:** 4/5 (Production)   **Language:** Rust (Cargo, edition 2024)

**Coverage note:** `noteit` is a Rust project. The three Go-only analysts
(harden-go-concurrency, harden-go-arch, harden-go-errors) and the reused
go-hardening engine were SKIPPED as not applicable. Coverage for this audit
comes from three Rust-focused analysts (`rust-robustness`, `correctness-security`,
`coverage-quality`) plus the evidence-based `docs/analysis/MATURITY.md` rating
(stage 2/5, weighted score ≈ 64.8/100). This is a smaller finding surface than a
Go audit would produce; treat gaps in concurrency/architecture dimensions as
"not assessed" rather than "clean."

**Dedup notes applied:**
- ROB-1 (data-loss on $EDITOR non-zero exit) and SEC-1 (predictable/O_EXCL-less
  temp path, symlink race) both target `edit_in_editor()` in `src/cli.rs`
  (~lines 190-208). Merged into **H-01**: the fix (switch to the `tempfile`
  crate's `NamedTempFile`, or `OpenOptions::create_new`, and stop deleting the
  file on non-zero exit) closes both the correctness/data-loss concern and the
  minor local symlink-race hardening gap in one change.
- CQ-8/CQ-9/CQ-10 (three `dead_code` warnings for unused `tests/common/mod.rs`
  helpers) are merged into **H-09**: CQ-10's own evidence notes one edit
  (`#[allow(dead_code)]` on the shared helpers file, or wiring/removing each
  helper) resolves all three.
- CQ-16 ("no duplicate-test issue remains") is informational only — confirms a
  prior fix held, no action required, not carried into the checklist.

**Highest-leverage single move:** stand up `.github/workflows/ci.yml`
(H-04 / CQ-12) running `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
and `cargo test`. It is the dominant node in the blocks/unblocks graph — it
turns every other fix in this runbook (fmt, clippy, coverage, audit) from a
point-in-time fix into a durably-enforced gate, and is a direct prerequisite
for H-06 (clippy-deny gate) and a natural home for H-10 (cargo-audit).

---

## Phase 1 — Stabilize (correctness, data-loss, security-critical)

### H-01 Fix data-loss + symlink-race in `noteit new`'s $EDITOR temp-file handling
- **Dimension:** data-integrity / security   **Severity:** High   **Leverage:** 3
- **Evidence:** `src/cli.rs:190-201` (`edit_in_editor`) — on non-zero editor exit,
  the temp file is deleted (`let _ = std::fs::remove_file(&path);`) before the
  error is returned, discarding any text the user already wrote/saved, even
  though the note-recall tool's cardinal invariant is "never lose a note."
  Separately (`src/cli.rs:190`, SEC-1), the temp path is predictable
  (`dir.join(format!("noteit-{}.md", std::process::id()))`) and created via
  `std::fs::write` with no `O_EXCL`/`create_new`, so on shared multi-user Unix
  temp dirs a pre-planted symlink at that path could redirect the write/read to
  an attacker-controlled file (not exploitable on Windows, the primary
  documented platform, since `%TEMP%` is per-user there).
- **Fix approach:** Switch to the `tempfile` crate's `NamedTempFile`
  (already a dev-dependency; promote to a normal dependency) for an
  unpredictable, O_EXCL-equivalent, securely-permissioned temp file. On
  non-zero editor exit, do NOT delete the file — read it first, and if
  non-empty, surface the temp path in the error message ("editor exited with
  `<status>`; your text was preserved at `<path>`") so the user can recover it.
  Only delete on the genuinely-empty/never-touched case.
- **Verify:** `cargo test --lib cli:: -- --nocapture` (add a test using a fake
  `$EDITOR` script that writes text then exits 1; assert the temp file still
  exists / its content is surfaced in the error) and
  `grep -n 'fn edit_in_editor' -A 15 src/cli.rs` to confirm `NamedTempFile` use.
- **Blocks:** [] **Unblocks:** [H-02, H-13]
- **Target-stage impact:** Correctness & Robustness dimension B→A; closes the
  audit's single named "never lose a note" violation.
- **Outcome:** _pending_

### H-02 Surface recovery path when editor temp file is not valid UTF-8
- **Dimension:** error-handling   **Severity:** Medium   **Leverage:** 2
- **Evidence:** `src/cli.rs:205` (ROB-2) — `std::fs::read_to_string(&path)?`
  fails on non-UTF-8 content (e.g. editor default encoding, pasted bytes) and
  returns `Err` without deleting the temp file *or* telling the user where it
  is; bytes survive on disk but are effectively unrecoverable in practice.
- **Fix approach:** On the `read_to_string` error path, include the temp file
  path in the returned error message ("could not read note from `<path>`
  (invalid UTF-8): `<err>`; the file was left in place"). Optionally offer a
  lossy-read-with-warning fallback.
- **Verify:** `cargo test --lib cli::` (add a test with a fake `$EDITOR`
  writing invalid UTF-8 bytes; assert the error message contains the temp path).
- **Blocks:** [] **Unblocks:** []  (pairs with H-01; ordered right after it)
- **Target-stage impact:** Improves recoverability guarantee for the $EDITOR
  capture path; supports the same B→A move as H-01.
- **Outcome:** _pending_

### H-03 Fix `list --global` sort order violating `render_grouped`'s contiguity contract
- **Dimension:** robustness   **Severity:** Medium   **Leverage:** 2
- **Evidence:** `src/cli.rs:297` (ROB-3) — rows are sorted by
  `(display_name, created_at)` before calling `render_grouped`, but
  `render_grouped`'s own doc comment requires rows pre-sorted by context id
  (it flushes/reprints a header whenever `ctx.id` changes). Two contexts
  sharing a `display_name` (e.g. two repos both named "app") get their notes
  interleaved and printed as alternating duplicate-looking header blocks.
- **Fix approach:** Sort by `(ctx.id, created_at desc)` as the primary key so
  `render_grouped`'s contiguity invariant holds; use `display_name` only as a
  secondary/display-ordering key, or pre-group by id/name in a stable pass
  before interleaving notes within each group by time.
- **Verify:** `cargo test --lib render::` (add a test: two `Context` rows with
  identical `display_name` but different ids, interleaved note timestamps;
  assert `render_grouped` emits exactly two header lines, not more).
- **Blocks:** [] **Unblocks:** []
- **Target-stage impact:** Fixes a real render/grouping defect that could
  mislead users into thinking notes are missing or duplicated.
- **Outcome:** _pending_

---

## Phase 2 — Harden (robustness, reuse, deps, CI foundation)

### H-04 Add CI workflow (build + test + clippy + fmt + coverage)
- **Dimension:** ci-dx   **Severity:** High   **Leverage:** 5
- **Evidence:** CQ-12 — `Test-Path .github\workflows` returned `False`; no CI
  exists at all, so tests/clippy/fmt only run when a human remembers to.
- **Fix approach:** Add `.github/workflows/ci.yml` running, at minimum on
  push/PR: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --all-targets`, and optionally `cargo llvm-cov` for a coverage
  gate/badge.
- **Verify:** `gh workflow list` (or) `Test-Path D:\rust\noteit\.github\workflows\ci.yml`
- **Blocks:** [] **Unblocks:** [H-05, H-06, H-10]
- **Target-stage impact:** CI/CD & Release dimension F→C+ immediately; the
  single highest-leverage move in this runbook.
- **Outcome:** _pending_

### H-05 Run `cargo fmt` across the tree (formatting-only commit)
- **Dimension:** lint   **Severity:** Medium   **Leverage:** 2
- **Evidence:** CQ-11 — `cargo fmt --check` exits 1; unformatted code in
  `src/cli.rs`, `tests/context.rs`, `tests/repoid.rs`, `tests/store.rs`
  (import ordering, long lines, multi-arg calls never fmt'd).
- **Fix approach:** Run `cargo fmt` once across the whole tree as a
  standalone formatting-only commit (no logic changes mixed in), then rely on
  H-04's CI gate to keep it enforced.
- **Verify:** `cargo fmt --check`
- **Blocks:** [] **Unblocks:** []
- **Target-stage impact:** Removes onboarding friction / noisy diffs; trivial
  one-shot fix.
- **Outcome:** _pending_

### H-06 Add clippy-deny-warnings gate and fix the 5 outstanding warning classes
- **Dimension:** lint   **Severity:** High   **Leverage:** 5
- **Evidence:** CQ-5 — no `#![deny(warnings)]`, no `clippy.toml`, no CI running
  clippy; `cargo clippy --all-targets` currently emits 5 distinct warning
  classes uncontested (see H-07/H-08/H-09).
- **Fix approach:** Wire `cargo clippy --all-targets -- -D warnings` into
  H-04's CI workflow; fix the existing 5 warning classes first (H-07, H-08,
  H-09) so the new gate starts green.
- **Verify:** `cargo clippy --all-targets -- -D warnings`
- **Blocks:** [] **Unblocks:** [H-07, H-08, H-09]
- **Target-stage impact:** Without this gate, every clippy finding fixed once
  can silently regress; highest-leverage lint-dimension change alongside H-04.
- **Outcome:** _pending_

### H-07 Fix `clippy::collapsible_if` in `src/cli.rs:248`
- **Dimension:** lint   **Severity:** Low   **Leverage:** 2
- **Evidence:** CQ-6 — nested `if !matches!(...) { if let Some(r) = ... }`
  should collapse via a let-chain per clippy's own suggested fix.
- **Fix approach:** Apply clippy's suggested let-chain collapse
  (`if !matches!(inv, Invocation::Adopt { undo: true }) && let Some(r) = adopt_if_needed(&mut store, &resolved)? { ... }`),
  or run `cargo clippy --fix --lib -p noteit`.
- **Verify:** `cargo clippy --all-targets`
- **Blocks:** [] **Unblocks:** []
- **Target-stage impact:** Cosmetic; no behavior change.
- **Outcome:** _pending_

### H-08 Fix `clippy::unnecessary_map_or` in `src/repoid.rs:77`
- **Dimension:** lint   **Severity:** Low   **Leverage:** 2
- **Evidence:** CQ-7 — `best.as_ref().map_or(true, |b| hex < *b)` should be
  `.is_none_or(|b| hex < *b)`.
- **Fix approach:** Replace `.map_or(true, |b| hex < *b)` with
  `.is_none_or(|b| hex < *b)` at `repoid.rs:77`.
- **Verify:** `cargo clippy --all-targets`
- **Blocks:** [] **Unblocks:** []
- **Target-stage impact:** Cosmetic; touches well-tested lexicographic
  multi-root-repo logic, no coverage risk.
- **Outcome:** _pending_

### H-09 Resolve `tests/common/mod.rs` dead-code warnings (3 helpers)
- **Dimension:** lint   **Severity:** Low   **Leverage:** 1
- **Evidence:** CQ-8/CQ-9/CQ-10 — `git_root_sha()`, `path_of()`, and
  `plain_dir()` each trigger per-binary `dead_code` warnings (each integration
  test binary compiles `tests/common/mod.rs` independently, so clippy warns
  per-binary when that binary's own tests don't call a given helper). Note:
  `git_root_sha()`'s doc comment ("the parity oracle") implies it should be
  wired into a real test, not deleted; `plain_dir()` genuinely is used
  elsewhere (`tests/context.rs:74`) — the warning is a false-positive of the
  shared-helpers-file pattern.
- **Fix approach:** Add `#[allow(dead_code)]` at the top of
  `tests/common/mod.rs` to accept the shared-helper-file pattern in one edit
  (resolves all three warnings), OR selectively wire `git_root_sha()` into the
  repoid parity-oracle test it was written for and remove `path_of()` if truly
  redundant with inlined `td.path().to_path_buf()` calls.
- **Verify:** `cargo clippy --all-targets`
- **Blocks:** [] **Unblocks:** []
- **Target-stage impact:** Low; one edit clears 3 warnings project-wide.
- **Outcome:** _pending_

### H-10 Install and wire `cargo-audit` into CI
- **Dimension:** ci-dx / security   **Severity:** Medium   **Leverage:** 3
- **Evidence:** CQ-14 — `cargo audit --version` fails (`error: no such command:
  audit`); dependency vulnerabilities are never checked; `cargo tree` shows a
  542-line transitive tree (driven by `gix`) with 3 duplicate `hashbrown`
  versions.
- **Fix approach:** `cargo install cargo-audit`; add it as a step in H-04's CI
  workflow (optionally a scheduled weekly run to catch newly-disclosed CVEs
  against already-pinned deps without a code change triggering CI).
- **Verify:** `cargo audit`
- **Blocks:** [] **Unblocks:** []
- **Target-stage impact:** Security dimension C→B; low urgency for a small CLI
  but easy to close alongside H-04.
- **Outcome:** _pending_

### H-11 Add `rust-toolchain.toml` pinning the edition-2024 minimum toolchain
- **Dimension:** ci-dx   **Severity:** Low   **Leverage:** 2
- **Evidence:** CQ-13 — no `rust-toolchain.toml`; project uses
  `edition = "2024"` (requires rustc 1.85+); local toolchain is 1.96.0 but
  nothing pins this for new contributors/CI.
- **Fix approach:** Add `rust-toolchain.toml` pinning a minimum stable channel
  that supports edition 2024 and the let-chains feature used by H-07's fix.
- **Verify:** `Test-Path D:\rust\noteit\rust-toolchain.toml`
- **Blocks:** [] **Unblocks:** []
- **Target-stage impact:** Minor DX friction removal.
- **Outcome:** _pending_

---

## Phase 3 — Mature (coverage, docs, polish)

### H-12 Add integration coverage for `cli::run()` (the actual dispatch entrypoint)
- **Dimension:** test-coverage   **Severity:** High   **Leverage:** 5
- **Evidence:** CQ-1 — `run()` has zero test coverage; `cli.rs` measures
  25.99% region / 29.60% line coverage overall, the lowest of any module.
  Every `#[test]` in `tests/cli.rs` calls `parse()` only, never `run()`; no
  test captures stdout/stderr or exit codes.
- **Fix approach:** Add an integration-style harness calling
  `noteit::cli::run(&args)` against a `Store::open_in_memory()`-backed temp
  env (inject db path via env var, or refactor `run()` to accept a `Store`),
  capturing stdout/stderr and exit code. Cover: Capture success + empty-body
  Ok(2), New empty-body Ok(2), List (local/global/flat/tag/all/limit),
  Search local vs global, SetStatus done/open success + not-found (Ok(1)) +
  invalid-id (Ok(2)), Rename, Adopt{undo:true} both branches, and the
  unreachable Adopt{undo:false} eprintln path.
- **Verify:** `cargo llvm-cov --summary-only` (confirm `cli.rs` line coverage
  rises well above 30%)
- **Blocks:** [] **Unblocks:** [H-13]
- **Target-stage impact:** Testing & Coverage dimension (weight 5, highest in
  scorecard) C→B; closes the single largest coverage hole in the project.
- **Outcome:** _pending_

### H-13 Add coverage for `edit_in_editor()` ($EDITOR/$VISUAL path)
- **Dimension:** test-coverage   **Severity:** Medium   **Leverage:** 3
- **Evidence:** CQ-2 — `edit_in_editor()` is completely untested; none of its
  three error paths (`Command::status()` Err, non-zero exit, temp-file
  write/read) are exercised. Depends on H-01's fix landing first so the test
  targets the corrected behavior.
- **Fix approach:** Set `$EDITOR` to a small deterministic script/binary that
  writes fixed content to `argv[1]` and exits 0 (happy path), and a second
  that exits non-zero (error path + H-01's preserved-content behavior). On
  Windows use a `.cmd` or compiled Rust helper since bash scripts won't run
  directly.
- **Verify:** `cargo llvm-cov --summary-only` (confirm `edit_in_editor` shows
  executed lines)
- **Blocks:** [] **Unblocks:** []
- **Target-stage impact:** Removes an untested IO/subprocess integration
  point; couples naturally with H-01/H-02's fixes.
- **Outcome:** _pending_

### H-14 Add coverage for the 2 untested `render.rs` functions
- **Dimension:** test-coverage   **Severity:** Low   **Leverage:** 2
- **Evidence:** CQ-4 — `render.rs` at 51.32% region / 53.54% line coverage,
  2 of 7 functions never executed (likely grouped/flat truncation-message
  formatters, reached only via `--global` list/search paths under H-12).
- **Fix approach:** Read `render.rs` to identify the 2 uncalled functions;
  add direct unit tests once confirmed (some coverage will also land as a
  side effect of H-12).
- **Verify:** `cargo llvm-cov --summary-only` (module breakdown for `render.rs`)
- **Blocks:** [] **Unblocks:** []
- **Target-stage impact:** Moderate; exercises real branching logic
  (grouped vs flat, truncation counts).
- **Outcome:** _pending_

### H-15 Add a thin `assert_cmd` test for `main.rs`
- **Dimension:** test-coverage   **Severity:** Low   **Leverage:** 1
- **Evidence:** CQ-3 — `main.rs` has 0% coverage (7/7 lines missed); no
  end-to-end process-level test drives the compiled binary.
- **Fix approach:** Optional — add an `assert_cmd`-based test that spawns the
  compiled `noteit` binary once (e.g. `noteit --version`) to cover `main()`'s
  `process::exit(code)` wiring. Low priority since `run()` is the meaningful
  unit once H-12 lands.
- **Verify:** `cargo llvm-cov --summary-only`
- **Blocks:** [] **Unblocks:** []
- **Target-stage impact:** Cosmetic coverage-number improvement only.
- **Outcome:** _pending_

### H-16 Rename misleading test `new_opens_the_editor`
- **Dimension:** test-quality   **Severity:** Low   **Leverage:** 1
- **Evidence:** CQ-15 — `tests/cli.rs:126` `new_opens_the_editor` only checks
  `parse()` returns `Invocation::New`; it never opens an editor.
- **Fix approach:** Rename to `new_parses_as_the_new_invocation` (or similar);
  add the real editor-invocation test from H-13 under the
  `new_opens_the_editor` name instead.
- **Verify:** `grep -n 'fn new_opens_the_editor' tests/cli.rs`
- **Blocks:** [] **Unblocks:** []
- **Target-stage impact:** Low; prevents a future maintainer from believing
  editor-invocation is covered when it isn't (until H-13 lands).
- **Outcome:** _pending_

### H-17 Document `run()`'s exit-code contract (0/1/2)
- **Dimension:** docs   **Severity:** Low   **Leverage:** 1
- **Evidence:** CQ-17 — `src/cli.rs:210` `pub fn run(...)` has no doc comment;
  the exit-code convention (2 = usage/empty-body error, 1 = not-found,
  0 = success) is explained only via scattered inline comments near each
  return site.
- **Fix approach:** Add a `///` doc comment on `run()` enumerating the
  exit-code contract as one authoritative reference.
- **Verify:** `cargo doc --no-deps` (manually inspect generated docs for `cli::run`)
- **Blocks:** [] **Unblocks:** []
- **Target-stage impact:** Minor DX improvement; main remaining doc gap of note.
- **Outcome:** _pending_

---

## Informational (no action required)

- **CQ-16** — confirmed the previously-fixed duplicate-test issue has not
  regressed; the "duplicate" clippy language refers to repeated `dead_code`
  lint instances across compilation units (see H-09), not duplicate test
  bodies. No checklist item.
