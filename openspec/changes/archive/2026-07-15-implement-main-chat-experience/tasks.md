## 1. Phase 1 Input Foundation

- [x] 1.1 Add chat input types and configuration state types.
- [x] 1.2 Add `ChatInputBox` as the input area container.
- [x] 1.3 Add `ChatTextArea` with auto-resize, clear action, Shift+Enter newline, and IME-safe Enter submit.
- [x] 1.4 Add `ButtonArea` with selector controls and action buttons.
- [x] 1.5 Add shared `SelectorDropdown` with option, divider, and toggle rows.
- [x] 1.6 Add config, provider, mode, model, and reasoning selectors.
- [x] 1.7 Add static model catalog seed data for Phase 1 selector linkage.
- [x] 1.8 Add `useChatConfig` linkage rules for provider, agent, model, mode, reasoning, and long-context state.
- [x] 1.9 Replace the hardcoded main-layout input area with the chat input component.

## 2. Phase 2 Frontend Message Experience

- [x] 2.1 Add or update TypeScript types for `ChatMessage`, `MessageRole`, `MessageStatus`, `ChatStreamEvent`, `ToolUseBlock`, and `TokenUsage`.
- [x] 2.2 Extend `AgentService` with `sendMessage`, `listMessages`, `stopGeneration`, and `subscribeMessageEvents`.
- [x] 2.3 Implement Web/mock message storage and timer-based mock streaming in `web-agent-client.ts`.
- [x] 2.4 Add Tauri adapter stubs or typed method shells that preserve the same service contract before desktop commands are wired.
- [x] 2.5 Add `MessageList` with empty state, chronological rendering, internal scrolling, and load-earlier entry point.
- [x] 2.6 Add `MessageItem` with user, assistant, system, tool, streaming, failed, and cancelled rendering states.
- [x] 2.7 Add `ThinkingBlock` for collapsible thinking content.
- [x] 2.8 Add `ToolUseBlock` for collapsible tool-use data.
- [x] 2.9 Add `WelcomeScreen` for empty active sessions.
- [x] 2.10 Add `WaitingIndicator` for submitted messages before the first assistant content arrives.
- [x] 2.11 Add `ScrollControl` and auto-scroll behavior that pauses when the user scrolls away from the bottom.
- [x] 2.12 Wire `ChatInputBox` submit and stop states to the message service and stream reducer.
- [x] 2.13 Ensure active-session switching reloads messages and unsubscribes from the previous session stream.
- [x] 2.14 Add focused Playwright coverage for empty state, send, mock streaming, stop, and session message switching.

## 3. Phase 3 Desktop Persistence And Runtime

- [x] 3.1 Add a SQLite migration for the `messages` table with session foreign key cascade and status fields.
- [x] 3.2 Add Rust message model types matching the frontend message contract.
- [x] 3.3 Add Rust repository functions for creating, updating, listing, and deleting session-owned messages.
- [x] 3.4 Add `send_message` Tauri command that persists the user message, creates an assistant message, and starts generation.
- [x] 3.5 Add `list_messages` Tauri command with `limit` and `before_id` pagination.
- [x] 3.6 Add `stop_generation` Tauri command with soft-cancel-first semantics and process-kill fallback.
- [x] 3.7 Register the new Tauri commands in the app command handler.
- [x] 3.8 Implement `tauri-agent-client.ts` message methods with Tauri `invoke()` inside the adapter only.
- [x] 3.9 Implement `subscribeMessageEvents` in `tauri-agent-client.ts` with a single `chat:event` listener.
- [x] 3.10 Add an Agent runtime manager that tracks active generation by session id.
- [x] 3.11 Add Agent output parser trait or equivalent abstraction.
- [x] 3.12 Implement the first Claude Code output parser.
- [x] 3.13 Implement a generic line-based parser fallback for other agents.
- [x] 3.14 Emit `started`, `token`, `thinking`, `tool_use`, `completed`, `failed`, and `cancelled` stream events where supported.
- [x] 3.15 Persist assistant message status and content updates during generation, failure, and cancellation.
- [x] 3.16 Handle stderr, non-zero exit codes, cancellation, and process cleanup without panics.
- [x] 3.17 Add Rust tests for message repository behavior and output parser behavior.

## 4. Verification

- [x] 4.1 Run `openspec validate "implement-main-chat-experience" --strict`.
- [x] 4.2 Run `npm run build`.
- [x] 4.3 Run `npm run test`.
- [x] 4.4 Run `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; $env:CARGO_NET_OFFLINE="true"; cargo check --manifest-path src-tauri\Cargo.toml`.
- [x] 4.5 Run `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo test --manifest-path src-tauri\Cargo.toml` when Rust implementation is present.
