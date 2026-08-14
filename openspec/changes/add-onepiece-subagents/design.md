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

## D2. Progress goes to the task list, not the transcript

The obvious design — stream the child's turns into the parent — reintroduces the exact cost the
child exists to avoid. The parent must be able to see that work is advancing without paying for
how.

`add-agent-task-list` is the right channel for this and is why this change depends on it: the task
list is already projected into the parent's system prompt, already bounded, and already rewritten
in place rather than appended. A child's progress updating a task line costs the parent nothing per
update.

Execution observability carries the detail for the user's benefit. That surface is not in the
model's context at all, which is the point.

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

This should not start before `add-agent-task-list` (D2's progress channel) and is easier after
`add-agent-user-question`, which establishes what "non-interactive execution context" means as a
runtime property rather than a special case. Both were delivered ahead of it for that reason.
