## 1. Admission and isolation

- [x] 1.1 Add the `delegate_subagent` handler to the OnePiece-only native tool registry with its eligibility predicates and closed schema.
- [x] 1.2 Admit a child attempt bound to the parent session, generation, and workspace, rejecting caller-supplied scope.
- [x] 1.3 Give the child its own context rather than a copy of the parent's transcript.
- [x] 1.4 Reject nested delegation by omitting `delegate_subagent` from the child surface entirely.
- [ ] 1.5 Provision an isolated worktree for a mutating child and forbid two children sharing one.

## 2. Authority and tool pool

- [x] 2.1 Assemble the restricted child tool pool: bounded file reads, content search, and filename search.
- [ ] 2.2 Grant workspace mutation only when the parent session's permission mode already permits it and the caller asks explicitly.
- [x] 2.3 Refuse every tool outside that surface at the dispatcher, not only in the offered catalog.
- [x] 2.4 Classify child start as its own delegation-start operation defaulting to explicit approval.

## 3. Bounds, results, and lifecycle

- [x] 3.1 Enforce per-attempt turn, tool-call, duration, and result-size ceilings with classified limit outcomes.
- [x] 3.2 Enforce a per-session concurrent-child cap without terminating running children.
- [x] 3.3 Return a bounded structured result and keep the child's turns and tool output out of the parent's transcript.
- [ ] 3.4 Report child progress through the parent's task list and execution observability.
- [ ] 3.5 Seal a mutating child's changes as a ChangeSet applied through the existing once-only approval.
- [ ] 3.6 Cancel and reap children, processes, and worktrees on parent generation cancellation and session end.

## 4. Accounting, logging, and boundary

- [x] 4.1 Attribute child usage to a distinguishable purpose that still rolls up to the parent session.
- [x] 4.2 Keep durable result metadata to counts only.
- [x] 4.3 Reuse the parent's credential and provider configuration through the existing boundary without copying them.
- [ ] 4.4 Expose child attempt visibility through the shared service boundary with Tauri and Web/mock implementations.

## 5. Tests

- [x] 5.1 Eligibility tests: OnePiece only, execute mode only, workspace required, closed schema, forged-scope rejection.
- [x] 5.2 Authority tests: the exact child surface, the unreachable write path, and each prohibited tool refused at the dispatcher.
- [x] 5.3 Nesting rejection test.
- [x] 5.4 Bound tests for the result cap and the per-session concurrency cap, including that a refused claim leaves running children alone.
- [ ] 5.5 Isolation tests: parent workspace unmodified until apply, distinct worktrees for concurrent children.
- [ ] 5.6 Lifecycle tests for cancellation and reaping on both edges.
- [x] 5.7 Context-economy test asserting the child's tool output never enters the parent's result beyond its bounded answer.
- [ ] 5.8 Accounting and redaction tests.
- [ ] 5.9 Web/mock parity tests.

## 6. Validation

- [x] 6.1 `npm run lint:ci`
- [x] 6.2 `npm run test`
- [x] 6.3 `npm run build`
- [x] 6.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 6.5 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 6.6 `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 6.7 `openspec validate add-onepiece-subagents --strict`

## Status

Read-only child attempts are implemented and verified end to end: the tool, its eligibility and
approval classification, the child loop, the restricted surface, the ceilings, the per-session
concurrency cap, and the bounded result. The gate `VANEHUB_ONEPIECE_SUBAGENT_ENABLED` still
defaults off. Child turns are accounted under their own `subagent-delegation` purpose, so child
spend rolls up to the parent session while staying separable from the parent's own turns.

The child's authority is structural rather than filtered. `execute_child_tool` dispatches to
exactly three functions -- a bounded file *read*, content search, and filename search -- so a
child has no code path to a write, a process, the network, the user, or another child. The file
tool is called with a hardcoded `"read"`, so a model asking to write reaches the read path rather
than being rejected by a rule that could be got wrong. That is pinned by a test that asserts the
target file is unchanged.

Deferred, and not archivable until they land:

- Mutating children (tasks 1.5, 2.2, 3.5, 5.5). These need an isolated worktree and a sealed
  ChangeSet through the existing once-only apply approval. Read-only was delivered first because
  it is the common case and needs none of that machinery.
- Progress into the parent's task list and execution observability (3.4).
- Cancellation and reaping on session end (3.6): the loop honours its cancellation flag and
  deadline per turn, but nothing reaps a child when the owning session ends.
- Child visibility through the service boundary (4.4, 5.9).
- Live-provider coverage of the loop itself (5.6, 5.8). The pure parts -- catalog, dispatch,
  bounds, result shaping, concurrency -- are tested; the SSE path is not, matching how the
  existing Utility delegation executor is covered.
