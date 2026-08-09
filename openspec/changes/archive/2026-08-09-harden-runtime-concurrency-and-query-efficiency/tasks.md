## 1. Break lock-across-wait deadlocks in process/terminal monitors

- [x] 1.1 `ProcessMonitor::run` reaps the child without holding the `Arc<Mutex<Child>>` lock across `wait()` (poll `try_wait()` with short holds)
- [x] 1.2 `terminal_process` reader thread reaps without holding the PTY child lock across `wait()`
- [x] 1.3 `terminate_terminal_child` kills inside the lock, releases it, then reaps outside
- [x] 1.4 `api_process_adapter::monitor_generation` gains the `monitoring` guard the CLI adapter has
- [x] 1.5 `BlockingStderrDrain::finish` takes a deadline and abandons the worker on timeout; `relay_stdio` passes it

## 2. Transactional migrations + startup density verification

- [x] 2.1 `apply_migration` wraps the schema change and the version row in one `unchecked_transaction`
- [x] 2.2 Assert the recorded `schema_migrations` history is dense and within the expected version range after applying
- [x] 2.3 `migration_sequence_matches_expected` test asserts `EXPECTED_MIGRATIONS` against `migrate()`; `density_check_rejects_a_missing_migration_row` test covers the gap case

## 3. Route command errors through `CommandError` redaction

- [x] 3.1 All 10 `cli_config` commands return `Result<T, CommandError>` via `map_command_error`
- [x] 3.2 `From<CliConfigError> for CommandError` maps path-bearing variants to fixed category-level messages
- [x] 3.3 `CommandError::redacted` redacts verbatim-forwarded lower-layer messages (`CliError::Internal`, `SdkError::Package`, `SessionsError::Repository`, `McpError::Database`, `ApplicationError::Internal`, …) at the `From` boundary, leaving structured codes untouched

## 4. Batch repository reads

- [x] 4.1 `load_code_candidates` uses one `WHERE source_id IN (...)` query; the `?1`-bound `workspace_id`/`scope_folder` columns get separate parameters
- [x] 4.2 `list_run_views` bulk-loads iterations and evidence in two queries; `load_evidence` per-iteration retained for `find_run_view`
- [x] 4.3 `workspace_statuses` runs one query with correlated COUNTs + a `row_number()` window for the latest failure; `list_code_index_workspaces` uses `list_workspaces_with_status`
- [x] 4.4 `reconcile_apply` applies the whole upsert + orphan-delete diff in one transaction with prepared statements
- [x] 4.5 Migration 54 adds `idx_loop_evidence(iteration_id, created_at)`

## 5. Move blocking work off the Tauri main thread / async executor

- [x] 5.1 `get_session_git_status`, `get_session_git_diff`, `list_session_logs`, `export_session_logs`, `list_session_directory` are `async fn` delegating to `WorkspaceApi::*_blocking` (`spawn_blocking`)
- [x] 5.2 `list_connectors` snapshots DB + credentials on `spawn_blocking`, awaits transport health, then assembles
- [x] 5.3 `get_session_git_diff` uses `git ls-files --error-unmatch -- <path>` instead of a full `git status` walk

## 6. Frontend streaming + runtime-adapter hardening

- [x] 6.1 `applyChatEvents` applies a batch in one traversal; `useSessionMessageEvents` and `use-main-layout-model` buffer on `requestAnimationFrame`
- [x] 6.2 `shell-tab` coalesces PTY output chunks into one `terminal.write` per frame
- [x] 6.3 `createRuntimeAdapter` throws when `web-http` is selected but no `webHttp` adapter is provided

## 7. Small fixes

- [x] 7.1 `load_code_candidates` casts `i64` line numbers through `u32::try_from` instead of `as u32`
- [x] 7.2 `floating-assistant-app` drops the dead `subscribeEvents(() => undefined)` subscription
- [x] 7.3 retention `unwrap_or_default()` on a NULL julianday diff treats it as long-overdue, not 0 days

## 8. Verification

- [x] 8.1 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (1873 lib + architecture 17 pass; flaky MCP relay/socket timing tests pass isolated)
- [x] 8.2 `npm run lint:ci`, `tsc --noEmit`, `npm run build`
- [x] 8.3 `openspec validate harden-runtime-concurrency-and-query-efficiency --strict` and `openspec validate --specs --strict`
