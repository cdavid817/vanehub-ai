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

  it("preserves references for non-target messages so memoized rows skip re-render", () => {
    const earlier: ChatMessage = { ...baseMessage, id: "assistant-0", content: "earlier" };
    const target: ChatMessage = { ...baseMessage, id: "assistant-1" };

    const next = applyChatEvent([earlier, target], {
      type: "token",
      sessionId: "session-1",
      messageId: "assistant-1",
      contentDelta: "hi",
    });

    // Unchanged message keeps its identity — this is what lets React.memo(MessageItem)
    // avoid re-parsing markdown/mermaid for every historical message on every token.
    expect(next[0]).toBe(earlier);
    // Only the streaming target is rebuilt, with the token appended.
    expect(next[1]).not.toBe(target);
    expect(next[1]?.content).toBe("hi");
  });
});
