import { describe, expect, it } from "vitest";
import type { ChatMessage, ToolUseBlock } from "../../types/chat";
import { approvedPlanExitCallId } from "./plan-exit-signal";

function message(toolUse: ChatMessage["toolUse"]): ChatMessage {
  return {
    id: "message",
    sessionId: "session-1",
    role: "assistant",
    content: "",
    createdAt: "2026-08-15T00:00:00Z",
    status: "completed",
    toolUse,
  } as ChatMessage;
}

const block = (id: string, name: string, status: ToolUseBlock["status"]): ToolUseBlock =>
  ({ id, name, status, input: null, output: null });

describe("approvedPlanExitCallId", () => {
  it("reports the call id of an approved request", () => {
    const messages = [message([block("call-1", "exit_plan_mode", "completed")])];
    expect(approvedPlanExitCallId(messages)).toBe("call-1");
  });

  it("reports nothing while the request is still awaiting a decision", () => {
    const messages = [message([block("call-1", "exit_plan_mode", "awaiting_input")])];
    expect(approvedPlanExitCallId(messages)).toBeNull();
  });

  it("reports nothing for a declined request", () => {
    // Declining marks the block failed, so a decline can never read as an approval.
    const messages = [message([block("call-1", "exit_plan_mode", "failed")])];
    expect(approvedPlanExitCallId(messages)).toBeNull();
  });

  // The newest request decides. Without this, declining after an earlier approval would still
  // report the old approval and put the session back into execute mode.
  it("lets a later decline override an earlier approval", () => {
    const messages = [
      message([block("call-1", "exit_plan_mode", "completed")]),
      message([block("call-2", "exit_plan_mode", "failed")]),
    ];
    expect(approvedPlanExitCallId(messages)).toBeNull();
  });

  it("ignores other tools and messages without tool blocks", () => {
    const messages = [
      message(undefined),
      message([block("call-q", "ask_user_question", "completed")]),
      message([block("call-e", "edit", "completed")]),
    ];
    expect(approvedPlanExitCallId(messages)).toBeNull();
    expect(approvedPlanExitCallId([])).toBeNull();
  });

  it("finds the request among sibling tool blocks in one message", () => {
    const messages = [
      message([
        block("call-g", "grep", "completed"),
        block("call-1", "exit_plan_mode", "completed"),
      ]),
    ];
    expect(approvedPlanExitCallId(messages)).toBe("call-1");
  });
});
