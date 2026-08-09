## Why

A systematic review across the Rust bounded contexts and the React frontend turned up a class of defects that share a shape: a blocking call held a lock or ran on the wrong thread, a query paid per-row instead of per-batch, or a runtime fallback hid a missing implementation behind fabricated data. None changed the public contract, but each could wedge the UI or silently serve wrong results:

- The CLI/API process monitors and the PTY terminal reader reaped their child by locking `Arc<Mutex<Child>>` and calling `wait()` while holding the lock. `stop_generation` / `stop()` lock the same `Arc` to kill a runaway process, so a CLI that closed stdout but stayed alive (daemonized or detached grandchildren) deadlocked user-initiated cancellation and leaked the process tree. `BlockingStderrDrain::finish` joined its worker with no deadline, so an MCP stdio relay could wedge shutdown forever if a grandchild held the stderr pipe. The API adapter's `monitor_generation` lacked the `monitoring` guard the CLI adapter has, so a double-monitor could race two `run_generation` threads.
- `apply_migration` recorded its version row outside any transaction, so a mid-migration failure left DDL applied while `schema_migrations` never recorded the version — a re-run relied on `IF NOT EXISTS` idempotency that data-bearing migrations do not guarantee. Version-number collisions across shared local databases were silent (the second migration claiming a number never ran; its table was just missing at startup).
- The `cli_config` command family returned `Result<T, String>` via `e.to_string()`, bypassing the `CommandError` redaction framework and forwarding absolute filesystem paths (profile JSON, auth.json) to the frontend. Several `From` implementations forwarded lower-layer messages verbatim.
- `list_code_index_workspaces` ran one `workspace_status()` per workspace (each ~10 COUNT subqueries); `load_code_candidates` ran one query per source_id; `list_run_views` hydrated iterations per-run and evidence per-iteration (1+N+N×M). `get_session_git_diff` ran a full `git status` directory walk just to decide whether one path was untracked, then spawned git again for the diff.
- Five sync `#[tauri::command]`s (`get_session_git_status`, `get_session_git_diff`, `list_session_logs`, `export_session_logs`, `list_session_directory`) ran blocking git (30s timeout) / file / dialog I/O on the Tauri main thread. `list_connectors` ran synchronous rusqlite + credential I/O inline on the async executor.
- Each `token`/`thinking` chat stream event rebuilt the whole message array (O(n)) and re-rendered every subscriber once per token. PTY output was written to xterm per chunk with no coalescing.
- When `web-http` was selected but a service had no `webHttp` adapter, `createRuntimeAdapter` silently fell through to the web mock — an HTTP deployment looked healthy while every read returned fabricated data. None of the 16 runtime clients implements `webHttp` today, so any web-http deployment was silently mock-backed.

## What Changes

- Process/terminal monitors and `terminate_terminal_child` reap the child without holding the lock across the blocking `wait()` — they poll `try_wait()` with short lock holds so a concurrent `stop()` kill can proceed. The API adapter gains the `monitoring` guard. `BlockingStderrDrain::finish` takes a deadline and abandons the worker on timeout, mirroring the tokio variant.
- `apply_migration` wraps the schema change and the version row in one `unchecked_transaction`, so a mid-migration failure rolls back. After applying, the runtime asserts the recorded history is dense and within the expected version range, turning a diverged `schema_migrations` table into an explicit startup error rather than an opaque "no such table" crash.
- The `cli_config` command family routes through `CommandError` + `map_command_error`; a `From<CliConfigError>` maps path-bearing variants to fixed category-level messages. Lower-layer messages forwarded verbatim (`CliError::Internal`, `SdkError::Package`, `SessionsError::Repository`, `McpError::Database`, etc.) are redacted at the `From` boundary via `CommandError::redacted`, leaving structured error codes (`connector-credentials-required`) untouched.
- Repository reads that were one-query-per-item became single batched queries: `load_code_candidates` (`WHERE source_id IN (...)`), `list_run_views` (bulk iterations + evidence in two queries), `workspace_statuses` (one query with correlated COUNTs + a `row_number()` window for the latest failure), and `reconcile_apply` (one transaction with prepared statements for the whole upsert + orphan-delete diff). A new migration adds `idx_loop_evidence(iteration_id, created_at)`.
- The five blocking workspace/desktop commands are `async fn` delegating to `WorkspaceApi::*_blocking` wrappers that run on `spawn_blocking`; `list_connectors` snapshots DB + credentials on `spawn_blocking`, awaits transport health, then assembles. The git diff preflight uses `git ls-files --error-unmatch -- <path>` instead of a full `git status` walk.
- Chat stream events are buffered and flushed on `requestAnimationFrame` (`applyChatEvents` does one traversal per batch); PTY output chunks are coalesced into one `terminal.write` per frame. Terminal events flush immediately so the stop indicator is not delayed.
- `createRuntimeAdapter` throws an explicit "no HTTP adapter" error when `web-http` is selected but no `webHttp` adapter is provided, instead of silently serving the mock. The app's bootstrap-failure handler surfaces it as a recovery panel at startup.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-runtime-architecture`: child process reaping must not hold the child lock across the blocking wait; migration application is transactional and the recorded history is verified dense at startup; command errors redact forwarded lower-layer messages at the boundary.
- `runtime-performance-governance`: blocking git/log/directory work moves off the Tauri main thread; repository reads batch instead of per-row; chat stream events apply in batched traversals.
- `frontend-runtime-architecture`: a `web-http` runtime with no HTTP adapter fails loudly instead of serving the mock; streaming events are coalesced before reaching the query cache.

## Impact

**Runtime scope: both.** Native Rust (agent_runtime / sessions / tooling / retrieval / communications / workspaces / database / process) and React/TypeScript frontend (services / hooks / session-workspace). No public Tauri command name or parameter changed; no database schema migration was modified in place (migration 54 is additive). No new dependencies.

Affected files (representative):
- `src-tauri/src/contexts/agent_runtime/infrastructure/process_adapter.rs`, `terminal_process.rs`, `api_process_adapter.rs`
- `src-tauri/src/platform/process/stderr_drain.rs`, `src-tauri/src/contexts/tooling/mcp/infrastructure/relay_stdio.rs`
- `src-tauri/src/platform/database/migrations.rs`, `src-tauri/src/contexts/retrieval/application/ports.rs`, `infrastructure/sqlite_repository.rs`, `code_index_repository.rs`
- `src-tauri/src/contexts/agent_runtime/infrastructure/loop_repository_views.rs`
- `src-tauri/src/commands/tooling/cli_config/*.rs`, `src-tauri/src/commands/error.rs`
- `src-tauri/src/contexts/workspaces/api.rs`, `infrastructure/session_queries.rs`, `src-tauri/src/commands/workspaces/*.rs`
- `src-tauri/src/contexts/communications/application/service.rs`, `api.rs`
- `src/services/chat-events.ts`, `runtime-adapter.ts`, `src/hooks/use-active-session-chat.ts`, `src/main-layout/use-main-layout-model.ts`, `src/session-workspace/shell-tab.tsx`, `src/floating-assistant/floating-assistant-app.tsx`

Downstream: HTTP deployments that previously appeared to work now fail fast with a diagnosable message (they were silently mock-backed before). All other changes are behavior-preserving.
