## 1. Background command registry

- [x] 1.1 Add a session-scoped background command registry in the agent runtime's tool infrastructure with handle allocation, per-session concurrency limit, and lookup that rejects unknown and foreign handles.
- [x] 1.2 Spawn background commands through `ManagedChild::spawn_in` so Windows job-object and Unix process-group containment are inherited rather than reimplemented.
- [x] 1.3 Drain stdout and stderr into a bounded rolling buffer that discards oldest bytes on overflow and records that a drop occurred.
- [x] 1.4 Track lifecycle status (running, exited, killed, lifetime-exceeded), exit code, and a per-command retrieval cursor.
- [x] 1.5 Enforce the maximum background lifetime by terminating the process tree and recording a lifetime-exceeded status.
- [x] 1.6 Reap a session's background commands when the session ends and terminate remaining trees on desktop runtime shutdown.

## 2. Tool surface

- [x] 2.1 Extend the `shell` tool schema with a clamped `timeout_ms` and a `run_in_background` flag, keeping the current foreground default when both are absent.
- [x] 2.2 Add `shell_output` and `shell_kill` tool definitions to the baseline catalog, and add `shell_output` (read-only) but not `shell_kill` to the plan-mode catalog.
- [x] 2.3 Route the two new tool names through the tool-call executor with workspace and session context.
- [x] 2.4 Classify the new tool names in the permission mapping: background start as shell execution, retrieval and termination as no-approval operations.

## 3. Tests

- [x] 3.1 Registry unit tests for concurrency limit, buffer overflow reporting, cursor advancement, lifetime expiry, and foreign/unknown handle rejection.
- [x] 3.2 Tool schema tests pinning the full argument surface of `shell`, `shell_output`, and `shell_kill` in both catalogs.
- [x] 3.3 Executor tests routing the new tool names and rejecting them without a workspace folder.
- [x] 3.4 Permission classification tests for background start, retrieval, and termination.
- [x] 3.5 An end-to-end test that starts a real background command, polls it to completion, and asserts the exit code and process-tree cleanup.

## 4. Validation

- [x] 4.1 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 4.2 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 4.3 `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 4.4 `openspec validate add-background-shell-execution --strict`
