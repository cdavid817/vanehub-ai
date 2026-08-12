## 1. Breaking configuration contract

- [x] 1.1 Replace frontend `PermissionMode` and `permissionMode` with `SessionExecutionMode` and `executionMode`, accepting only `inherit`, `plan`, and `execute`
- [x] 1.2 Replace Rust DTO, application, domain, and serialized chat-configuration fields with `execution_mode` and reject removed values
- [x] 1.3 Add a database migration that clears legacy `sessions.chat_preferences` snapshots and policy-governed saved CLI selections

## 2. Effective execution policy

- [x] 2.1 Add provider-neutral effective-policy types and the complete Agent-template/session-mode resolution matrix
- [x] 2.2 Resolve Agent policy for native OnePiece generations and enforce plan-mode narrowing without bypassing the existing permission pipeline
- [x] 2.3 Resolve Agent policy for every managed CLI chat and Agent Terminal launch, failing closed on lookup or mapping failure
- [x] 2.4 Replace separate session and template mutations with final provider mappings for Claude Code, Codex CLI, Gemini CLI, OpenCode, and Antigravity CLI

## 3. CLI parameter ownership

- [x] 3.1 Remove execution, approval, automatic-approval, and sandbox controls from the backend editable CLI parameter catalogs
- [x] 3.2 Update frontend catalog definitions, Web/mock profiles, validation, and previews to match the reduced editable parameter set
- [x] 3.3 Ensure persisted/message ordinary parameters cannot override runtime-owned effective-policy selections

## 4. Service and UI

- [x] 4.1 Extend the Tauri and Web/mock `AgentService` contracts with Agent policy and resolved effective behavior for session chat configuration
- [x] 4.2 Replace the permission selector with an execution-mode selector for Inherit, Plan, and Execute
- [x] 4.3 Display the Agent policy and effective behavior, including read-only narrowing and next-launch policy-change guidance
- [x] 4.4 Update Simplified Chinese, Traditional Chinese, English, Japanese, and Korean localization resources

## 5. Regression coverage

- [x] 5.1 Add table-driven Rust tests for every policy/session combination and all managed CLI chat and terminal mappings
- [x] 5.2 Add Rust tests for native OnePiece enforcement, legacy snapshot reset, removed fields, and fail-closed lookup/mapping errors
- [x] 5.3 Update TypeScript service, adapter, configuration, component, localization, and contract tests for the breaking field and effective behavior
- [x] 5.4 Update Playwright coverage for the execution selector, effective-policy hint, and read-only safety ceiling

## 6. Verification

- [x] 6.1 Run `openspec validate unify-agent-session-execution-policy --strict` and `openspec validate --specs --strict`
- [x] 6.2 Run `npm run lint:ci`, `npm run test`, `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, `npm run contracts:check`, and `npm run build`
- [x] 6.3 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`
- [x] 6.4 Run `npx playwright test` and record the implementation verification result
