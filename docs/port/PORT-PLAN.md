# PORT-PLAN — plugin-host contract → noteit `plugin` module

Faithful 1:1 port of a **core plugin-host contract** from an internal Go
plugin-host package into `noteit/src/plugin/`. Test-first, std-only (zero new
crates). The source lives in a private project; noteit is PUBLIC — every
source-project-specific literal is de-identified (see module 5).

## Scope

IN: the core contract's `asset` / `host` / `registry` / `write_tree` /
`portable_libraries` units + their tests.

OUT (never enters public noteit): the source project's host-specific subpackage
and all of its private asset content. A source lint test that imports that
private subpackage is NOT ported verbatim; only the generic lint *property* is
re-expressed against synthetic fixtures.

## Module graph & dependency order

1. **asset** (`asset.rs`) — `Kind`, `TemplateData`, `Asset`, `Render`,
   `AssetRegistry`. Foundation.
2. **host** (`host.rs`) — `Host` trait + optional `Installer`/`Status`/`Doctor`
   capabilities, `DoctorCheck`/`DoctorReport`. Pure traits.
3. **registry** (`registry.rs`) — host factory registry (`HostRegistry`).
   Depends on `host`.
4. **write_tree** (`write_tree.rs`) — `TreeWriter` + `write_tree_atomic`.
5. **libraries** (`libraries.rs`) — de-identified `portable_library_skills`.
   Depends on `asset`.

Port order: asset → host → registry → write_tree → libraries.

## Key porting decisions (std-first, idiomatic)

- **Template rendering** — hand-rolled std-only `<%.Field%>` substituter over the
  5 `TemplateData` fields. No `text/template` equivalent needed: the source tests
  assert only substitution + created-marker behavior, never a parse-error path.
  Unknown `.Field` → placeholder left untouched (passthrough). An unterminated
  `<%` → `RenderError::UnterminatedDelimiter`.
- **Global registries → explicit structs.** Go's package-`init()` globals have no
  Rust equivalent; idiomatic 1:1-in-spirit is owned `AssetRegistry` /
  `HostRegistry` value types. Tests construct fresh instances.
- **`interface` → `trait`.** `Host`, `Installer`, `Status`, `Doctor`,
  `TreeWriter`. `Status::print_status` takes `&mut dyn Write` (generalized from
  `*os.File`).
- **`error` → `io::Result`** for filesystem/host ops; `Render` returns
  `Result<Vec<u8>, RenderError>`.
- **`DoctorReport`/`DoctorCheck`** → plain structs; the source's `json` tags are
  dropped (the only consumer was a deferred MCP tool — out of scope, no serde
  crate pulled in).
- **De-identification (`libraries.rs`)** — strip every source-project literal:
  the private command/agent library skill names and paths, the private MCP tool
  namespace (→ generic "active MCP server"), and host-name-specific phrasing
  (→ "host-specific"). A guard test asserts the rendered skills contain none of
  the forbidden tokens.

## License / provenance

The source package's license is MIT — compatible with noteit's BSD-3.
Clean-room-in-spirit: behavior preserved, form re-authored in Rust. No private
paths, names, or asset content are reproduced here.

## Deviations log (parity notes)

- `Render` is infallible except for an unterminated delimiter; the source's
  parse-error and execute-error paths collapse to that one case (no
  `text/template` engine).
- `Status` writer generalized from `*os.File` to `&mut dyn Write`.
- Global mutable registries → owned structs (thread-safe by construction).
- **`write_tree` path identity:** the source keys its "wanted" set with a purely
  lexical clean *before* writing; the Rust port uses `fs::canonicalize` (which
  needs the file to exist, so it inserts *after* the write). Both sides resolve
  paths the same way internally, so set-membership for the sweep is consistent
  and functionally equivalent; the only difference is a symlinked component under
  the target would be compared resolved (Rust) vs lexical (Go). Low impact.
