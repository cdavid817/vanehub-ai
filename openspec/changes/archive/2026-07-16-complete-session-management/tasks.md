## 1. Existing Session Parity

- [x] 1.1 Change Web adapter default session title to "新会话" and update service tests.
- [x] 1.2 Replace hard-coded session configuration placeholders in the main layout with active session or service-backed runtime details.
- [x] 1.3 Ensure session lists refresh after message send, completion, failure, cancellation, archive, restore, and delete operations.

## 2. Frontend Runtime Contract

- [x] 2.1 Review `AgentService` for any required session runtime detail fields or methods and update both adapters consistently.
- [x] 2.2 Keep send and stop controls scoped to the active session in the first-version UI.
- [x] 2.3 Display concise user-facing runtime errors in chat messages without exposing raw stdout or stderr.
- [x] 2.4 Add or update frontend tests for session lifecycle transitions and active-session message ownership.

## 3. Web Mock Runtime

- [x] 3.1 Update Web mock message generation to transition session lifecycle through running and terminal states consistently.
- [x] 3.2 Update Web mock cancellation so stop marks the active assistant message cancelled and the session stopped.
- [x] 3.3 Ensure Web mock archive/delete of a running session clears timers, active stream state, messages, and active session selection correctly.

## 4. Desktop Session Runtime

- [x] 4.1 Add a Rust session runtime abstraction keyed by `session_id` for active generation handles.
- [x] 4.2 Implement a generic CLI adapter that executes a real installed CLI when supported launch metadata is available.
- [x] 4.3 Return structured unsupported/unavailable failures when the selected Agent CLI cannot be executed.
- [x] 4.4 Route desktop `send_message` through the session runtime instead of returning a hard-coded preview response.
- [x] 4.5 Update session lifecycle and updated timestamp when generation starts, runs, completes, fails, or is cancelled.
- [x] 4.6 Keep assistant message status synchronized with runtime completion, failure, and cancellation.
- [x] 4.7 Stop a running generation before archiving or deleting its session, then clear active session selection when applicable.

## 5. Unified Logging

- [x] 5.1 Persist session runtime stdout, stderr, command metadata, exit status, unsupported runtime details, and cancellation diagnostics through the unified logging service.
- [x] 5.2 Include session id and Agent id context in session runtime log entries.
- [x] 5.3 Verify runtime diagnostics are redacted before disk persistence and are not rendered raw in React components.

## 6. Verification

- [x] 6.1 Run `openspec validate "complete-session-management" --strict`.
- [x] 6.2 Run `openspec validate --specs --strict`.
- [x] 6.3 Run `npm run test`.
- [x] 6.4 Run `npm run build`.
- [x] 6.5 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 6.6 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 6.7 Manually verify Web/mock create, send, stop, archive-running, restore, delete, and concise error behavior.
- [x] 6.8 Manually verify desktop unsupported/unavailable CLI sends fail explicitly without preview success.

## 7. Project and Worktree Session Creation

- [x] 7.1 Extend `Session` and create-session input types with project path, effective folder, worktree path, worktree name, and worktree branch metadata.
- [x] 7.2 Add service methods for known project listing, project inspection, directory selection, and dialog-based session creation input.
- [x] 7.3 Ensure agent selection uses stable agent ids for Claude Code, Gemini CLI, Codex, and OpenCode.
- [x] 7.4 Implement in-memory Web/mock known project history, Git inspection, and worktree-aware session creation.
- [x] 7.5 Add SQLite migrations for known project history and optional session project/worktree metadata.
- [x] 7.6 Implement Rust project path canonicalization, Git inspection, worktree validation, and sibling worktree creation.
- [x] 7.7 Route Git failures and command output to unified logs while returning concise UI errors.
- [x] 7.8 Add Tauri adapter support for known projects, project inspection, native directory picker, and richer create-session input.
- [x] 7.9 Replace sidebar immediate new-session creation with a create-session dialog.
- [x] 7.10 Add dialog controls for Agent selection, known project history, directory browsing, selected project status, optional title, and optional Git worktree settings.
- [x] 7.11 Add Web adapter and Rust tests for project/worktree session creation behavior.
- [x] 7.12 Run OpenSpec, frontend, and Rust verification for the merged implementation.
