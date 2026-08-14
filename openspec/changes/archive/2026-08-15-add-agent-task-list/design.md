# Design

## D1. Whole-list replacement, not per-item operations

`todo_write` takes the entire list and replaces it. The alternative — `add`, `complete`, `remove` addressed by item id — requires the model to carry stable ids across turns, and it drifts the moment a turn is compacted away or an id is misremembered. A replacement call is idempotent, needs no id vocabulary, and makes every write self-describing: the tool result *is* the current state.

The cost is that the model resends unchanged items. That is the right trade: the list is bounded at 40 short items, so a full rewrite is cheap, while a desynchronized id space is not recoverable.

## D2. The list lives in the system prompt, not the transcript

The list is stored in session-scoped runtime state and injected as a system-prompt section, next to the existing Skill and memory sections.

Putting it in the message history instead would fail exactly when it matters. Context compaction summarizes earlier turns; an enumerated checklist is the first thing a summary flattens, and a half-remembered checklist is worse than none. It would also grow the transcript by one full copy per revision.

As a section it costs one bounded block per generation, is always current rather than as-of-last-mention, and survives compaction untouched.

## D3. At most one item in progress

The tool rejects a list with two or more `in_progress` items. This is a real constraint, not decoration: a model that marks five things in progress has not committed to any of them, and the resulting list tells the user nothing about what is actually happening now. Rejecting at the boundary is what makes the constraint hold — a normalization that silently demoted extras would teach the model that the field does not mean anything.

A list with no in-progress item is allowed: that is the honest state between finishing one step and starting the next, and while planning before execution.

## D4. Separate from the unified Todo Board, deliberately

The Todo Board (`unified-todo-board`) is user-owned: durable work items with a user-controlled stage, reconciled from Sessions, Plans, and scheduled tasks, with an archive lifecycle. This list is model-owned: a scratchpad for one session, rewritten as often as the model reconsiders, gone when the session ends.

Wiring `todo_write` into the board would let a model's mid-turn churn reorder and rewrite records the user curates, and would put board writes on the hot path of a tool loop. Keeping them separate costs a small conceptual overlap and buys a clean ownership boundary.

If the two are ever joined, the direction that preserves that boundary is a board *source* that projects a finished session's list as one work item — a read of the model's list by the board, not a write of the board by the model. That is out of scope here.

## D5. Available in plan mode

Plan mode withholds tools with workspace, process, or network effects. `todo_write` has none: it writes VaneHub's own session state, like `remember`. Planning is also where an explicit list is most valuable, so withholding it there would remove the tool from the mode that needs it most.

## D6. Bounds

| Bound | Value | Rationale |
| --- | --- | --- |
| Items per list | 40 | Well past any real task decomposition; small enough that the projected section stays a bounded prompt cost. |
| Characters per item | 200 | A task title, not a paragraph. Long rationale belongs in the reply, not in a checklist row. |

Both are rejections rather than truncations, matching D3's reasoning: a silently shortened item is a lie about what the model wrote.
