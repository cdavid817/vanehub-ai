# Design

## D1. What this is, against the two delegations that already exist

| | `delegate_utility_skill` | `delegate_cli` | `delegate_subagent` |
| --- | --- | --- | --- |
| Who runs it | A named Utility Skill | An external Claude Code / Codex process | OnePiece itself |
| Instructions | The Skill's pinned revision | The external CLI's own prompt | The caller's task text |
| Why you reach for it | A packaged specialist procedure | A second vendor's judgment | Exploration that would flood the parent's context |

The gap the third fills is not capability, it is *context economy*. A search across a codebase
costs the main session every file it read on the way to the answer, and that accumulated transcript
is what triggers compaction — so the act of exploring degrades the session that needed the answer.
A child pays that cost in its own window and returns a paragraph.

## D2. Progress goes to the user, not into the parent's context

The obvious design — stream the child's turns into the parent — reintroduces the exact cost the
child exists to avoid. Nothing the child reads may enter the parent's context; only its conclusion
may.

This section originally proposed the parent's task list as the progress channel. Implementation
showed that was wrong, for a reason that is only obvious once the loop exists: **the parent is
blocked inside the `delegate_subagent` tool call for the child's entire run.** It cannot read a
task list, or anything else, until the child returns. There is no moment during the child's work
at which the parent could observe progress, so a channel aimed at the parent conveys nothing.

The audience for progress is the user watching a long investigation, and the channel that reaches
them already exists on the execution context: `NativeToolProgressSink`. The child publishes a
bounded event per turn carrying a count and a fixed phrase — never a path, a pattern, or anything
it read, since the child's reading staying inside its own context is the whole feature.

That also removes this change's stated dependency on `add-agent-task-list`. The task list remains
the right home for the *parent's* own plan; it was never the right home for a blocked caller's
view of work it cannot see.

## D3. A child never has more authority than its parent

Two rules, both load-bearing:

- **Mode inheritance.** A plan-mode parent cannot spawn a mutating child. Otherwise
  `delegate_subagent` becomes a plan-mode escape hatch — the model could not write files, so it
  would delegate writing them.
- **Approval does not widen.** Approving the child start authorizes *the delegation*, not the
  child's individual effects. The child's pool is still bounded by the parent's mode, and its
  changes still land through the existing once-only ChangeSet approval.

The prohibited-tool list follows from this. `ask_user_question` is refused because a child is a
non-interactive context (`add-agent-user-question` D4) — a blocked child burns its ceiling with
nobody to answer. `delegate_cli` and `apply_delegation_changes` are refused because a child must
not be able to start a third-party process or commit changes to the parent's repository.
`delegate_subagent` is refused because of D4.

## D4. No nesting, and why that is not arbitrary

`utility-skill-delegation-runtime` already prohibits nested Utility delegation, and the same
reasoning applies harder here. Nesting turns a bounded fan-out into an unbounded tree: ceilings
are per-attempt, so depth multiplies them, and a model that decomposes recursively will discover
that. It also makes cancellation a tree-walk instead of a list-walk, which is exactly the code
that is hardest to get right and most costly to get wrong — an un-reaped grandchild worktree on
Windows is a directory nobody can delete.

Flat fan-out with a per-session concurrency cap gets the parallelism without any of that.

## D5. Mutating children reuse the sealed-ChangeSet path, not a second write path

`complete-onepiece-builtin-tool-system` already established how untrusted work becomes a repository
change: an isolated workspace, a sealed ChangeSet, and a non-rememberable once-only approval bound
to content hash, diff hash, repository identity, exact base commit, and clean-state witness.

A child writing directly into the parent's workspace would bypass all of that, and two concurrent
children writing there would corrupt each other. So mutating children get their own worktree and
return a ChangeSet through the existing path. This change adds no new way to mutate a repository —
that is the single most important constraint here.

Read-only children need none of this, which is why read-only is the default: the common case
(exploration) stays cheap, and the expensive machinery engages only when someone asks for it.

## D6. Bounds

| Bound | Rationale |
| --- | --- |
| Concurrent children per parent session | Caps fan-out; a rejected start never terminates a running child to make room. |
| Tool calls, tokens, wall-clock per attempt | Mirrors the existing attempt execution profile rather than inventing a second ceiling vocabulary. |
| Result characters | The result enters the parent's context, so it is the one bound the parent directly pays for. |

Nothing retries automatically. An attempt that hit a ceiling and a replacement attempt that hits
the same ceiling differ only in cost.

## D7. Sequencing

This is easier after `add-agent-user-question`, which establishes what "non-interactive execution
context" means as a runtime property rather than a special case — a child refuses to ask for
exactly that reason. The dependency on `add-agent-task-list` that this section originally claimed
does not survive D2's correction: a blocked parent cannot read a task list, so the child never
needed one.

## D8. What mutating children still need, and what is already settled

Read-only children shipped first and are complete. Mutating children are the last piece, and this
section records what an implementation survey settled so it does not have to be redone.

**Settled: the ChangeSet path is reusable as-is.** `ChangeSetRecord`, `ChangeSetFileRecord`, and
`ChangeSetStatus` live in `agent_runtime/application/native_tools/persistence.rs`, not in
`cli_delegation`. They are shared runtime types keyed by `attempt_id`, so a subagent attempt can
produce one and the existing `apply_delegation_changes` handler and its once-only exact-ChangeSet
approval apply it unchanged. There is no second write path to build and no context boundary to
generalize across, which was the open risk when this change was written.

**Settled: the isolation primitive is a platform concern, not a delegation one.**
`platform::git::GitAdapter::execute(root, args, timeout)` is a generic git runner. Worktree
provisioning, the diff, and cleanup all go through it directly. `cli_delegation`'s
`IndependentGitWorkspaceAdapter` and `GitDelegationChangeSetCapture` are the same idea already
solved, but typed to that context's domain; they are a reference, not a dependency.

**Not settled, and the reason this is a whole build rather than a wiring change:** the result has
to be *sealed* before the worktree goes away. A mutating child whose changes are captured but not
sealed into an Artifact leaves its edits in a directory that is about to be deleted -- worse than
not running it, because the model reports work that no longer exists. So the minimum honest
delivery is the whole chain: preflight the workspace is a clean git repository, provision a
detached worktree, widen the child's pool to writes *scoped to that worktree*, capture the diff,
seal it as an Artifact, insert the ChangeSet record, and reap the worktree on every exit path
including cancellation. That needs the Artifact service in the subagent executor, which it does
not have today.

**The one property that must not be lost.** Today a child cannot write because there is no code
path to a write: `execute_child_tool` dispatches to three read-only functions and calls the file
tool with a hardcoded `"read"`. Adding mutation turns that structural guarantee into a mode flag,
which is a genuine weakening. The way to keep most of it is to build the mutating pool as a
*separate* dispatcher rather than a conditional inside the read-only one, so the read-only path
still cannot express a write no matter what the flag says.
