## Why

`src-tauri` is one `[package]` holding 21 bounded contexts, 87 dependencies, and a second binary. Any change recompiles everything, and `cargo test` cannot be scoped to what changed. Item 9 of the optimization ticket proposes migrating to a Cargo workspace, staged so CI stays green at each step.

Measuring the crate before starting changes two of the ticket's staged steps.

### `platform` is not a base layer, so it cannot be extracted first

The ticket's step 2 is "extract the base layer with no business dependencies: `vanehub-platform` (database, private relay fs)". `platform/` references bounded contexts **61 times across 10 contexts**, and **55 of those are in one file**: `platform/database/migrations/mod.rs`, where `migrate()` calls each context's `apply_schema` directly.

The dependency runs platform → contexts, which is the opposite of what a base crate needs. Extracting `vanehub-platform` requires first inverting it, so contexts register their schema with the migration runner instead of the runner reaching into them. That inversion is a design change to the migration mechanism, with a spec requirement attached (`native-runtime-architecture`, "Migration application is transactional with startup density verification"), and it is a prerequisite the ticket does not mention.

### One of the two suggested pilots is the wrong choice

The ticket suggests `web_research` and `retrieval` as low-coupling pilots. Measured outbound coupling — how many other contexts each one reaches into:

| Context | Outbound contexts | Outbound refs |
|---|---:|---:|
| `goals`, `operations`, `retrieval`, `work_board` | **0** | 0 |
| `web_research` | 2 | 2 |
| `agent_runtime` | 10 | 161 |

`retrieval` is a good call — zero outbound. `web_research` is not: it depends on `agent_runtime`, which is the most coupled context in the crate. Extracting it means either dragging `agent_runtime` along or stubbing the edge.

Four contexts have zero outbound coupling, and their inbound counts decide the blast radius:

| Context | Outbound | Inbound (files) | `.rs` files | Lines |
|---|---:|---:|---:|---:|
| `work_board` | 0 | **0** | 5 | 765 |
| `goals` | 0 | **0** | 15 | 1,363 |
| `retrieval` | 0 | **0** | 35 | 11,500 |
| `operations` | 0 | **51** | 19 | 4,037 |

`work_board` and `goals` are fully isolated in both directions. `operations` is the de facto foundation — nothing it needs, 51 files that need it.

## What Changes

This change delivers the **skeleton and the one extraction that has no prerequisites**, and leaves the rest as sequenced follow-ups. The full arc is in design.md.

- Add a root `[workspace]` with `src-tauri` as its only member, plus `[workspace.dependencies]` carrying the 87 shared version declarations.
- Extract `vanehub-permission-hook` into its own member crate, removing the `default-run = "vanehub-ai"` workaround in `src-tauri/Cargo.toml:11` that exists only because two binaries share one package.
- Keep `crate-type = ["staticlib", "cdylib", "rlib"]` on the Tauri crate alone.
- Update the CI Rust job to `--workspace`.
- **No context is extracted and no source file moves**, apart from the permission-hook binary. The skeleton is proved on its own before anything depends on it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. Build-graph reorganisation with no externally observable behaviour change: no Tauri command, SQLite schema, adapter contract, or runtime behaviour is affected in either the desktop or Web runtime. The change sets `skip_specs: true`.

Note that the *later* phase inverting the migration dependency will touch `native-runtime-architecture` and will need its own delta. That is one reason it is not in this change.

## Impact

- New root `Cargo.toml` — `[workspace]`, `[workspace.dependencies]`, `[workspace.lints]`.
- `src-tauri/Cargo.toml` — dependencies switch to `workspace = true`; `default-run` removed.
- New `crates/vanehub-permission-hook/` — the second binary and its manifest.
- `.github/workflows/ci.yml` — Rust job switches to `--workspace`.
- `scripts/prepare-permission-hook-sidecar.mjs` and the Tauri sidecar config — the hook binary's output path changes, and `npm run sidecar:prepare` must keep finding it.
- `src-tauri/tests/architecture.rs` — path budgets are recorded relative to the repository root and are unaffected, but the subtree walker must not start counting the new crate twice.
