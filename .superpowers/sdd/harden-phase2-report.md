# Hardening Phase 2 ("Harden") — Execution Report

**Status: DONE**

## Commits per step

| Step | Item(s) | Commit SHA | Message |
|---|---|---|---|
| 1 | H-05 | `42eb867` | style: cargo fmt across the tree (H-05) |
| 2 | H-07/H-08/H-09 | `aefda69` | fix: clear clippy collapsible_if, map_or, dead_code warnings (H-07/H-08/H-09) |
| 3 | H-06 | `7074aff` | build: add clippy-deny lint gate (H-06) |
| 4 | H-11 | `7dee57c` | build: pin toolchain via rust-toolchain.toml (H-11) |
| 5 | H-10 | (no commit; wired into H-04's ci.yml, `c4b209e`) | cargo-audit run locally only |
| 6 | H-04 | `c4b209e` | ci: add GitHub Actions workflow (fmt/clippy/test/audit) (H-04) |
| docs | — | `a52135a` | docs(harden): mark Phase 2 items done |

## Step details

### Step 1 — H-05 cargo fmt
Ran `cargo fmt` tree-wide (mechanical formatting only). Verified `cargo fmt --check` exits 0
and `cargo test` shows 74 passed (11+16+6+2+9+7+23 across 7 integration test binaries).

### Step 2 — H-07/H-08/H-09
- `src/cli.rs`: collapsed the nested `if !matches!(...) { if let Some(r) = ... }` into a
  single `if !matches!(inv, Invocation::Adopt { undo: true }) && let Some(r) =
  adopt_if_needed(&mut store, &resolved)? { ... }`. **Let-chains compiled successfully**
  on the installed toolchain (rustc 1.96.0) — no restructuring fallback was needed.
- `src/repoid.rs`: replaced `best.as_ref().map_or(true, |b| hex < *b)` with
  `best.as_ref().is_none_or(|b| hex < *b)`.
- `tests/common/mod.rs`: added `#![allow(dead_code)]` as the first line of the file.

`cargo clippy --all-targets` → zero warnings. `cargo test` → 74 passed.

### Step 3 — H-06 clippy-deny gate
Added to `Cargo.toml`:
```toml
[lints.clippy]
all = { level = "deny", priority = -1 }
```
`cargo clippy --all-targets` exits 0 (no new denials — tree was already clean from step 2).
`cargo test` → 74 passed.

### Step 4 — H-11 pin toolchain
Created `rust-toolchain.toml`:
```toml
[toolchain]
channel = "1.96.0"
components = ["rustfmt", "clippy"]
```
`cargo test` still builds/passes (74 tests) under the pinned toolchain.

### Step 5 — H-10 cargo-audit
`cargo install cargo-audit` succeeded (v0.22.2, ~6m18s build — bundled rusqlite/gix
dependency chain took the bulk of the time). Ran `cargo audit`:

```
    Fetching advisory database from `https://github.com/RustSec/advisory-db.git`
      Loaded 1166 security advisories (from %USERPROFILE%\.cargo\advisory-db)
    Updating crates.io index
    Scanning Cargo.lock for vulnerabilities (179 crate dependencies)
EXIT:0
```

**Result: 0 advisories / vulnerabilities found** across all 179 crate dependencies. No
`audit.toml` was needed (nothing to ignore). `Cargo.lock` was not modified as a result of
this step. No commit was made for this step per instructions — the audit tooling and its
usage were captured directly in the CI workflow added in Step 6.

### Step 6 — H-04 CI workflow
Created `.github/workflows/ci.yml` with:
- `actions/checkout@v4`
- `dtolnay/rust-toolchain@stable` (components: rustfmt, clippy) — honors
  `rust-toolchain.toml` automatically via rustup
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all-targets`
- A separate `audit` job installing cargo-audit via `taiki-e/install-action@v2` and
  running `cargo audit` as a **blocking** step (no advisories were found in Step 5, so
  there was no reason to make it non-blocking).

Triggers: `push`/`pull_request` on branch `main`. A top-of-file comment notes the repo
currently has no git remote configured (`git remote -v` returned nothing at execution
time), so this workflow activates once pushed to GitHub.

## Final full round

- `cargo test` → **74 passed**, 0 failed (across `noteit` lib/bin unit tests + 7
  integration test binaries: adoption 11, cli 16, context 6, editor 2, render 9,
  repoid 7, store 23; doc-tests 0).
- `cargo fmt --check` → **exit 0**.
- `cargo clippy --all-targets -- -D warnings` → **exit 0**.

## Docs updated

- `docs/analysis/HARDENING-CHECKLIST.md` — checked off H-04, H-05, H-06, H-07, H-08,
  H-09, H-10, H-11 each with a `DONE <shortsha>` note; rev bumped 001→002.
- `docs/analysis/HARDENING-RUNBOOK.md` — updated the `**Outcome:**` line for each of the
  same 8 items with concrete evidence (commit sha, verify command results); rev bumped
  001→002.
- Both files existed with the exact structure anticipated by the task instructions — no
  discrepancy to report.

## Concerns / notes

- **Let-chains compiled without issue** on rustc 1.96.0 (edition 2024) — no fallback
  restructuring was required for H-07.
- **cargo audit found zero advisories** — nothing to fix, no `audit.toml` created.
- `cargo install cargo-audit` took ~6.5 minutes to build from source (long dependency
  chain via `gix`/`reqwest`/`rustls`); ran in the background with polling per the task's
  guidance.
- No git remote is configured in this worktree, so the new CI workflow has not actually
  run on GitHub Actions yet — it will activate on first push, as noted in the workflow's
  top-of-file comment.
- No other blockers encountered.
