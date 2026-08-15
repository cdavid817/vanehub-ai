## Why

The OnePiece core instructions tell the Agent to "surface blockers clearly", but the Agent has no way to actually ask. It can only write a question into its reply and end the turn, which means the user answers in free text, the Agent re-parses that text, and an ambiguity that had two plausible readings is resolved by guessing at least as often as by asking. Every other blocking interaction in the runtime — tool approval — already has a structured round trip; clarification does not.

## What Changes

- Add an `ask_user_question` tool that presents one question with a small set of concrete options and blocks that tool call until the user chooses.
- Emit a distinct `awaiting_input` tool status so the chat surface renders a choice affordance rather than the allow/deny affordance approval uses.
- Resolve the answer through the existing blocked-tool-call channel by extending the tool resolution vocabulary, rather than adding a second blocking mechanism beside approvals.
- Always accept a free-text answer alongside the offered options, so an incomplete option set never traps the user.
- Refuse the tool in execution contexts that have no interactive user — Loop workers, scheduled-task runs, Plan attempts, and delegated Utility attempts — with an immediate error rather than a wait no one can end.
- Cancel a waiting question when its generation is cancelled, exactly as a waiting approval is cancelled today.

## Capabilities

### New Capabilities

- `agent-user-question`: Defines the structured clarification round trip, its bounds, the non-interactive refusal, cancellation, and the Web/mock behavior.

### Modified Capabilities

- `agent-tool-execution`: Adds `ask_user_question` to the baseline and plan-mode catalogs, adds the `awaiting_input` lifecycle status, and classifies the tool as a no-approval operation.

## Impact

- The Rust runtime gains a pending-question channel, a tool handler, and a resolution command; the tool-resolution decision type gains an answered variant.
- The frontend service boundary gains one method with matching Tauri and Web/mock implementations, and the chat surface gains a question card beside the existing approval card.
- Localization resources gain the question card's strings in every shipped locale.
- No SQLite schema changes: a pending question is runtime state with the same lifetime as the generation that is waiting on it.
- No new package dependencies are introduced.
