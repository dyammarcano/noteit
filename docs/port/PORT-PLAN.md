# PORT-PLAN — aihost core contract → noteit `plugin` module

Faithful 1:1 port of the **core plugin-host contract** from
`D:\lensr\pkg\aihost` into `noteit/src/plugin/`. Test-first, std-only (zero new
crates). Source is the user's PRIVATE lensr repo; noteit is PUBLIC — all
lensr-specific literals are de-identified (see module 5).

## Scope

IN: `asset.go`, `host.go`, `registry.go`, `write_tree.go`,
`portable_libraries.go` + tests `asset_test.go`, `registry_test.go`,
(lint logic from `prompt_lint_test.go`, adapted — see below).

OUT (never enters public noteit): the entire `aihost/claude/` subpackage,
`assets_*.go`, and any lensr-registered asset content. `prompt_lint_test.go`
imports `pkg/aihost/claude` → its lensr-coupled body is NOT ported; only the
generic lint *property* is re-expressed against synthetic fixtures.

## Module graph & dependency order

1. **asset** (`asset.rs`) — `Kind`, `TemplateData`, `Asset`, `Render`,
   `AssetRegistry`. No intra-module deps. Foundation.
2. **host** (`host.rs`) — `Host` trait + optional `Installer`/`Status`/`Doctor`
   capabilities, `DoctorCheck`/`DoctorReport`. Depends on nothing (pure traits).
3. **registry** (`registry.rs`) — host factory registry (`HostRegistry`).
   Depends on `host`.
4. **write_tree** (`write_tree.rs`) — `TreeWriter` + `write_tree_atomic`.
   Depends on nothing (its own trait).
5. **libraries** (`libraries.rs`) — de-identified `portable_library_skills`.
   Depends on `asset` (`AssetRegistry::by_kind`).

Port order: asset → host → registry → write_tree → libraries.

## Key porting decisions (std-first, idiomatic)

- **Template rendering** — hand-rolled std-only `<%.Field%>` substituter over the
  5 `TemplateData` fields. No `text/template` equivalent needed: `asset_test.go`
  asserts only substitution + created-marker behavior, never a parse-error path.
  Unknown `.Field` → placeholder left untouched (passthrough). An unterminated
  `<%` → `RenderError::UnterminatedDelimiter` (keeps the fallible signature
  meaningful; Go errored on parse, we error on the one malformable input).
- **Global registries → explicit structs.** Go's `var assetRegistry`/`var
  registry` rely on `init()` side effects Rust lacks. Idiomatic 1:1-in-spirit:
  `AssetRegistry` and `HostRegistry` value types with `register`/`by_*`/`all`
  methods. Tests construct fresh instances (no global reset dance needed).
- **`interface` → `trait`.** `Host`, `Installer`, `Status`, `Doctor`,
  `TreeWriter`. `Status.PrintStatus(*os.File)` → `&mut dyn Write` (more general).
- **`error` → `io::Result`** for filesystem/host ops; `Render` returns
  `Result<Vec<u8>, RenderError>`.
- **`DoctorReport`/`DoctorCheck`** → plain structs; Go's `json` tags dropped (the
  only consumer was the deferred `lensr.v1.plugin_doctor` MCP tool — out of
  scope, no serde crate pulled in).
- **De-identification (`libraries.rs`)** — strip every lensr literal:
  `lensr-command-library`→`command-library`, `lensr-agent-library`→
  `agent-library`, drop `lensr.v1.*` (→ generic "active MCP server"),
  "Claude-only"→"host-specific". A test asserts the rendered skills contain no
  "lensr" substring — a public-repo guard.

## License / provenance

Source `doc.go` claims BSD-3/inovacc but `D:\lensr\LICENSE` is MIT
(dyammarcano, 2026). Not a copyleft blocker; MIT is compatible with noteit's
BSD-3. Clean-room-in-spirit: behavior preserved, form re-authored in Rust.

## Deviations log (parity notes)

- `Render` is infallible except for unterminated-delimiter; Go's parse-error and
  execute-error paths collapse to that one case (no `text/template` engine).
- Status writer generalized from `*os.File` to `&mut dyn Write`.
- Global mutable registries → owned structs (thread-safe by construction; no
  `OnceLock`/`Mutex` needed since state is no longer global).
