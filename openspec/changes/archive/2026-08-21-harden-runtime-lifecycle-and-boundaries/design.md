## Context

The affected paths already expose stable service and command contracts. Process ownership is split between managed process wrappers and the workspace PTY registry; terminal UIs subscribe asynchronously; SQLite repositories are synchronous behind native APIs; observability is deliberately non-blocking. No schema or dependency change is required.

## Goals / Non-Goals

**Goals:**

- Make resource ownership explicit across normal exit, cancellation, partial initialization, and drop.
- Preserve primary operation behavior while making secondary persistence and telemetry failures observable.
- Bound hot-path work by batches, capacities, and narrow lock scopes.
- Restore the documented rule that concrete adapters are selected only by composition roots.

**Non-Goals:**

- Change Tauri command names, DTOs, database schema, terminal retention semantics, or Web/mock capabilities.
- Make optional telemetry failure determine Agent task success.
- Introduce a new async SQLite library or dependency.

## Decisions

1. **Use generation-aware terminal cleanup.** Reader completion will remove only the registry entry whose generation/identity it owns, then reap outside the registry lock. This avoids a stale reader removing a replacement. Frontend cleanup remains a second idempotent safety net. Merely asking the frontend to kill on `disconnected` was rejected because native ownership must survive renderer loss.

2. **Make required operation persistence fallible.** Initial manual-tool operation creation is part of admission and must succeed before side effects begin. Progress and terminal persistence remain best-effort for the primary tool outcome but emit safe diagnostics and retain a failure signal for the caller where the existing contract permits.

3. **Batch without schema changes.** Agent rows, modes, and tags use three bounded queries and in-memory grouping. Feedback current rows and event revisions use chunked `IN` queries to respect SQLite parameter limits. Single-entity methods remain unchanged.

4. **Use short registry snapshots.** Health collection clones stable `Arc` handles under the registry read lock, releases it, and then awaits per-connector state.

5. **Share a terminal output accumulator.** Agent terminal output is appended to a per-frame buffer with a maximum flush size. Replay storage uses access-ordered eviction with per-session and global byte ceilings. The service interface does not change.

6. **Publish ports, compose concrete adapters at the edge.** CLI delegation consumes an Agent Runtime-owned persistence port/API rather than `SqliteNativeToolRepository`. MCP relay composition constructs execution-observability infrastructure in the process bootstrap and passes a published telemetry interface into tooling.

7. **Diagnose telemetry without recursion.** A small logging adapter records safe categories such as `telemetry-start-failed` and `telemetry-finish-failed`; it never sends those diagnostics through the failed telemetry exporter.

## Risks / Trade-offs

- [Concurrent PTY EOF and explicit stop race] → Use identity-aware removal and idempotent wait ownership with deterministic tests.
- [Large `IN` queries exceed SQLite limits] → Chunk IDs at a conservative fixed size and merge results deterministically.
- [Diagnostics flood during exporter outage] → Reuse bounded/rate-limited unified diagnostic behavior and avoid raw error payloads.
- [Architecture guard exposes additional pre-existing violations] → Fix every newly enforced production violation in this change; test-only adapters remain scoped to test modules.

## Migration Plan

No data migration is required. Land lifecycle/error fixes, query/lock improvements, and boundary enforcement as separate commits. Rollback is commit-local because public contracts and stored data are unchanged.
