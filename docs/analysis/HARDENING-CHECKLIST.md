# Hardening Checklist — noteit
<!-- rev:003 -->

Baseline→Target by dimension: Testing & Coverage C→A (weight 5), CI/CD & Release
F→C+, Code Quality C→B, Security C→B, Correctness & Robustness B→A, Documentation
B→B+. Overall stage 2/5 → target 4/5.

Rust-only audit: Go-specific analysts/engine were not run (see runbook header).
Ordered so the apply loop can always pick the highest-leverage unblocked,
unchecked item next.

## Phase 1 — Stabilize
- [x] H-01 — Fix data-loss + symlink-race in `noteit new`'s $EDITOR temp-file handling (data-integrity/security, High, leverage 5) — blockedBy: [] — blocks: [H-02, H-13] — verify: `cargo test --lib cli:: -- --nocapture` — DONE d222b8c, verified (text-preserved test reads file off disk)
- [x] H-02 — Surface recovery path when editor temp file is not valid UTF-8 (error-handling, Medium, leverage 2) — blockedBy: [H-01] — blocks: [] — verify: `cargo test --lib cli::` — DONE 5ae98a1, verified
- [x] H-03 — Fix `list --global` sort order violating render_grouped's contiguity contract (robustness, Medium, leverage 2) — blockedBy: [] — blocks: [] — verify: `cargo test --lib render::` — DONE 947ccf0, verified (2-header test)

## Phase 2 — Harden
- [x] H-04 — Add CI workflow (build + test + clippy + fmt + coverage) (ci-dx, High, leverage 5) — blockedBy: [] — blocks: [H-05, H-06, H-10] — verify: `Test-Path .github\workflows\ci.yml` — DONE c4b209e
- [x] H-06 — Add clippy-deny-warnings gate; fix 5 outstanding warning classes (lint, High, leverage 5) — blockedBy: [H-04] — blocks: [H-07, H-08, H-09] — verify: `cargo clippy --all-targets -- -D warnings` — DONE 7074aff
- [x] H-05 — Run `cargo fmt` across the tree (formatting-only commit) (lint, Medium, leverage 2) — blockedBy: [H-04] — blocks: [] — verify: `cargo fmt --check` — DONE 42eb867
- [x] H-10 — Install and wire cargo-audit into CI (ci-dx/security, Medium, leverage 3) — blockedBy: [H-04] — blocks: [] — verify: `cargo audit` — DONE c4b209e (installed + run locally, 0 advisories across 179 crates; wired into ci.yml, no separate commit needed)
- [x] H-07 — Fix clippy::collapsible_if in src/cli.rs:248 (lint, Low, leverage 2) — blockedBy: [H-06] — blocks: [] — verify: `cargo clippy --all-targets` — DONE aefda69
- [x] H-08 — Fix clippy::unnecessary_map_or in src/repoid.rs:77 (lint, Low, leverage 2) — blockedBy: [H-06] — blocks: [] — verify: `cargo clippy --all-targets` — DONE aefda69
- [x] H-11 — Add rust-toolchain.toml pinning edition-2024 minimum toolchain (ci-dx, Low, leverage 2) — blockedBy: [] — blocks: [] — verify: `Test-Path rust-toolchain.toml` — DONE 7dee57c
- [x] H-09 — Resolve tests/common/mod.rs dead-code warnings, 3 helpers merged (lint, Low, leverage 1) — blockedBy: [H-06] — blocks: [] — verify: `cargo clippy --all-targets` — DONE aefda69

## Phase 3 — Mature
- [x] H-12 — Add integration coverage for cli::run() dispatch entrypoint (test-coverage, High, leverage 5) — blockedBy: [] — blocks: [H-13] — verify: `cargo llvm-cov --summary-only` — DONE 9352749 (extracted `run_core`, 16 new tests in tests/run.rs; llvm-cov measurement deferred, see runbook)
- [x] H-13 — Add coverage for edit_in_editor() ($EDITOR/$VISUAL path) (test-coverage, Medium, leverage 3) — blockedBy: [H-01, H-12] — blocks: [] — verify: `cargo llvm-cov --summary-only` — DONE 696056a (happy-path + spawn-failure tests added to tests/editor.rs)
- [x] H-14 — Add coverage for the 2 untested render.rs functions (test-coverage, Low, leverage 2) — blockedBy: [] — blocks: [] — verify: `cargo llvm-cov --summary-only` — DONE d55a4a2 (render_flat + render_grouped empty-case tests added)
- [x] H-16 — Rename misleading test new_opens_the_editor (test-quality, Low, leverage 1) — blockedBy: [H-13] — blocks: [] — verify: `grep -n 'fn new_opens_the_editor' tests/cli.rs` — DONE 02bf92c (renamed to new_parses_as_the_new_invocation)
- [x] H-15 — Add a thin assert_cmd test for main.rs (test-coverage, Low, leverage 1) — blockedBy: [] — blocks: [] — verify: `cargo llvm-cov --summary-only` — DONE 9a9bf27 (tests/main_smoke.rs via assert_cmd + predicates)
- [x] H-17 — Document run()'s exit-code contract (0/1/2) (docs, Low, leverage 1) — blockedBy: [] — blocks: [] — verify: `cargo doc --no-deps` — DONE (landed in 9352749 alongside the run_core extraction; `pub fn run` now carries a `/// # Exit codes` doc comment)
