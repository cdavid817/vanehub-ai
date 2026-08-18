## Context

See proposal.md — Why for the coupling measurements and for the two ticket steps they invalidate.

## Goals / Non-Goals

**Goals:**

- Get a working workspace with the packaging chain intact, proved before any context depends on it.
- Remove the `default-run` workaround, which exists only because two unrelated binaries share one package.
- Leave a sequenced, measured plan for the extractions this change does not perform.

**Non-Goals:**

- Extracting any bounded context. The prerequisites differ per context and the first one is not ready.
- Extracting `vanehub-platform`. It is not a base layer yet; see below.
- Reducing compile time in this change. The skeleton alone changes nothing there — one member compiles like one package. The gain arrives with the first real split, and this change records a baseline to measure it against.

## Decisions

### Ship the skeleton with one member and prove the packaging chain first

A workspace with a single member is a no-op for the compiler and a real test of everything around it: `npm run tauri -- dev`, `npm run package`, the sidecar preparation script, the Tauri bundler's expectations about target directories, and the three Desktop Smoke platforms.

Those are where a workspace migration actually breaks, and finding out with one member is cheap. Finding out three crates later is not.

### Extract `vanehub-permission-hook` now, because it is the one piece with no prerequisites

It is a separate binary that shares a package purely by history, and that sharing is what forces `default-run = "vanehub-ai"` at `src-tauri/Cargo.toml:11`. Moving it to its own crate removes the workaround rather than carrying it into the workspace.

Its blast radius is the sidecar chain — `scripts/prepare-permission-hook-sidecar.mjs`, the Tauri sidecar config, and `npm run sidecar:prepare` — which is exactly the packaging surface this change wants to exercise anyway.

### The migration inversion is a prerequisite, and it belongs to its own change

`platform/database/migrations/mod.rs` calls `crate::contexts::<name>::infrastructure::apply_schema` for 51 of the 79 migrations. As long as that holds, `platform` cannot be a crate that contexts depend on, because it depends on them.

Inverting it means contexts register their schema with the migration runner. That is a real design change: it touches ordering guarantees, the `EXPECTED_MIGRATIONS` registry that catches version collisions, and a spec requirement about transactional application with startup density verification. Bundling it into a workspace-skeleton change would put the one guard that catches migration collisions into the same diff as a build-graph reorganisation.

### Pilot order, once the skeleton is proved

| Order | Crate | Why |
|---|---|---|
| 1 | `vanehub-permission-hook` | No prerequisites — in this change |
| 2 | `work_board` or `goals` | Zero inbound *and* zero outbound; 765 and 1,363 lines. A pilot should prove the pattern, not survive it |
| 3 | `retrieval` | Zero coupling but 11,500 lines — proves the pattern holds at scale |
| 4 | `operations` | Zero outbound, 51 inbound files. Highest value, largest mechanical edit; do it once the pattern is boring |
| 5 | migration inversion, then `vanehub-platform` | The prerequisite and the base layer |
| — | `agent_runtime`, `tooling`, `cli_delegation` | 10, 6 and 6 outbound contexts. Not extractable without work that is not a move |

The ticket's `web_research` pilot is dropped: it reaches into `agent_runtime`.

### `[workspace.dependencies]` now, `[workspace.lints]` deliberately empty

Hoisting all 87 version declarations is the point of the workspace for dependency management, and it is mechanical.

`[workspace.lints]` is added as a section but left carrying nothing new. The panic-shortcut gate introduced by `freeze-panic-shortcuts-in-production-code` is deliberately *not* a `Cargo.toml` lint — it is a target-scoped clippy invocation, because `[lints]` has no target selectivity and would fail on ~9,560 test sites. That reasoning does not change under a workspace, and `[workspace.lints]` inherits the same limitation.

## Risks / Trade-offs

- **The Tauri bundler or the sidecar script depends on the current target-directory layout** → This is the specific risk the single-member skeleton exists to surface. Desktop Smoke runs on all three platforms and is the check; a local pass on one OS is not evidence for the others.
- **`cargo test --workspace` changes which tests run** → Compare the test count against the current baseline; it must not fall.
- **Path budgets in `architecture.rs` are repository-root-relative and might double-count a moved crate** → Verified explicitly rather than assumed, since the budgets are the ratchet everything else relies on.
- **A workspace makes it tempting to extract the next context immediately** → The pilot order above is measured, not intuited, and the two contexts the ticket suggested are not both suitable.
- **No compile-time win lands in this change** → Stated in Non-Goals. Recording the baseline now is what makes the later claim checkable, and the last attempt to claim a compile-time win in this work stream was measured and withdrawn.

## Migration Plan

Each step leaves the tree buildable and packageable. Rollback is `git revert` plus `cargo clean`; no data, schema, or runtime surface is touched.
