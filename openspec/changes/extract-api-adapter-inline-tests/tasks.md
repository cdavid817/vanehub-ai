## 1. Capture the baseline

- [ ] 1.1 Record the current physical line count of `api_process_adapter.rs` and the aggregate for `src-tauri/src/contexts/agent_runtime/infrastructure/`
- [ ] 1.2 Capture the sorted list of test names this module reports, via `cargo test --manifest-path src-tauri/Cargo.toml --lib -- --list`, filtered to `api_process_adapter`, and save it for the post-move comparison

## 2. Move the inline test module

- [ ] 2.1 Create `src-tauri/src/contexts/agent_runtime/infrastructure/api_process_adapter/tests.rs` and move the entire body of the inline `mod tests` (from line 6,156 to end of file) into it verbatim, keeping `use super::*;` as its first import
- [ ] 2.2 Replace the inline module in `api_process_adapter.rs` with a single `#[cfg(test)] mod tests;` declaration
- [ ] 2.3 Confirm `cargo check --manifest-path src-tauri/Cargo.toml` compiles with no visibility widening; if any item needs `pub(super)`, record which and why rather than applying it silently

## 3. Move the test-only scaffolding

- [ ] 3.1 Move the `#[cfg(test)]` port doubles and their statics — `NoopWorkspaceMutationPort` with its `AgentWorkspaceMutationPort` impl and `NOOP_WORKSPACE_MUTATIONS`, and `UnavailableSkillReads` with its `AgentSkillPort` impl — into the tests module
- [ ] 3.2 Move the `#[cfg(test)]` constants `MESSAGES_ENDPOINT` and `TEST_SESSION_ID` into the tests module
- [ ] 3.3 Move the four `#[cfg(test)]` helper methods (`execute_tool_call`, `execute_tool_call_with_code_intelligence`, `execute_tool_call_with_workspace_mutations`, `execute_tool_call_with_skills`) into a dedicated `impl RuntimeAgentApiAdapter` block inside the tests module
- [ ] 3.4 Confirm no `#[cfg(test)]` item remains in `api_process_adapter.rs`

## 4. Prove the move was pure

- [ ] 4.1 Re-capture the sorted test-name list and assert it is byte-identical to the baseline from 1.2 — not merely the same length
- [ ] 4.2 Confirm `git diff` on the production half of `api_process_adapter.rs` contains only deletions of `#[cfg(test)]` items and the one added `mod tests;` line, with no change to any production item
- [ ] 4.3 `cargo test --manifest-path src-tauri/Cargo.toml` passes with an unchanged total test count

## 5. Update budgets and verify

- [ ] 5.1 Lower the `api_process_adapter.rs` path budget in `src-tauri/tests/architecture.rs` to the measured post-move count
- [ ] 5.2 Measure the `agent_runtime/infrastructure` subtree delta; if it rose, raise the subtree budget by exactly that amount and state the module-boilerplate reason in the same commit — if the delta exceeds what boilerplate explains, stop and find the cause
- [ ] 5.3 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, and `cargo check --manifest-path src-tauri/Cargo.toml` pass
- [ ] 5.4 `npm run architecture:check` passes, including the composition-root rule that reads this file by name at `architecture.rs:1283`
- [ ] 5.5 Record the `cargo check` wall-clock time before and after the move as the stated benefit, on an otherwise idle machine
- [ ] 5.6 `openspec validate extract-api-adapter-inline-tests --strict` and `openspec validate --specs --strict` pass
