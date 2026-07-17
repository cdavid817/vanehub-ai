## 1. Runtime Schema And Documentation

- [x] 1.1 Add nullable session runtime metadata storage for provider resume ids without breaking existing sessions.
- [x] 1.2 Add a technical documentation section that records v1 headless CLI constraints, provider command assumptions, logging rules, and future PTY/ConPTY improvements.

## 2. Provider CLI Runtime

- [x] 2.1 Add Rust helpers for provider-specific CLI command construction for `claude-code`, `codex-cli`, `gemini-cli`, and `opencode`.
- [x] 2.2 Prefer stdin prompt delivery for providers that support it and keep prompt content redacted from command audits.
- [x] 2.3 Add stdout line parsers that normalize provider events into existing chat stream event types.
- [x] 2.4 Capture provider runtime session ids when available and reuse them through provider-specific resume arguments.

## 3. Streaming Execution And Cancellation

- [x] 3.1 Replace the blocking desktop `send_message` generation path with background streaming execution that returns the assistant placeholder promptly.
- [x] 3.2 Persist assistant content incrementally as token events stream in.
- [x] 3.3 Store active child process handles in `ChatRuntimeManager` by session id and terminate the owned process on stop, archive, or delete.
- [x] 3.4 Keep session lifecycle synchronized across starting, running, completed, failed, and cancelled outcomes.
- [x] 3.5 Route stdout/stderr diagnostics through unified logging with redaction and concise chat-facing errors.

## 4. Frontend Contract And I18n

- [x] 4.1 Confirm the existing React chat surface works with promptly returned assistant placeholders and background events.
- [x] 4.2 Add or adjust localized zh-CN and en text only where new runtime status or error labels are user-visible.
- [x] 4.3 Preserve both existing visual themes without introducing direct style branches or new UI libraries.

## 5. Verification

- [x] 5.1 Add Rust tests for command builders, parsers, provider session metadata, and cancellation lifecycle behavior.
- [x] 5.2 Run `openspec validate "implement-cli-chat-runtime-v1" --strict`.
- [x] 5.3 Run `npm run test`.
- [x] 5.4 Run `npm run build`.
- [x] 5.5 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 5.6 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
