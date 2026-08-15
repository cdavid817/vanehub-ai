## Context

Surveyed before writing any of this:

- `is_plan_mode` reads `configuration.execution_mode == "plan"` once per generation, and `resolve_tool_catalog` picks `plan_mode_tool_catalog()` from it. Both are per-generation constants.
- `SessionExecutionMode` (`inherit`/`plan`/`execute`) lives in frontend state (`useChatConfig`) and is sent with each request. The Rust side never sets it; it only reads what arrived.
- `changeExecutionMode` has a special path only for execute→plan with an associated PlanRun (it pauses the run first). plan→execute is the plain `setSessionExecutionMode`.
- `ask_user_question` is handled inline in the tool loop rather than in `execute_tool_call_impl`, because it needs the event sink and the blocked-call channel. It sets `status = "awaiting_input"`, emits the block, and waits on `await_approval`.
- `resolve_agent_question` deliberately does not route through `resolve_pending_approval`: an approval writes a permission record that may become a grant, an answer authorizes nothing. Both share the transport, `api.resolve_tool_approval`.
- `ToolApprovalDecision` already carries `Approved`, `Denied`, and `Answered`.
- `agent-chat-configuration` already governs the OnePiece Plan/PlanRun approval boundary, which freezes a Plan version and provisions a PlanRun and worktree. That is a different mechanism from the session's execution mode.

## Goals / Non-Goals

Goals:

- The model can propose leaving plan mode and have the session actually leave it when the user agrees.
- The user sees the plan being approved before approving it.
- Declining is a first-class outcome the model is told about, not a timeout.

Non-Goals:

- Touching the OnePiece Plan/PlanRun approval boundary, or creating a PlanRun, Plan version, or worktree.
- Letting the model leave plan mode without the user.
- Making the approval take effect inside the generation that requested it (D3).
- Offering the tool outside plan mode, where it would mean nothing.

## Decisions

### D1: An approval, not a question

`ask_user_question` could carry this: ask "approve this plan?" with options. It should not. The answer would be a string the model interprets, and nothing would move the session out of plan mode — the model would believe it had been approved while every later generation still resolved the read-only catalog. The gap being closed is precisely the one between a user saying yes and the session becoming write-capable.

So this is its own tool with a two-outcome decision, resolved through `ToolApprovalDecision::Approved`/`Denied`.

### D2: The decision records no permission grant

It goes over the same blocked-tool-call transport as an answer, through its own command, and writes no permission record — following the reasoning already written down for `resolve_agent_question`. A permission record answers "may this agent do this action to this resource," and may harden into a grant. Approving a plan answers neither: it authorizes a session mode, for this session, once. A grant here would be a standing "always allowed to leave plan mode," which is the opposite of what plan mode is for.

### D3: Approval applies to the next turn, and the model is told so

The tool catalog and the permission policy are both resolved once at the start of a generation. The turn that calls `exit_plan_mode` was handed the read-only catalog and does not have `shell` or `edit` in its tool list at all — approval mid-turn cannot conjure them.

Two ways out: re-resolve the catalog mid-generation, or state the boundary. Re-resolving means a generation whose declared tools change under the provider between round trips, which breaks the prompt-cache prefix and makes "what could the model do during this turn" unanswerable from the request. Not worth it for one round trip.

So the tool's success output says the plan was approved and that write tools become available on the next turn. The model's correct move is to stop and hand back, which is what it would do anyway having just had its plan approved.

### D4: The mode follows the resolved tool block, not the button

This decision changed during implementation, and the original is worth recording because the replacement is better on both counts it was meant to satisfy.

The plan was for the approve button to call the service, check the delivered flag, and then invoke a callback that sets the mode. Wiring that callback meant drilling it through five presentational components — chat tab, message list, message item, tool-use block, activity row — several of which sit near the 300-line cap, to deliver one boolean.

The backend already publishes the signal. An approved request resolves its tool block to `completed`, a declined one to `failed`, and it only emits that block if the blocked generation was alive to receive the decision. So the session reads the decision off the transcript: the newest `exit_plan_mode` block, approved, names the call id, and the config hook moves to execute mode when that id appears.

That is the same "only when a live waiter received it" guarantee, taken from the thing that actually knows rather than reconstructed at the button. It also works no matter which surface approved the request, and it survives the approving component unmounting mid-flight. The button now only delivers the decision.

Keyed on the call id rather than a boolean, so re-running for the same approval cannot fight a user who deliberately switched back to plan mode afterwards.

### D5: Plan mode only

The tool is added to `plan_mode_tool_catalog()` and not to `tool_catalog()`. Outside plan mode there is nothing to exit, and a tool that is always declared costs its schema on every request forever. This also keeps the ordinary catalog's prompt-cache prefix untouched.

## Risks / Trade-offs

- A user approving a plan gets a session that stays `execute` afterwards; there is no automatic return to plan mode. That matches what the mode selector already does when a user switches by hand.
- The model can call this repeatedly if declined. Bounded the same way a question is: it blocks, so a decline costs a round trip, and the decline text tells it to revise rather than re-ask.
- The proposed plan is a bounded string, so a very long plan is rejected rather than truncated into an approval the user only partly read.

## Migration Plan

None. The tool is new, appears only in plan mode, and no stored data changes.

## Open Questions

None.
