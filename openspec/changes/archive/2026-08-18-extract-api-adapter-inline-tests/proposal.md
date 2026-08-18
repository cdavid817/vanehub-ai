## Why

`src-tauri/src/contexts/agent_runtime/infrastructure/api_process_adapter.rs` is the largest file in the repository at 13,927 physical lines. Its inline `mod tests` starts at line 6,156, so **7,772 lines (56%) of the file are test code**, and every `cargo check` of the library parses and type-checks all of it. A further 19 `#[cfg(test)]` items — a `NoopWorkspaceMutationPort` port double and its static, a `MESSAGES_ENDPOINT` constant, a `TEST_SESSION_ID` constant, an `UnavailableSkillReads` port double, and four `execute_tool_call*` helper methods hung off the production impl — sit scattered through the production half at lines 104-5,870.

The file is also still growing: it gained 602 lines in the two days between the audit that produced the optimization ticket and the branch this change starts from. `freeze-large-file-line-budgets` recorded its ceiling at 13,927 and named this change as the owner, so the budget diagnostic already points here.

## What Changes

- Declare `#[cfg(test)] mod tests;` in `api_process_adapter.rs` and move the inline `mod tests` body to a new sibling child module file `api_process_adapter/tests.rs`. The parent file keeps its name and path, so no external `use` changes and no rename churn.
- Move the 19 `#[cfg(test)]` items out of the production half into the same tests module, including the four `#[cfg(test)]` helper methods currently declared inside production `impl` blocks — an inherent `impl` may live in any module of the defining crate, so they move without changing call sites inside the tests.
- Lower the recorded path budget for `api_process_adapter.rs` to the post-move measurement.
- **No production logic changes.** No function body, signature, trait implementation, or control flow in non-test code is modified. The test bodies themselves are moved verbatim.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This is a pure code-organization refactor with no externally observable behavior change: no Tauri command, SQLite schema, adapter contract, or runtime behavior is affected in either the desktop or Web runtime. The change sets `skip_specs: true`.

## Impact

- `src-tauri/src/contexts/agent_runtime/infrastructure/api_process_adapter.rs` — loses its inline test module and its test-only items; keeps every production item.
- `src-tauri/src/contexts/agent_runtime/infrastructure/api_process_adapter/tests.rs` — new file holding the moved tests and test scaffolding.
- `src-tauri/tests/architecture.rs` — the `api_process_adapter.rs` path budget drops; the `agent_runtime/infrastructure` subtree budget is expected to need a small explicit raise for the `mod` declaration and re-imports the split adds.
- No frontend file is touched. No frontend/backend isolation or runtime adapter boundary is affected.
