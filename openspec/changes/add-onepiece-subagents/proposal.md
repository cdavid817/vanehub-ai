## Why

OnePiece has two ways to hand work off, and neither is "do this yourself, somewhere else". `delegate_utility_skill` runs a *named Utility Skill* under tight ceilings and forbids nesting; `delegate_cli` hands the job to an external Claude Code or Codex process. What is missing is the shape that matters most for exploration: spawn a copy of OnePiece with its own context window, give it one bounded question, and get back only the answer.

Without it, every "find where X is handled across this codebase" costs the main session the full transcript of the search — dozens of file reads it will never need again — and that transcript is what drives context compaction, which is what degrades the session. The cost of exploring is paid in the one resource the main session cannot get back.

## What Changes

- Add a `delegate_subagent` tool, eligible only for stable Agent id `onepiece`, that runs a bounded child OnePiece attempt with its own context and returns a bounded structured result.
- Give the child a restricted tool pool: read-only exploration by default, with workspace mutation available only when the parent's own session already permits it and the caller asks for it explicitly.
- Bound every attempt by tool calls, tokens, wall-clock, and result size, and forbid nesting so a child cannot spawn its own children.
- Report child progress into the parent's task list rather than into the parent's transcript, so the parent sees advancement without paying for the child's context.
- Run mutating children in an isolated worktree and return a sealed ChangeSet through the existing delegated-apply path, rather than letting a child write into the parent's workspace concurrently.
- Reuse the existing attempt-execution profile, credential isolation, token accounting, and unified logging rather than adding a parallel execution stack.

## Capabilities

### New Capabilities

- `onepiece-subagents`: Defines child attempt admission, the restricted tool pool, isolation, bounds, nesting prohibition, progress reporting, result sealing, and cancellation.

### Modified Capabilities

- `agent-tool-execution`: Adds `delegate_subagent` to the OnePiece-only registry with its eligibility and approval classification.

## Impact

- The Rust runtime gains a child attempt coordinator layered on the existing OnePiece attempt execution profile; no new provider execution branch is introduced.
- Mutating children depend on the existing worktree isolation and the existing once-only ChangeSet apply approval; this change does not introduce a second way to mutate a repository.
- Token accounting must attribute child consumption to a distinguishable purpose while still rolling up to the parent session.
- The frontend service boundary gains child attempt visibility with matching Tauri and Web/mock implementations.
- This change depends on `add-agent-task-list` for progress reporting and on the existing delegated-apply path for mutating results; it should not start before both are in place.
- No new package dependencies are introduced.
