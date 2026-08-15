## 1. Admission and isolation

- [x] 1.1 Add the `delegate_subagent` handler to the OnePiece-only native tool registry with its eligibility predicates and closed schema.
- [x] 1.2 Admit a child attempt bound to the parent session, generation, and workspace, rejecting caller-supplied scope.
- [x] 1.3 Give the child its own context rather than a copy of the parent's transcript.
- [x] 1.4 Reject nested delegation by omitting `delegate_subagent` from the child surface entirely.
- [x] 1.5 Provision an isolated worktree for a mutating child and forbid two children sharing one.

## 2. Authority and tool pool

- [x] 2.1 Assemble the restricted child tool pool: bounded file reads, content search, and filename search.
- [x] 2.2 Grant workspace mutation only when the caller asks explicitly, through a separate dispatcher.
- [x] 2.3 Refuse every tool outside that surface at the dispatcher, not only in the offered catalog.
- [x] 2.4 Classify child start as its own delegation-start operation defaulting to explicit approval.

## 3. Bounds, results, and lifecycle

- [x] 3.1 Enforce per-attempt turn, tool-call, duration, and result-size ceilings with classified limit outcomes.
- [x] 3.2 Enforce a per-session concurrent-child cap without terminating running children.
- [x] 3.3 Return a bounded structured result and keep the child's turns and tool output out of the parent's transcript.
- [x] 3.4 Report child progress through the execution context's progress sink.
- [x] 3.5 Seal a mutating child's changes as a ChangeSet for the existing once-only apply approval.
- [x] 3.6 Cancel children on parent generation cancellation and session end.

## 4. Accounting, logging, and boundary

- [x] 4.1 Attribute child usage to a distinguishable purpose that still rolls up to the parent session.
- [x] 4.2 Keep durable result metadata to counts only.
- [x] 4.3 Reuse the parent's credential and provider configuration through the existing boundary without copying them.
- [x] 4.4 Expose child attempt visibility through the existing native tool operations surface.

## 5. Tests

- [x] 5.1 Eligibility tests: OnePiece only, execute mode only, workspace required, closed schema, forged-scope rejection.
- [x] 5.2 Authority tests: the exact child surface, the unreachable write path, and each prohibited tool refused at the dispatcher.
- [x] 5.3 Nesting rejection test.
- [x] 5.4 Bound tests for the result cap and the per-session concurrency cap, including that a refused claim leaves running children alone.
- [x] 5.5 Isolation tests: parent workspace unmodified, worktree reaped on drop, dirty and non-repository refusals.
- [x] 5.6 Lifecycle tests for cancellation, worktree reaping, and the turn ceiling.
- [x] 5.7 Context-economy test asserting the child's tool output never enters the parent's result beyond its bounded answer.
- [x] 5.8 Accounting and redaction tests.
- [x] 5.9 Capability-mapping guards so an extended tool cannot fall through to the filesystem fallback.

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

Child progress is published through the execution context's progress sink, not the parent's task
list as the design first proposed. The parent is blocked inside this tool call for the child's
whole run, so it cannot read a task list until the child returns; the user watching can. Progress
carries counts and a fixed phrase only.

Cancellation is inherited rather than implemented here: the child's execution context shares the
generation's cancellation flag, so cancelling the generation -- which is what ending or archiving
a session does -- stops the child at its next turn boundary. A test pins that, because inherited
guarantees are the ones that quietly disappear.

Mutating children are implemented. A child asked for `change_files` gets a detached git worktree
of the parent's exact base commit, edits only there, and its work is captured and sealed as a
ChangeSet Artifact plus record before the worktree is reaped. The parent's workspace is never
touched; applying the change set is the user's separate, already-existing decision.

Two preflights refuse rather than warn: the workspace must be a git repository (otherwise there is
no base commit to bind to) and must be clean (otherwise the captured change set cannot state what
it applies to).

The read-only guarantee survived the addition. `execute_child_tool` and
`execute_mutating_child_tool` are separate dispatchers, not one function with a flag, so the
read-only path still has no code path to a write. A test writes through both and asserts the
read-only one leaves the file unchanged.

The turn loop is now driven end to end by a scripted SSE endpoint on loopback, which is the only
way to cover the sequence itself: that a tool call is executed and its result carried back into
the next request, that the loop stops when the model stops asking for tools, and that the turn
ceiling terminates a model that never concludes. The fixture is a plain `TcpListener` serving
canned event streams -- no new dependency, and reusable for the Utility delegation executor, which
has the same coverage gap.

One caveat is recorded in the test module: `reqwest`'s builder honours `ALL_PROXY`/`HTTP_PROXY`,
so a SOCKS proxy in the environment intercepts the loopback request and these fail with a
transport error rather than an assertion. They must run with those unset.
