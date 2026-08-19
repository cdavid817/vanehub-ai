## 1. Capture the baseline

- [x] 1.1 Record `api_process_adapter.rs` physical lines and the `agent_runtime/infrastructure` subtree aggregate
- [x] 1.2 Capture the sorted and unsorted test-name lists via `cargo test --manifest-path src-tauri/Cargo.toml --lib -- --list`, for the post-split comparison
- [x] 1.3 Capture the multiset of top-level item names (`struct`, `enum`, `trait`, `fn`, `const`, `static`, `type`, and each `impl` header) in the production file, for the post-split comparison

## 2. Create the directory module

- [x] 2.1 Convert `api_process_adapter.rs` to `api_process_adapter/mod.rs`, leaving `tests.rs` where it is
- [x] 2.2 Confirm every external `use` path still resolves unchanged — no caller outside the module may be edited
- [x] 2.3 Repoint the guard at `architecture.rs:1285` if the source text it greps moved; confirm it still finds the three behavior test names it asserts on

## 3. Move the regions

Each of these leaves `cargo check` green before the next begins.

- [x] 3.1 `sinks.rs` — `EvidenceToolCounts`, `EvidenceCountingSink` and its two impls
- [x] 3.2 `invocation.rs` — `WireFormat`, `begin_api_invocation`, `begin_child_invocation`, `finish_child_invocation`, `api_invocation_snapshot`, `finish_api_invocation`, `record_accounting_diagnostic`, `record_context_snapshot`, `context_snapshot_diagnostic`, `wire_format_for`
- [x] 3.3 `prompt.rs` — tool catalog resolution, system prompt formatting, personalization settings, memory section formatting
- [x] 3.4 `compaction.rs` — compaction predicates, automatic compaction, optimization, context quality assessment, optimizer and control records, candidate turns
- [x] 3.5 `generation.rs` — `run_generation`, `project_native_outcomes`, `GenerationOptions` and its helpers, summarization, streaming, child turns, memory extraction
- [x] 3.6 `interactive.rs` — permission action mapping, `ask_user_question`, `request_plan_exit`, input validation, `await_approval`, plan-mode denial
- [x] 3.7 `execution.rs` — `execute_with_code_intelligence` moved intact, plus skill tool dispatch and lifecycle
- [x] 3.8 `native_tools.rs` — skill input types and reads, registered native tools, todo write, shell background/output/kill, code intelligence tools, remember/recall/search
- [x] 3.9 `mod.rs` retains the struct, its three impls, constants, and type aliases

## 4. Keep visibility minimal and reviewable

- [x] 4.1 Grant each moved item the least visibility its callers require, defaulting to `pub(super)`; an item already `pub(crate)` before the split keeps that
- [x] 4.2 Record every item that needed more than `pub(super)`, with the reason, so the exception list is the review surface rather than the whole diff
- [x] 4.3 Mark re-exports in `mod.rs` that exist only to satisfy `tests.rs`'s `use super::*;`, distinguishing them from the module's intended API
- [x] 4.4 Confirm `tests.rs` is byte-identical — a compile failure there is fixed by adding a re-export, never by editing the test

## 5. Prove the move was pure

- [x] 5.1 Test-name lists match the 1.2 baseline byte-for-byte, unsorted included
- [x] 5.2 The top-level item multiset matches the 1.3 baseline — nothing dropped, nothing duplicated
- [x] 5.3 `cargo test --manifest-path src-tauri/Cargo.toml` passes with an unchanged total test count
- [x] 5.4 Confirm no function body differs from its pre-split text apart from indentation

## 6. Budgets and verification

- [x] 6.1 Remove the now-absent `api_process_adapter.rs` path budget entry; add path budgets for any new module that warrants one
- [x] 6.2 Confirm the `agent_runtime/infrastructure` subtree stayed within 58,072, or raise it by exactly the measured module-boilerplate amount with a stated reason — investigate rather than absorb anything larger
- [x] 6.3 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, and `cargo check --manifest-path src-tauri/Cargo.toml` pass
- [x] 6.4 `npm run architecture:check` passes, including the context-boundary rules that would catch a visibility widening crossing a bounded context
- [x] 6.5 Record the residual explicitly: which modules remain above 1,500 lines, and that `execute_with_code_intelligence` is unchanged at ~978 lines awaiting its own change
- [x] 6.6 `openspec validate split-api-adapter-modules --strict` and `openspec validate --specs --strict` pass
