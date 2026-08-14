## Why

A native Agent has no way to write down what it is doing. On a task with more than a handful of steps it re-derives the plan from the transcript on every turn, drops steps that scrolled out of the recent-message window, and gives the user no signal about progress beyond prose. Context compaction makes this worse, not better: the summary preserves the conversation's gist, which is exactly where an enumerated checklist degrades fastest.

The existing Todo Board does not solve this. It is a user-owned organization surface fed by Sessions, Plans, and scheduled tasks; it is not something the model can write to mid-turn, and it should not be — a model revising its working checklist ten times in one turn would churn a board the user curates.

## What Changes

- Add a `todo_write` baseline tool that replaces the calling session's task list in one call and returns the normalized list.
- Keep the task list in session-scoped runtime state rather than in the message history, and inject it as a bounded system-prompt section so it survives context compaction intact.
- Enforce list invariants at the tool boundary: a bounded item count, bounded item text, and at most one in-progress item at a time.
- Offer the tool in plan mode as well as execute mode, since it writes VaneHub-internal state and has no workspace or network effect.
- Classify the tool as a no-approval operation, alongside the other tools that only touch VaneHub's own storage.

## Capabilities

### New Capabilities

- `agent-task-list`: Defines the session-scoped Agent task list, its replacement semantics, invariants, bounds, system-prompt projection, and lifecycle.

### Modified Capabilities

- `agent-tool-execution`: Adds `todo_write` to the baseline and plan-mode catalogs and classifies it as a no-approval operation.

## Impact

- Only the Rust runtime and the native tool-use loop are affected; no SQLite schema, Tauri command, frontend service contract, or React surface changes.
- The task list is runtime-only and session-scoped, so it does not enter the migration sequence and does not survive a desktop restart.
- The Todo Board's records, sources, and lifecycle are untouched; this list is deliberately a separate, model-owned scratchpad.
- The system prompt gains one more optional section, bounded like the existing Skill and memory sections.
- No new package dependencies are introduced.
