import type { ChatMessage } from "../../types/chat";

/** Must match the Rust tool name; the block carries it verbatim. */
export const EXIT_PLAN_MODE_TOOL_NAME = "exit_plan_mode";

/**
 * The call id of the most recent approved `exit_plan_mode` request, or null.
 *
 * Read off the tool block rather than passed up from the button that approved it. The backend
 * marks an approved request `completed` and a declined one `failed`, and it only emits that block
 * if the blocked generation was still alive to receive the decision — which is exactly the
 * condition under which the session should change mode. Deciding from the button instead would
 * mean drilling a callback through five presentational components and would leave a session
 * write-capable on the strength of a decision that reached a dead generation.
 */
export function approvedPlanExitCallId(messages: ChatMessage[]): string | null {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const blocks = messages[index]?.toolUse;
    if (!blocks) continue;
    for (let position = blocks.length - 1; position >= 0; position -= 1) {
      const block = blocks[position];
      if (block.name !== EXIT_PLAN_MODE_TOOL_NAME) continue;
      // The newest one decides: a later declined request must not be overridden by an older
      // approval still sitting further up the transcript.
      return block.status === "completed" ? block.id : null;
    }
  }
  return null;
}
