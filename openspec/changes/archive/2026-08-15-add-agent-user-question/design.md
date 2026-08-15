# Design

## D1. Reuse the blocked-tool-call channel, do not build a second one

Tool approval already has everything a question needs: a per-call channel keyed by call id
(`PendingApprovals` in `api_process_adapter.rs`), a wait loop that polls cancellation
(`await_approval`), a resolution path from the UI down to that channel
(`AgentRuntimeApi::resolve_tool_approval` → `ToolApprovalPort::resolve`), and a sweep that
delivers a synthetic denial when a generation dies (`bootstrap/permissions.rs`).

So the answer travels on that same channel: the tool-resolution decision gains an answered
variant carrying the answer text, and a new command resolves a question through the existing
`resolve` path. Building a parallel registry would mean writing a second wait loop, a second
cancellation sweep, and a second chance to leave a generation blocked forever — for a mechanism
that is structurally identical.

The one thing that is *not* reused is the permission record. An approval creates a
`PendingApprovalEntry` because the user is authorizing an effect; a question authorizes nothing,
so it creates no permission state.

## D2. The question travels in the tool call's own input

`ToolUseBlock` already carries `input` to the frontend (`src/types/chat.ts`), and the chat surface
already renders tool blocks. So the question text and its options need no separate fetch, no
separate event, and no separate store — the card reads what the model sent.

This is the difference from `ApprovalCard`, which *does* fetch: it needs the permission record
(risk level, action, resource) that lives outside the tool call. A question has no such record.

## D3. `awaiting_input` is a distinct status, not a reuse of `awaiting_approval`

Approval asks "may I do this?" and offers allow/deny. A question asks "which of these did you
mean?" and offers N choices. Rendering the second with the first's affordance would be wrong in
both directions: the user would see a security prompt for a harmless clarification, and the
options the model actually offered would have nowhere to go.

`ToolUseBlock["status"]` in `src/types/chat.ts` gains `awaiting_input`, and `ToolLifecyclePhase`
gains the matching variant. Both are closed unions, so every match site is a compile error until
it is handled — which is the point.

## D4. Non-interactive contexts must refuse, not wait

This is the failure mode that matters most. VaneHub runs native Agents in several places with no
human attached: Loop workers and verifiers, scheduled-task runs, Plan attempts and repairs, and
delegated Utility attempts. A question that blocks in any of those hangs the run until its
tool-call, token, or timeout ceiling fires — turning a two-second clarification into a burned
attempt, and in the Loop case into a burned worktree.

So interactivity is a property of the execution profile, not of the tool: the catalog excludes
`ask_user_question` outside interactive sessions, and the executor refuses it regardless of what
the catalog offered, on the same fail-closed principle plan mode already uses (the catalog shapes
what the model is *told*; the executor is the boundary that *holds*).

## D5. Free text is always accepted

The offered options are the model's guess at the answer space, and that guess is sometimes
incomplete — which is precisely the situation where the tool was worth calling. The card always
offers a free-text field, and whatever the user types is returned verbatim rather than matched to
the nearest option.

## D6. Bounds

| Bound | Value | Rationale |
| --- | --- | --- |
| Questions per call | 1 | A round trip that asks four things at once is a form, not a clarification; the user answers the easy ones and guesses the rest. |
| Options per question | 2–4 | Fewer than two is not a choice. More than four stops being scannable and starts being a menu the model should have narrowed itself. |
| Question characters | 300 | A question, not a briefing. |
| Option characters | 120 | A label, not a paragraph. |

## D7. Cancellation is inherited, not reimplemented

Because the wait uses the approval channel's loop, a cancelled generation already unblocks it, and
the existing sweep that delivers a synthetic denial on generation death already covers a question
left hanging. The answered variant just needs to be treated as "resolved" wherever the decision is
consumed.
