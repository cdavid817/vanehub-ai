## Why

Plan mode is a one-way door the model cannot knock on. A session set to `plan` gets the read-only catalog for every generation, and the only way out is the user reaching for the mode selector. The model can write a complete plan and then has no way to say "this is done — approve it and let me build it." It can only stop and hope the user notices.

`add-agent-user-question` closed the neighbouring gap: the model can now ask a bounded question and block on the answer. Leaving plan mode is the same shape — a bounded request, published to the chat surface, blocked on the user — but it is not a question, and answering it in prose would not move the session out of plan mode.

This is deliberately not the OnePiece Plan/PlanRun approval boundary, which freezes a reviewed Plan version, creates a PlanRun and an integration worktree, and is governed by its own requirement. That boundary stays exactly where it is and stays user-driven. What is missing is the much smaller thing: a chat session whose execution mode is `plan` has no model-initiated way to propose becoming `execute`.

## What Changes

- Add an `exit_plan_mode` tool that submits the plan the model has settled on, publishes it to the session's chat surface, and blocks until the user approves or declines.
- Offer it only in plan mode, so it costs nothing in the catalog of a session that is not planning.
- On approval, move the session's execution mode from plan to execute, and tell the model the change applies to the next turn rather than the one it is in.
- On decline, leave the session in plan mode and tell the model so it can revise rather than retry.
- Refuse in a non-interactive execution context, the way a question already does, rather than blocking a run nobody is watching.
- Leave the OnePiece Plan/PlanRun approval boundary untouched: this tool never freezes a Plan version, creates a PlanRun, or provisions a worktree.

## Capabilities

### New Capabilities

- `agent-plan-exit-request`: A model-initiated, user-approved transition out of plan mode for a native API agent session.

### Modified Capabilities

- `agent-chat-configuration`: Names the one plan-mode tool that exists to request leaving plan mode, so the read-only restriction and this request are read together rather than as a contradiction.

## Impact

- The plan-mode tool catalog gains one tool; the ordinary catalog is unchanged, so no request outside plan mode pays for it.
- A new command delivers the decision over the existing blocked-tool-call transport. It records no permission grant, matching the precedent set for answering a question: this authorizes a session mode, not an action on a resource.
- The chat surface gains a card for reviewing the proposed plan and approving or declining it.
- Approval takes effect on the next generation. The catalog and permission policy for a generation are resolved once at its start, so the in-flight turn keeps the read-only tools it was given; design D3 records why this is stated to the model rather than worked around.
- No new package dependencies, no new persistence, and no change to the OnePiece Plan/PlanRun workflow.
