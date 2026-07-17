# CLI Chat Runtime V1

## Scope

The first desktop CLI chat runtime runs one headless Agent CLI process per submitted message. It is session-scoped, streams stdout into the existing chat event model, persists assistant content incrementally, and cancels by terminating the child process owned by the VaneHub session id.

This version is intentionally not a terminal emulator and does not host a long-lived interactive TUI session.

## Runtime Boundary

- React components call `src/services/agent-service.ts`.
- `src/services/tauri-agent-client.ts` owns Tauri `invoke()` and event subscription details.
- `src/services/web-agent-client.ts` keeps an in-memory mock implementation with the same contract.
- `src-tauri/src/lib.rs` owns SQLite persistence, CLI process launch, stdout/stderr parsing, cancellation, and unified logging.

React components must not call Tauri commands directly or write local diagnostic files.

## Provider Command Assumptions

V1 uses provider-specific headless command builders:

- `claude-code`: `claude -p --output-format stream-json --include-partial-messages --verbose`
- `codex-cli`: `codex exec --json -- -`
- `gemini-cli`: `gemini -p <prompt> -o stream-json -y`
- `opencode`: `opencode run --format json <prompt>`

When a provider runtime session id is available, the runtime passes it through the provider's resume path:

- Claude: `--resume <session-id>`
- Codex: `exec resume <session-id>`
- Gemini: `--resume <session-id>`
- OpenCode: `--session <session-id>`

Prompt delivery prefers stdin where the CLI supports it. Claude and Codex use stdin in v1. Gemini and OpenCode keep their prompt argument shape because that is the compatible headless contract used for this first implementation.

## Streaming And Persistence

The Tauri `send_message` command persists the user message and assistant placeholder, marks the session as starting/running, starts the provider process, and returns the assistant placeholder. A background worker reads stdout line by line, normalizes provider events, updates the assistant message, and emits `chat:event` updates.

Terminal outcomes update both the assistant message and owning session lifecycle:

- success -> assistant `completed`, session `idle`
- provider error or non-zero exit -> assistant `failed`, session `failed`
- stop/archive/delete -> assistant `cancelled`, session `stopped` or removed

## Logging Rules

CLI chat runtime diagnostics must go through unified logging. Command audits redact prompt content. stderr and failure details are diagnostic data and should not be appended to assistant chat content. The chat UI should receive concise user-facing errors while detailed context is written to the configured log directory with redaction.

## Known Constraints

- Streaming quality depends on each provider CLI's event format.
- Some providers may not emit a native session id in every mode; VaneHub continues without resume metadata when it is unavailable.
- V1 does not implement native Windows ConPTY or a persistent TUI session.
- Gemini and OpenCode prompt arguments remain visible to the child process command line in v1 because their compatible headless contracts are argument based.

## Future Improvements

- Add a native Windows ConPTY driver for true interactive TUI sessions.
- Move provider runtime helpers out of the large Tauri lib module into dedicated runtime modules.
- Add richer provider-specific event transformers for tool calls, token usage, and diagnostics.
- Persist structured provider runtime metadata instead of only a single resume id.
- Add integration fixtures for installed CLI versions used by release testing.
