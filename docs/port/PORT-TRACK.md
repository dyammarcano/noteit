# PORT-TRACK — plugin-host contract → noteit `plugin`

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

**De-identification verified:** `de_identified_no_private_literals` asserts no
forbidden source-project token survives into any rendered library skill.

## Phase B (noteit-native, on top of the ported contract) — DONE

| Piece | Rust file | Notes |
|-------|-----------|-------|
| noteit assets | `src/plugin/noteit.rs` | skill + 3 commands + 1 agent; `<%.McpCommand%>`-templated |
| Host backends | `src/plugin/hosts.rs` | `NoteitHost` for claude/codex/gemini; Claude = native `.claude-plugin/plugin.json`, codex/gemini = skills-forward + generic `plugin.json` + library skills |
| CLI command | `src/plugin/command.rs` | `PluginCmd` + `run`: list / install / status / uninstall; `--host <h>|all` |
| CLI wiring | `src/cli.rs` | `plugin` verb, `Invocation::Plugin`, `parse_plugin`; dispatched in `run()` BEFORE the DB opens (filesystem-only, no notes DB needed) |

**Install target:** `$HOME/.<host>/plugins/noteit/` (overridable via
`NOTEIT_PLUGIN_ROOT`). Writes via the ported `write_tree_atomic` (tmp+rename +
stale-sweep over `commands`/`agents`/`skills`).

**Suite:** 127 → 139 tests (+12: hosts 5, command 4, noteit 3). fmt/clippy green.
Env-mutating tests serialized via `plugin::ENV_LOCK` (edition-2024 `set_var` is a
data race otherwise).

**Real-binary smoke (verified):** `plugin list/install --host claude/status/
uninstall` install a correct Claude plugin tree with rendered
`noteit add "$ARGUMENTS"` bodies, `status` reflects install state, `uninstall`
removes cleanly.
