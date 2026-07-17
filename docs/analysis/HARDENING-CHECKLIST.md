# Hardening Checklist — noteit
<!-- rev:001 -->

Baseline→Target by dimension: Testing & Coverage C→A (weight 5), CI/CD & Release
F→C+, Code Quality C→B, Security C→B, Correctness & Robustness B→A, Documentation
B→B+. Overall stage 2/5 → target 4/5.

Rust-only audit: Go-specific analysts/engine were not run (see runbook header).
Ordered so the apply loop can always pick the highest-leverage unblocked,
unchecked item next.

## Phase 1 — Stabilize
- [ ] H-01 — Fix data-loss + symlink-race in `noteit new`'s $EDITOR temp-file handling (data-integrity/security, High, leverage 5) — blockedBy: [] — blocks: [H-02, H-13] — verify: `cargo test --lib cli:: -- --nocapture`
- [ ] H-02 — Surface recovery path when editor temp file is not valid UTF-8 (error-handling, Medium, leverage 2) — blockedBy: [H-01] — blocks: [] — verify: `cargo test --lib cli::`
- [ ] H-03 — Fix `list --global` sort order violating render_grouped's contiguity contract (robustness, Medium, leverage 2) — blockedBy: [] — blocks: [] — verify: `cargo test --lib render::`

## Phase 2 — Harden
- [ ] H-04 — Add CI workflow (build + test + clippy + fmt + coverage) (ci-dx, High, leverage 5) — blockedBy: [] — blocks: [H-05, H-06, H-10] — verify: `Test-Path D:\rust\noteit\.github\workflows\ci.yml`
- [ ] H-06 — Add clippy-deny-warnings gate; fix 5 outstanding warning classes (lint, High, leverage 5) — blockedBy: [H-04] — blocks: [H-07, H-08, H-09] — verify: `cargo clippy --all-targets -- -D warnings`
- [ ] H-05 — Run `cargo fmt` across the tree (formatting-only commit) (lint, Medium, leverage 2) — blockedBy: [H-04] — blocks: [] — verify: `cargo fmt --check`
- [ ] H-10 — Install and wire cargo-audit into CI (ci-dx/security, Medium, leverage 3) — blockedBy: [H-04] — blocks: [] — verify: `cargo audit`
- [ ] H-07 — Fix clippy::collapsible_if in src/cli.rs:248 (lint, Low, leverage 2) — blockedBy: [H-06] — blocks: [] — verify: `cargo clippy --all-targets`
- [ ] H-08 — Fix clippy::unnecessary_map_or in src/repoid.rs:77 (lint, Low, leverage 2) — blockedBy: [H-06] — blocks: [] — verify: `cargo clippy --all-targets`
- [ ] H-11 — Add rust-toolchain.toml pinning edition-2024 minimum toolchain (ci-dx, Low, leverage 2) — blockedBy: [] — blocks: [] — verify: `Test-Path D:\rust\noteit\rust-toolchain.toml`
- [ ] H-09 — Resolve tests/common/mod.rs dead-code warnings, 3 helpers merged (lint, Low, leverage 1) — blockedBy: [H-06] — blocks: [] — verify: `cargo clippy --all-targets`

## Phase 3 — Mature
- [ ] H-12 — Add integration coverage for cli::run() dispatch entrypoint (test-coverage, High, leverage 5) — blockedBy: [] — blocks: [H-13] — verify: `cargo llvm-cov --summary-only`
- [ ] H-13 — Add coverage for edit_in_editor() ($EDITOR/$VISUAL path) (test-coverage, Medium, leverage 3) — blockedBy: [H-01, H-12] — blocks: [] — verify: `cargo llvm-cov --summary-only`
- [ ] H-14 — Add coverage for the 2 untested render.rs functions (test-coverage, Low, leverage 2) — blockedBy: [] — blocks: [] — verify: `cargo llvm-cov --summary-only`
- [ ] H-16 — Rename misleading test new_opens_the_editor (test-quality, Low, leverage 1) — blockedBy: [H-13] — blocks: [] — verify: `grep -n 'fn new_opens_the_editor' tests/cli.rs`
- [ ] H-15 — Add a thin assert_cmd test for main.rs (test-coverage, Low, leverage 1) — blockedBy: [] — blocks: [] — verify: `cargo llvm-cov --summary-only`
- [ ] H-17 — Document run()'s exit-code contract (0/1/2) (docs, Low, leverage 1) — blockedBy: [] — blocks: [] — verify: `cargo doc --no-deps`
