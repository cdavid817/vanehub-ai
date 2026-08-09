## Context

This change captures a code-review-driven hardening pass. It does not add features; it removes failure modes that a feature-driven spec would not have surfaced: a lock held across a blocking wait, a per-row query inside a hot path, a synchronous spawn on the main thread, and a fallback that fabricated data instead of failing.

## Decisions

### Decision 1: Poll `try_wait()` instead of calling `wait()` while holding the child lock

`stop_generation` / `stop()` and the process monitor both need the same `Arc<Mutex<Child>>`. Holding the lock across `wait()` — which blocks until the child exits — deadlocks cancellation whenever the child closes its stdout/PTY but stays alive. Alternatives considered:

- *A separate `tokio::sync::Mutex`*: would not help; the deadlock is about lock-across-blocking-call, not async-vs-sync.
- *A cancel flag the monitor checks before `wait()`*: the monitor is already in `wait()`, so a flag set after the fact cannot interrupt it.
- *Job Object / process-group containment*: `platform/process` already does this for the bounded execution path; the agent-runtime monitors predate it and did not reuse it. Reusing it is a larger change deferred here.

Polling `try_wait()` with 50ms holds lets a concurrent `kill()` acquire the lock between polls. The poll interval is short enough that a normally-exiting child is reaped promptly; it is the unbounded `wait()` that was the bug, not its latency.

### Decision 2: Redact at the `From` boundary, not at `Serialize`

An earlier draft applied `redact_text` once at `CommandError`'s `Serialize` impl. That mangled structured error codes like `connector-credentials-required` (the heuristic treated "credentials" as a sensitive key and appended `=[REDACTED]`). Redaction belongs where lower-layer messages are forwarded verbatim — the `From` variants — so category-level safe codes pass through unchanged. A `CommandError::redacted` constructor localizes the scrubbing to the variants that need it.

### Decision 3: Migration density check, not name-match check, at startup

`apply_migration` is version-gated; a collision (two migrations claiming the same number) is silently skipped because the first claim fills the version row. A name-match check against an `EXPECTED_MIGRATIONS` constant would catch collisions precisely but would make a shared local database already in a collided state unbootable — worse than the "no such table" crash it would hit later. The startup check verifies density + upper-bound only; name parity is asserted in tests (`migration_sequence_matches_expected`) where a collided fixture does not block real deployments.

### Decision 4: `web-http` throws instead of falling back to mock

`createRuntimeAdapter` fell through to `webMock` when `webHttp` was absent, so an HTTP deployment silently served fabricated data. None of the 16 runtime clients implements `webHttp` today, so any `web-http` deployment was mock-backed. Throwing surfaces the gap at startup; `main.tsx`'s bootstrap-failure handler already renders a recovery panel for module-load errors, so the failure is diagnosable rather than a white screen. Implementing the actual HTTP adapters is out of scope and tracked as a follow-up.

### Decision 5: Batched queries, not per-row, on hot paths

`load_code_candidates`, `list_run_views`, `workspace_statuses`, and `reconcile_apply` all had the same shape: one round-trip per item. Each now prepares its statement(s) once and binds a batch. `workspace_statuses` uses a `row_number()` window to pick each workspace's latest `failure_category` in one query instead of a per-group `LIMIT 1` subquery. The batched results are asserted equal to the per-item results in tests so the refactor cannot drift.

### Decision 6: `spawn_blocking` lives in the api layer, not the command layer

The architecture test forbids IO primitives (`spawn_blocking`, `query_row`, …) in command adapters. The five workspace commands delegate to `WorkspaceApi::*_blocking` async wrappers that own the `spawn_blocking` call; `list_connectors` snapshots on `spawn_blocking` from `CommunicationsApi`, awaits transport health, then assembles on the pool. The application service layer stays free of `tauri::async_runtime`.

## Risks

- **`web-http` deployments now fail fast.** This is intended (they were silently mock-backed) but is a visible behavior change. Mitigation: the failure message names the gap and the bootstrap panel renders it.
- **Polling `try_wait()` introduces up to 50ms of reap latency.** Negligible vs. the unbounded `wait()` it replaces, and only on the monitor/stop path.
- **Batched SQL with `IN (...)` placeholders** is built per-call; the placeholder count is bounded by the batch size (workspace list, run list, source-id candidates are all bounded).
