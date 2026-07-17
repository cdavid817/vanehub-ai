## Why

The main chat surface already exposes sessions, messages, streaming events, and cancellation controls, but the desktop runtime still runs Agent CLIs through a generic synchronous command path. Users need a first real session chat path where messages sent from a created CLI session execute against the selected local CLI and stream back into the existing chat UI.

## What Changes

- Add a first-version desktop CLI chat runtime that runs the selected session Agent through a provider-specific headless CLI command path.
- Stream stdout events from the running CLI process into normalized `chat:event` updates instead of waiting for full command completion before updating the UI.
- Persist user messages, assistant content, terminal message status, and session lifecycle transitions through the existing SQLite-backed Rust layer.
- Track active child processes by session so stop, archive, and delete can cancel the correct generation.
- Preserve the frontend service boundary: React continues to use `agent-service.ts`; Tauri invocation remains inside `tauri-agent-client.ts`.
- Keep Web/mock behavior compatible with the same message contract without requiring local CLI access.
- Add technical documentation that records first-version constraints and later improvement paths, including native Windows PTY/ConPTY and richer provider session metadata.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `chat-experience`: Desktop chat generation shall stream from real provider-specific CLI execution rather than a generic synchronous command output path.
- `session-runtime-management`: Session runtime ownership shall retain cancellable child process handles and provider runtime metadata needed for CLI resume where available.
- `unified-log-management`: CLI chat runtime diagnostics shall persist through the unified logging service with redaction before disk writes.

## Impact

- Desktop runtime: Rust/Tauri chat runtime, process spawning, stdout/stderr parsing, cancellation, SQLite message/session updates, and unified logs.
- Web runtime: Contract parity only; mock adapter remains in-memory and does not launch local CLIs.
- Frontend: Small status/i18n adjustments only if needed; no direct Tauri calls from components.
- Testing: Rust unit tests for command builders/parsers/runtime lifecycle, frontend tests for chat event application where needed, and project validation commands after implementation.
