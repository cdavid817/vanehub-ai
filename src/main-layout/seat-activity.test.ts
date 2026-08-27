import { describe, expect, it } from "vitest";
import type { ChatMessage } from "../types/chat";
import { seatActivity } from "./seat-activity";

const message = (patch: Partial<ChatMessage> = {}): ChatMessage => ({
  id: "message-1", sessionId: "session-1", role: "assistant", speakerSeatId: "seat-1",
  content: "", status: "streaming", createdAt: "2026-08-27T00:00:00Z",
  updatedAt: "2026-08-27T00:00:00Z", sessionSequence: 1, executionRunId: "run-1", ...patch,
});

describe("seatActivity", () => {
  it("distinguishes startup, thinking, tool activity, and streamed output", () => {
    expect(seatActivity([], "seat-1", true)).toBe("starting");
    expect(seatActivity([message({ thinkingContent: "plan" })], "seat-1", true)).toBe("thinking");
    expect(seatActivity([message({ toolUse: [{ id: "tool-1", name: "Read", status: "running" }] })], "seat-1", true)).toBe("tool");
    expect(seatActivity([message({ content: "partial" })], "seat-1", true)).toBe("streaming");
    expect(seatActivity([message({ content: "web partial" })], "seat-1", false)).toBe("streaming");
  });

  it("shows stable terminal state for the attributed member", () => {
    expect(seatActivity([message({ status: "completed" })], "seat-1", false)).toBe("completed");
    expect(seatActivity([message({ status: "failed" })], "seat-1", false)).toBe("failed");
    expect(seatActivity([message()], "seat-2", false)).toBe("idle");
  });
});
