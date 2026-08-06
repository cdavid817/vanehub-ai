## Why

The DAG-based Multi-Agent coordination approach is being abandoned. The capability shipped a complete backend on 2026-07-23 — plan validation, dependency-aware scheduling, ordered Agent failover, a SQLite-backed run store, and four Tauri commands — but no user-facing surface was ever delivered for it. The one attempt at a UI was built and then reverted, so today the capability has zero consumers: every line of it is unreachable from the product while still carrying spec obligations, a database table, a background scheduler, and an executor that must keep compiling against the evolving agent runtime. Retiring it removes that maintenance burden instead of preserving an approach the product is not pursuing.

## What Changes

- **BREAKING** Remove the `multi-agent-coordination` capability in full. All seven of its requirements are retired with no replacement surface.
- **BREAKING** Remove the four coordination methods (`startCoordination`, `listCoordinationRuns`, `getCoordinationRun`, `cancelCoordinationRun`) from the `AgentService` frontend service boundary and from both the Tauri and Web/mock adapters, which MUST stay interface-identical.
- **BREAKING** Remove the four coordination Tauri commands and unregister them from the command registry, so the desktop runtime no longer exposes a coordination API.
- **BREAKING** Drop the `coordination_runs` SQLite table and stop creating it on startup. Existing local rows are discarded; they are unreachable execution history for a retired feature and no export path is provided.
- Remove the Rust domain, application, infrastructure, scheduler, executor, repository, and schema modules for coordination, along with the coordination variants in the shared DTO, mapper, and error types.
- Remove the frontend coordination runtime (`src/services/coordination-runtime.ts`), its types (`src/types/coordination.ts`), and their tests.
- Remove the coordination page from the developer guide and its `SUMMARY.md` index entry.
- Narrow execution-observability requirements so tracing and metrics no longer promise coordination-node or failover coverage.
- Preserve the archived `2026-07-23-add-multi-agent-coordination` artifacts as immutable history; this change supersedes that decision rather than rewriting the archive.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `multi-agent-coordination`: All requirements are removed and the capability is retired — plan graph, dependency-aware scheduling, prerequisite output propagation, ordered Agent failover, durable lifecycle, query/cancellation boundary, and safe diagnostics.
- `agent-execution-observability`: Removes coordination nodes and failover attempts from the correlated-trace and bounded-metrics requirements, and removes the coordination-fallback observation scenario. Tracing of ordinary task, Agent, process, tool, and MCP stages is unchanged.
- `user-guide-documentation`: Removes the requirement that the guide set demonstrate a representative Multi-Agent coding workflow, which mandated documenting task decomposition, primary/fallback selection, dependency graphs, output propagation, and cancellation — all behaviours this change retires.
- `multilingual-readme`: Removes the scenario governing how READMEs describe Multi-Agent coordination before UI delivery. The surrounding requirement that README claims reflect implemented state is unchanged and still governs every other feature.

## Impact

- **Desktop and Web runtimes:** Both. The desktop runtime loses the coordination commands, scheduler, and SQLite store; the Web/mock adapter loses its simulated coordination implementation. No other Agent, session, loop, or scheduled-task behavior changes.
- **Frontend:** Deletes the coordination runtime, types, and tests, and removes four methods from the service boundary. No React component consumes coordination today, so no UI is affected.
- **Backend:** Deletes eleven Rust modules and edits roughly ten more that reference them — bootstrap wiring, command registry, DTO, mapper, and the application/domain error enums.
- **Database:** Removes the `coordination_runs` table. This is the only user-data-affecting part of the change and is irreversible for existing local installs.
- **Architecture:** No change to frontend/backend isolation. The service boundary stays the single dependency for React, and the Tauri and Web adapters remain interface-identical after the four methods are removed from both.
- **Not in scope:** A separate defect found while testing this area — claude-code `result` events with `is_error: true` are parsed as successful completions in `providers/output.rs`, discarding the CLI's own error text — affects all CLI agent execution paths and needs its own change.
