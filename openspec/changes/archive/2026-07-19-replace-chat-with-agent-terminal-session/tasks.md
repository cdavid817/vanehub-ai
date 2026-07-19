## 1. Native Agent Terminal Domain and Ports

- [x] 1.1 Add Agent Terminal application models for terminal id, session id, state, capability, dimensions, runtime session id, and terminal events.
- [x] 1.2 Add `agent_runtime` application ports for terminal process start/attach/input/resize/stop, CLI profile loading, session runtime id persistence, clock, events, and logging.
- [x] 1.3 Add application use cases for open-or-attach terminal, write input, resize terminal, stop terminal, idle cleanup, and shutdown cleanup.
- [x] 1.4 Add application tests for fresh start, attach existing process, archived-session rejection, one-live-terminal-per-session, idle timeout, shutdown cleanup, and failure lifecycle mapping.

## 2. Native Provider Invocation and Shell Wrappers

- [x] 2.1 Add provider-specific interactive invocation builders for `claude-code`, `codex-cli`, `gemini-cli`, and `opencode`.
- [x] 2.2 Add provider-specific resume grammar using persisted `runtimeSessionId` where supported.
- [x] 2.3 Add `CliParameterLaunchScope::Interactive` profile loading for Agent Terminal process snapshots.
- [x] 2.4 Implement native wrapper generation for Windows PowerShell, Windows cmd fallback, and macOS/Linux default shell execution.
- [x] 2.5 Add tests for wrapper quoting, paths with spaces, command redaction, profile token placement, fresh invocation, and resume invocation fixtures.

## 3. Native Process Registry and Events

- [x] 3.1 Implement an Agent Terminal process registry keyed by session id with retained process handles and last-active timestamps.
- [x] 3.2 Implement PTY-backed stdout/stderr/input/resize event streaming for Agent Terminal processes.
- [x] 3.3 Parse provider runtime session ids from supported terminal output or startup metadata and persist them through the sessions gateway.
- [x] 3.4 Ensure terminal stdout/stderr is streamed to the UI but not inserted into `messages` rows.
- [x] 3.5 Write startup, attach, exit, idle cleanup, shutdown cleanup, and failure diagnostics through unified logging with redaction.

## 4. Tauri Commands and Bootstrap

- [x] 4.1 Add Tauri commands for Agent Terminal open-or-attach, input, resize, stop, and event subscription routing.
- [x] 4.2 Register Agent Terminal commands in the command registry and bootstrap the assembled `agent_runtime` terminal service.
- [x] 4.3 Hook desktop shutdown lifecycle into Agent Terminal shutdown cleanup.
- [x] 4.4 Add command contract tests for DTO serialization, command-safe errors, and frontend adapter compatibility.

## 5. Frontend Service and Runtime Adapters

- [x] 5.1 Extend `AgentService` and frontend types with Agent Terminal session, input, resize, stop, and event subscription contracts.
- [x] 5.2 Implement Tauri adapter methods that call the new native commands and listen for terminal events.
- [x] 5.3 Implement Web/mock adapter methods with deterministic simulated terminal behavior and mock runtime session id updates.
- [x] 5.4 Add service adapter tests for Tauri payload mapping and Web/mock parity.

## 6. Workspace UI

- [x] 6.1 Replace the main Chat panel with an `AgentTerminalTab` using xterm and the Agent Terminal service contract.
- [x] 6.2 Automatically open or attach the Agent Terminal after session creation and when selecting an active single-Agent CLI session.
- [x] 6.3 Remove session-page model, provider, permission, reasoning, thinking, streaming, and prompt-composer controls from the Agent Terminal surface.
- [x] 6.4 Keep the ordinary Shell tab available and verify it still uses the existing project shell service without Agent CLI parameter injection.
- [x] 6.5 Add localized terminal state, error, disabled, and simulated Web/mock text in `zh-CN` and `en`.
- [x] 6.6 Add UI tests for terminal rendering, automatic startup trigger, Shell tab retention, localized labels, and absence of removed chat controls.

## 7. Create Session Flow

- [x] 7.1 Add Single Agent and disabled Multi Agent mode selection to the create-session dialog.
- [x] 7.2 Ensure Single Agent session creation uses the agent selected in the create-session dialog as the session's stable agent id.
- [x] 7.3 Prevent Multi Agent submission and show localized coming-soon or disabled state.
- [x] 7.4 Add focused tests for mode selection, disabled Multi Agent behavior, and created session Agent identity.

## 8. Verification

- [x] 8.1 Run `npm run lint`.
- [x] 8.2 Run `npm run test`.
- [x] 8.3 Run `npm run build`.
- [x] 8.4 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 8.5 Run `cargo clippy --manifest-path src-tauri/Cargo.toml`.
- [x] 8.6 Run `openspec validate replace-chat-with-agent-terminal-session --strict`.
- [x] 8.7 Run `openspec validate --specs --strict`.
