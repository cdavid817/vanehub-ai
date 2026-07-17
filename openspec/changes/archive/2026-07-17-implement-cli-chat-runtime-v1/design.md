## Context

The React chat surface already calls the `AgentService` boundary for sending, listing, stopping, and subscribing to messages. The Tauri adapter exposes those calls through Rust commands, and the Web adapter mirrors the contract with in-memory mock streaming.

The desktop Rust implementation has message persistence, lifecycle transitions, `ChatRuntimeManager`, and event types, but the current generation path is generic and synchronous: it executes one command, waits for the full output, then parses and emits events. That prevents true streaming, weakens cancellation, and does not respect each Agent CLI's headless protocol.

`clowder-ai` shows the useful pattern for this project: provider-specific command builders and event transformers behind one normalized message stream. Its tmux PTY path is not a first-version fit for native Windows/Tauri, so this design uses headless CLI invocations with resume metadata where available.

## Goals / Non-Goals

**Goals:**

- Run chat messages for CLI sessions through the selected local Agent CLI in the Tauri desktop runtime.
- Stream stdout incrementally into normalized chat events while persisting assistant content and terminal status.
- Keep cancellation session-scoped by storing the running child process in `ChatRuntimeManager`.
- Build provider-specific command specs for Claude Code, Codex CLI, Gemini CLI, and OpenCode while preserving one service contract.
- Prefer prompt delivery through stdin when the CLI supports it to avoid leaking chat content through process argv.
- Persist provider runtime session ids when a CLI reports them so later turns can resume the native CLI conversation.
- Keep the current Web/mock adapter contract intact.
- Document first-version constraints and the path toward richer PTY/ConPTY support.

**Non-Goals:**

- No long-lived native Windows interactive TUI process in v1.
- No tmux dependency in VaneHub.
- No new frontend state management library or UI component library.
- No direct Tauri `invoke()` from React components.
- No replacement of the existing chat UI style; both existing visual themes remain token-driven.

## Decisions

### Use headless CLI invocations for v1

V1 will launch one child process per sent message and use provider resume/session metadata when available.

Alternatives considered:
- Long-lived PTY/TUI process: closer to a terminal, but native Windows requires ConPTY and adds substantial lifecycle complexity.
- Generic `command arg prompt`: simple, but fails provider protocols and leaks prompt content into argv.

Rationale: headless CLI execution is testable, cross-platform enough for the first desktop release, and aligns with existing Tauri command boundaries.

### Introduce provider-specific runtime helpers in Rust

The Rust layer will add focused helpers for:
- resolving the executable from cached CLI status or PATH,
- building provider command arguments,
- selecting stdin vs argv prompt delivery,
- parsing stdout lines into `ChatStreamEvent` updates,
- extracting provider session ids from structured events.

Alternatives considered:
- Keep all behavior in `send_message`: faster initially, but makes future provider fixes risky.
- Port `clowder-ai` TypeScript services directly: mismatched runtime and dependency stack.

Rationale: VaneHub's native integration belongs in Rust, but the adapter pattern from `clowder-ai` should guide the structure.

### Stream from background execution

`send_message` will create persisted user and assistant messages, mark the session `starting`, register the active generation, then spawn background work that reads stdout/stderr and emits events. The command may return the assistant placeholder immediately while events continue updating the UI.

Alternatives considered:
- Keep `send_message` blocking until completion: simpler, but not true streaming and weak for cancellation.

Rationale: The frontend already subscribes to `chat:event`; the backend should use that contract as intended.

### Keep stderr diagnostic-only

stderr output will be written through unified logging with redaction and summarized in failed message errors only when needed. Raw stderr will not be appended into assistant chat content.

Alternatives considered:
- Show stderr inline as assistant text: easier to debug, but poor UX and riskier for secrets.

Rationale: Chat content should remain assistant-facing; diagnostics belong in unified logs.

### Persist runtime session metadata additively

V1 may add nullable session runtime metadata columns, such as `runtime_session_id`, when provider events expose a native session id. Existing sessions remain valid with null metadata.

Alternatives considered:
- Reconstruct context by replaying VaneHub messages into every prompt: provider-agnostic but token-expensive and inconsistent with CLI-native state.

Rationale: CLI-native resume produces behavior closest to the underlying coding agent.

## Risks / Trade-offs

- Provider CLI event formats may drift -> keep parser tests focused on representative event shapes and fail with concise user-facing errors.
- Some CLIs may not expose stable session ids in all modes -> allow null metadata and continue as a non-resumed new turn with clear logging.
- Background generation can outlive UI navigation -> keep runtime ownership by session id and emit events only with the owning session id.
- Killing a child process may leave provider-side partial state -> mark the assistant message `cancelled`, preserve captured content, and write diagnostics.
- Prompt-in-argv may still be required for some CLI modes -> prefer stdin where supported and document any exception in command builder tests.

## Migration Plan

1. Add runtime metadata schema fields with nullable defaults.
2. Add provider command builder/parser helpers and tests.
3. Replace the synchronous generic execution path with background streaming execution.
4. Keep Web/mock behavior unchanged.
5. Validate OpenSpec, Rust tests/checks, frontend tests/build, and targeted Playwright chat smoke behavior where practical.

Rollback is straightforward: keep the existing Tauri command names and frontend contract; revert the runtime helper usage back to the previous synchronous path if needed.

## Open Questions

- Exact provider session id fields should be verified against installed CLI versions during implementation and covered by fixture-based tests.
- Native Windows ConPTY support remains a future improvement for true interactive TUI sessions.
- Richer provider-specific tool event rendering can be expanded after the first normalized streaming path is stable.
