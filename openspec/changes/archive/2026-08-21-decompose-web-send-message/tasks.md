## 1. Characterize the existing contract

- [x] 1.1 Add focused fake-timer coverage for admission failures and the synchronous run, session, and message state established before `sendMessage` resolves
- [x] 1.2 Record and assert the observable event ordering, completion transition, and cancellation behavior without exposing scheduler internals
- [x] 1.3 Run the focused Web adapter tests and confirm the characterization passes before moving implementation

## 2. Extract deterministic scheduling

- [x] 2.1 Define the immutable turn/scheduling context from values already computed synchronously by `sendMessage`
- [x] 2.2 Extract response, compaction, memory, skill, token, rich-block, and completion scheduling while preserving conditions, payloads, delays, and registration order
- [x] 2.3 Extract API tool, approval, clarification, plan-exit, grep, explicit-memory, and MCP scheduling while preserving conditions, payloads, delays, and registration order
- [x] 2.4 Keep every scheduled timer in the caller-owned active-stream timeout array and pass the focused ordering and cancellation tests

## 3. Move chat orchestration

- [x] 3.1 Move `sendMessage` into `webChatClient: ChatMessagingService` and remove its inline implementation and obsolete imports from `web-agent-client.ts`
- [x] 3.2 Confirm `webAgentClient` retains one `...webChatClient` spread, remains annotated `: AgentService`, and `src/services/tauri-agent-client.ts` is byte-identical
- [x] 3.3 Measure and ratchet the `web-agent-client.ts` and `src/services` budgets without adding an ESLint exemption
- [x] 3.4 Run `tsc --noEmit`, `npm run contracts:check`, and `npm run test`

## 4. Complete verification and reconcile the parent change

- [x] 4.1 Run `npm run lint:ci`, `npm run build`, `npm run architecture:check`, and the WebdriverIO desktop smoke gate with `CI=1 npm run test:desktop:smoke`
  - The stable WDIO smoke gate passed the real Tauri startup, IPC, and navigation path. A diagnostic full-suite run completed 22/26 spec files; four files failed before test execution because the embedded driver could not create a session on `127.0.0.1:4445`, matching the Linux full-suite instability already documented in `tests/desktop/wdio.conf.mjs`.
- [x] 4.2 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `npm run native:panic:check`, and `cargo test --workspace`
- [x] 4.3 Mark task 3.4 in `extract-web-client-state-modules` complete only after the moved implementation and full verification pass
- [x] 4.4 Run `openspec validate decompose-web-send-message --strict`, `openspec validate extract-web-client-state-modules --strict`, and `openspec validate --specs --strict`
