## 1. Capture the baseline

- [x] 1.1 Record `execute_with_code_intelligence`'s physical line span, `execution.rs`'s line count,
      and the `agent_runtime/infrastructure` subtree aggregate
- [x] 1.2 Capture the unsorted test-name list via
      `cargo test --manifest-path src-tauri/Cargo.toml --lib -- --list`, for the post-change subset
      comparison
- [x] 1.3 Save the pre-extraction text of each of the eight candidate fragments, for the
      text-identity check in 5.2

## 2. Establish coverage before cutting anything

Each of these is written against the **un-split** function and must pass before any extraction
begins. No existing test may be edited.

- [x] 2.1 Add a `RejectingSink` event-sink double that refuses events matching a predicate — the
      only way to reach the eight `failed_retryable("Agent generation event handling failed.")`
      exits, since `CapturingSink` never fails
- [x] 2.2 `a_rejected_token_event_fails_the_generation_retryably` — covers the SSE loop's sink exit
      (Seam 6)
- [x] 2.3 `a_rejected_completed_tool_use_event_fails_the_generation_retryably` — covers the tool
      outcome tail's sink exit (Seam 7)
- [x] 2.4 `a_rejected_awaiting_approval_event_fails_the_generation_retryably` — covers the
      permission gate's sink exit, which fires before `create_pending_approval` (Seam 8)
- [x] 2.5 `an_answer_delivered_to_an_approval_wait_is_treated_as_a_denial` — covers
      `ApprovalOutcome::Answered(_)`'s fail-closed arm (Seam 8)
- [x] 2.6 `a_policy_denied_tool_call_returns_denial_data_without_executing` — covers `Effect::Deny`
      (Seam 8)
- [x] 2.7 `an_endpoint_profile_context_window_smaller_than_the_request_fails_the_generation` — the
      first test in the suite to set `endpoint_profile: Some(..)`; covers profile → `ContextCapacity`
      → snapshot → overflow guard (Seams 1, 4, 5)
- [x] 2.8 Confirm all six pass against the unmodified function, and that the only change to
      `tests.rs` is additions

## 3. Extract the seams with no early exits

Each leaves `cargo test` green before the next begins.

- [x] 3.1 `prompt.rs` — `resolve_generation_tool_catalog` (execution.rs 232-287)
- [x] 3.2 `prompt.rs` — `resolve_generation_skill_tools` (292-335), keeping the three `let mut`
      bindings in the parent so the catalog lease's drop order is unchanged
- [x] 3.3 new `endpoint.rs` — `resolve_image_support` (384-400)
- [x] 3.4 `invocation.rs` — `analyze_round_context` (430-455)

## 4. Extract the seams whose exits become `Err`

- [x] 4.1 `endpoint.rs` — `resolve_endpoint` returning `Result<ResolvedEndpoint, _>` (99-181); the
      caller destructures into the same five names so the body below is untouched
- [x] 4.2 `execution.rs` — `stream_round` returning `Result<StreamedRound, _>` (525-613), preserving
      the asymmetry that the two sink-failure exits do **not** call `finish_api_invocation`
- [x] 4.3 `execution.rs` — `record_tool_outcome` returning `Result<ExecutedToolCall, _>`, replacing
      all five duplicated tails
- [x] 4.4 `interactive.rs` — `authorize_tool_call` returning a three-variant `ToolAuthorization`
      (871-953); the `continue`s stay in the parent
- [x] 4.5 `mod.rs` — declare `endpoint` and re-export whatever `tests.rs`'s `use super::*;` needs

## 5. Prove the extraction was faithful

- [x] 5.1 The 1.2 test-name list is a subset of the post-change list, and the difference is exactly
      the six tests added in section 2
- [x] 5.2 Each extracted fragment diffs against its 1.3 text with no change other than the exit
      rewrite, the receiver rename, and indentation
- [x] 5.3 `git diff` on `tests.rs` contains only added lines
- [x] 5.4 `cargo test --manifest-path src-tauri/Cargo.toml` passes; the total test count rose by
      exactly six

## 6. Budgets and verification

- [x] 6.1 Lower the `[ARCH-NATIVE-006]` budget for `execution.rs` to its measured value and rewrite
      the residual comment with the new function size
- [x] 6.2 Re-measure the `agent_runtime/infrastructure` subtree; raise `[ARCH-NATIVE-007]` only by
      an itemized amount (new module scaffolding, helper signatures, call sites, new tests) —
      investigate anything the itemisation does not explain
- [x] 6.3 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 6.4 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 6.5 `npm run architecture:check`
- [x] 6.6 `npx openspec validate decompose-api-tool-use-loop --strict` and
      `npx openspec validate --specs --strict`
- [x] 6.7 Record the residual: the function's final size, and each declined seam with its reason
