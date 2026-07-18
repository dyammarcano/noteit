# PORT-TRACK — aihost core contract → noteit `plugin`

| # | Module | Rust file | Ported tests | Status | Gates |
|---|--------|-----------|--------------|--------|-------|
| 1 | asset | `src/plugin/asset.rs` | Render (3), KindString, AssetByPath/ByKind + 2 renderer edge tests | ✅ done | test/fmt/clippy green |
| 2 | host | `src/plugin/host.rs` | trait-object dispatch (adapted) | ✅ done | green |
| 3 | registry | `src/plugin/registry.rs` | RegisterAndAll, ByName | ✅ done | green |
| 4 | write_tree | `src/plugin/write_tree.rs` | write+count, stale-sweep (authored — no Go test existed) | ✅ done | green |
| 5 | libraries | `src/plugin/libraries.rs` | sorted-index, fallback-desc, empty, **de-identification guard** | ✅ done | green |

**Suite:** 111 → 127 tests (+16). `cargo fmt --check` = 0, `cargo clippy
--all-targets -- -D warnings` = 0.

**Crates added:** none (std-only, as planned).

**Parity deviations** (per PORT-PLAN "Deviations log"): `Render` fallible only on
unterminated delimiter; `Status` writer generalized to `&mut dyn Write`; global
registries → owned structs; `DoctorReport`/`DoctorCheck` JSON tags dropped
(no in-scope consumer).

**De-identification verified:** `de_identified_no_lensr_literals` asserts no
`lensr` / `claude-only` substring in any rendered library skill.

## Phase B (noteit-native, on top of the ported contract) — TODO

- Concrete `Host` impls: Claude / Codex / Gemini install targets + manifests.
- noteit's own assets (commands/skill/agent) registered into an `AssetRegistry`.
- `noteit plugin install|list|status --host <h>` wired into `run_core`.
