## Why

Session records, sidebar actions, and message persistence exist, but sessions are not yet complete as runtime-backed work containers. Desktop chat still returns preview responses, lifecycle state can drift from generation state, and a few Web/UI details diverge from the session specification.

Completing session management now gives each session a reliable contract across metadata, messages, active runtime state, cancellation, and restart-safe persistence before deeper Agent integrations build on top of it.

## What Changes

- Align existing session behavior with the current specification, including Web default session title parity and removal of hard-coded session placeholders in the main layout info panel.
- Treat active session selection as the source for active Agent, interaction mode, lifecycle display, message listing, and generation controls.
- Replace the sidebar one-click new-session action with a create-session dialog that collects Agent, project folder, history selection, and optional Git worktree settings.
- Let users choose an Agent from Claude Code, Gemini CLI, Codex, and OpenCode when creating a session.
- Persist recently used project paths, detect Git repositories, and optionally create a sibling Git worktree before creating the session.
- Introduce a runtime session capability that binds desktop generations to a session-scoped Agent runtime state.
- Replace desktop preview responses with real CLI-backed Agent execution for supported, installed CLI Agents; unavailable or unsupported runtimes must fail explicitly rather than returning mock success.
- Persist and expose session runtime status transitions for starting, running, completed, failed, stopped, and cancelled generations.
- Route Agent runtime output, stderr/error diagnostics, and cancellation results through the existing chat message and unified log boundaries.
- Route Git inspection and worktree diagnostics through the unified logging service while showing concise UI errors.
- Keep Web runtime behavior aligned with the same service contract through deterministic mock runtime sessions.
- Preserve the React service boundary: components continue to use `AgentService`, the Tauri adapter owns `invoke()` and event listening, and Rust owns SQLite/process/runtime work.

## Capabilities

### New Capabilities
- `session-runtime-management`: Defines how a durable session owns active generation state, runtime lifecycle transitions, cancellation, and Agent runtime diagnostics.
- `project-worktree-management`: Manages known project paths, Git repository detection, and optional Git worktree creation for session startup.

### Modified Capabilities
- `session-management`: Tighten active-session, default-title, lifecycle, project/worktree metadata, create-session input, and UI metadata behavior so session records stay coherent across desktop and Web runtimes.
- `chat-experience`: Extend chat persistence and streaming requirements so messages are produced by session-scoped runtime execution rather than desktop preview responses.
- `unified-log-management`: Require session-scoped Agent runtime diagnostics and command output to be written through the unified logging service with redaction.
- `main-layout-ui`: The sidebar new-session action opens a create-session dialog with project, Agent, folder, and worktree controls.
- `native-runtime-architecture`: The native runtime gains guarded Git inspection and worktree creation commands behind the service boundary.

## Impact

- Frontend service contract: may add session runtime/detail fields or methods only behind `src/services/agent-service.ts`.
- Frontend service contract: extend `AgentService` with known-project listing, project inspection, directory selection reuse, and richer `createSession` input/output fields.
- Frontend adapters: update both `src/services/tauri-agent-client.ts` and `src/services/web-agent-client.ts` for parity.
- React UI: update `src/main-layout/main-layout.tsx`, create-session dialog UI, and chat components to consume active session/runtime state without direct Tauri access.
- Rust/Tauri: add or refactor commands/runtime helpers in `src-tauri/src/`, persist runtime and project/worktree state in SQLite, connect generation/cancellation to session ownership, and run guarded Git commands.
- SQLite: additive migrations only; existing sessions and messages remain valid.
- Logging: desktop runtime diagnostics must use the unified log service rather than feature-local files.
