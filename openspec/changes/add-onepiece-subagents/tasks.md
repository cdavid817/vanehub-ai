## 1. Admission and isolation

- [x] 1.1 Add the `delegate_subagent` handler to the OnePiece-only native tool registry with its eligibility predicates and closed schema.
- [ ] 1.2 Admit a child attempt bound to the parent session, generation, workspace, and an immutable Profile snapshot, rejecting caller-supplied scope.
- [ ] 1.3 Give the child its own context rather than a copy of the parent's transcript, reusing the existing attempt execution profile.
- [ ] 1.4 Reject nested delegation from inside a child attempt.
- [ ] 1.5 Provision an isolated worktree for a mutating child and forbid two children sharing one.

## 2. Authority and tool pool

- [ ] 2.1 Assemble the restricted child tool pool: read-only exploration plus the task-list tool by default.
- [ ] 2.2 Grant workspace mutation only when the parent session's permission mode already permits it and the caller asks explicitly.
- [ ] 2.3 Refuse `ask_user_question`, `delegate_cli`, `apply_delegation_changes`, and `delegate_subagent` inside a child, at the executor rather than only in the offered catalog.
- [x] 2.4 Classify child start as its own delegation-start operation defaulting to explicit approval.

## 3. Bounds, results, and lifecycle

- [ ] 3.1 Enforce per-attempt tool-call, token, duration, and result-size ceilings with classified limit outcomes.
- [ ] 3.2 Enforce a per-session concurrent-child cap without terminating running children.
- [ ] 3.3 Return a bounded structured result and keep the child's turns and tool output out of the parent's transcript.
- [ ] 3.4 Report child progress through the parent's task list and execution observability.
- [ ] 3.5 Seal a mutating child's changes as a ChangeSet applied through the existing once-only approval.
- [ ] 3.6 Cancel and reap children, processes, and worktrees on parent generation cancellation and session end.

## 4. Accounting, logging, and boundary

- [ ] 4.1 Attribute child usage to a distinguishable purpose that still rolls up to the parent session.
- [ ] 4.2 Keep durable logs to identifiers, outcome codes, counts, and timing.
- [ ] 4.3 Reuse the parent's Profile-scoped credential without copying it into records, prompts, or telemetry.
- [ ] 4.4 Expose child attempt visibility through the shared service boundary with Tauri and Web/mock implementations.

## 5. Tests

- [x] 5.1 Eligibility tests: OnePiece only, execute mode only, workspace required, closed schema, forged-scope rejection.
- [ ] 5.2 Authority tests: default read-only pool, mutating request refused from a plan-mode parent, each prohibited tool refused at the executor.
- [ ] 5.3 Nesting rejection test.
- [ ] 5.4 Bound tests for every ceiling and for the concurrency cap, including that a rejected start leaves running children alone.
- [ ] 5.5 Isolation tests: parent workspace unmodified until apply, distinct worktrees for concurrent children.
- [ ] 5.6 Lifecycle tests for cancellation and reaping on both edges.
- [ ] 5.7 Context-economy test asserting the child's transcript never enters the parent's turns.
- [ ] 5.8 Accounting and redaction tests.
- [ ] 5.9 Web/mock parity tests.

## 6. Validation

- [ ] 6.1 `npm run lint:ci`
- [ ] 6.2 `npm run test`
- [ ] 6.3 `npm run build`
- [ ] 6.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [ ] 6.5 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [ ] 6.6 `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] 6.7 `openspec validate add-onepiece-subagents --strict`

## Status

The governance half is implemented and verified: the tool identity, its OnePiece-only eligibility,
its closed input contract, its approval classification, the `VANEHUB_ONEPIECE_SUBAGENT_ENABLED`
gate (default off), and registry membership. The handler is registered against the unavailable
port and reports `backend_unavailable`.

The child attempt executor is not implemented, and it is the substance of this change. The
investigation that matters for whoever picks this up: the existing Utility delegation runtime does
**not** run a tool loop -- `NativeUtilityChildExecutor` calls `summarize_turns` and hardcodes its
tool counts to zero, so it is a one-shot completion, not a child agent. A subagent needs a real
child tool loop, which does not exist anywhere yet.

It is tractable rather than open-ended: `summarize_turns_with_usage` in `api_process_adapter.rs`
already performs the whole SSE read with a `ToolCallAccumulator` and simply passes no tools and
discards tool calls. A child loop is that function with a read-only catalog, plus
`execute_tool_call_impl` behind an allowlist, plus the ceilings. Everything under sections 2, 3,
and 4, and tests 5.2 onward, depends on it.

Do not archive until the executor lands.
