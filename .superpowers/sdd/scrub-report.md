# De-identification Scrub Report

Date: 2026-07-17

## Purpose
Remove all references to the private project (internal codename redacted here as well) and
its internal source paths, and all absolute local filesystem paths, from git-tracked files,
ahead of publishing this repo publicly.

## Files changed

### docs/superpowers/specs/2026-07-17-noteit-design.md
- 1x greenfield target path (drive-rooted local path) -> generic `/repos/noteit` placeholder
- 1x "Adopted from [private project], verified by direct source scan" -> "a prior internal project's implementation"
- 3x private-project source-file citations with line numbers -> de-pathed algorithm descriptions ("The reference implementation's ...")
- 1x "[private project] obtains this by statically linking..." -> "The reference implementation obtains this by..."
- 1x URN-namespace contrast paragraph naming the private project twice -> generalized to "its own namespace (distinct from the source project's)... noteit is not the source project"
- 1x "deliberately diverging from [private project]" (+ a source-file:line citation) -> "diverging from the reference implementation"
- 1x live-example local path -> generic placeholder
- 1x "[private project] derives no project name" -> "The reference implementation derives no project name"
- 1x manifest-path example citing a local drive path -> generic placeholder
- 1x "[private project]'s repo-id logic is good..." -> "the reference implementation's repo-id logic is good..."
- 1x "matching [private project]'s strcmp selection" -> "matching the reference implementation's strcmp selection"
- Total: 11 private-project-name references removed, 4 absolute local-path occurrences removed.

### docs/superpowers/plans/2026-07-17-noteit.md
- 1x "deliberate divergence from [private project] (source-file:line citation)" -> generalized, path dropped
- 1x observed-output local path -> generic placeholder
- 2x doc-comment private-project references (URN_PREFIX comment, strcmp-selection comment) -> generalized to "the reference implementation"
- ~23x PowerShell `Set-Location` lines pointing at the local drive path -> generic placeholder path
- Many Rust string-literal examples embedded in the plan doc (illustrative, non-executing) using the local drive path -> generic placeholder paths
- 1x prose reference to two local drive paths -> generic placeholders
- 1x example executable path assignment using the local drive path -> generic placeholder
- Total: 4 private-project-name references removed, ~30 absolute local-path occurrences removed.

### src/repoid.rs (comments only, no logic changed)
- 2x private-project-name references in doc comments -> generalized to "the reference implementation"

### tests/store.rs (executable test code)
- All string-literal test fixtures keyed on the local drive path (root, `\src` child, sibling,
  and two adjacent-prefix-but-not-descendant variants) rewritten to a neutral Windows-style
  placeholder root instead of the real local path.
- Note: an initial attempt used forward-slash placeholders, but this broke 3 tests because
  `path_contexts_under` builds its LIKE-prefix using `std::path::MAIN_SEPARATOR` (backslash on
  Windows) — the test fixtures must stay backslash-separated to match real code behavior.
  Reverted to a neutral Windows-style placeholder root, preserving test semantics.
- Total: ~30 absolute local-path occurrences removed/rewritten.

### docs/analysis/HARDENING-CHECKLIST.md
- 2x `Test-Path` verify commands citing the local drive path -> relative path (drive/root dropped)

### docs/analysis/HARDENING-RUNBOOK.md
- 2x `Test-Path` verify commands citing the local drive path -> relative path

### .superpowers/sdd/harden-phase2-report.md
- 1x captured `cargo audit` command output containing a real user-profile-rooted local path
  for the advisory-db cache -> generalized to an env-var-based placeholder

### .github/workflows/ci.yml
- `on.push.branches` and `on.pull_request.branches` changed from `[main]` to `[main, master]` so CI
  triggers regardless of default branch name (this repo's branch is `master`).

## Verification
- Case-insensitive search for the private project's codename across all tracked files -> empty
- Search for drive-rooted local filesystem paths across all tracked files -> empty (this report
  intentionally avoids quoting the raw matched patterns so it does not itself reintroduce a hit)
- `cargo test` -> 74 passed, 0 failed (across 8 integration test binaries + lib/main unit tests)
- `cargo fmt --check` -> exit 0
- `cargo clippy --all-targets -- -D warnings` -> exit 0

## Occurrences I was unsure how to rewrite
None outstanding — all matches were resolved. The one judgment call was tests/store.rs's
path-literal test fixtures: initially rewritten to forward-slash placeholders per general
guidance for "illustrative store-key examples", but this class of file is NOT illustrative —
it's executable test code whose assertions depend on `std::path::MAIN_SEPARATOR` (backslash on
Windows), so it was corrected to a neutral Windows-style placeholder path instead, after a full
`cargo test` run confirmed the regression and then the fix.
