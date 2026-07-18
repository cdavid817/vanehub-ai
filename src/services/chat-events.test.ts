import { describe, expect, it } from "vitest";
import { applyChatEvent } from "./chat-events";
import type { ChatMessage } from "../types/chat";

const baseMessage: ChatMessage = {
  id: "assistant-1",
  sessionId: "session-1",
  role: "assistant",
  content: "",
  status: "streaming",
  createdAt: "2026-07-18T00:00:00.000Z",
  updatedAt: "2026-07-18T00:00:00.000Z",
};

describe("applyChatEvent", () => {
  it("appends a rich block to the target message", () => {
    const [message] = applyChatEvent([baseMessage], {
      type: "rich_block",
      sessionId: "session-1",
      messageId: "assistant-1",
      block: { id: "card-1", kind: "card", v: 1, title: "Summary" },
    });

    expect(message?.richBlocks).toEqual([{ id: "card-1", kind: "card", v: 1, title: "Summary" }]);
  });

  it("replaces duplicate rich block ids", () => {
    const [message] = applyChatEvent(
      [
        {
          ...baseMessage,
          richBlocks: [{ id: "card-1", kind: "card", v: 1, title: "Before" }],
        },
      ],
      {
        type: "rich_block",
        sessionId: "session-1",
        messageId: "assistant-1",
        block: { id: "card-1", kind: "card", v: 1, title: "After", tone: "success" },
      },
    );

    expect(message?.richBlocks).toEqual([{ id: "card-1", kind: "card", v: 1, title: "After", tone: "success" }]);
  });
});
