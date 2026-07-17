import { describe, expect, it } from "vitest";
import type { ChatMessage } from "../types/chat";
import { aggregateSessionReport } from "./report-utils";

function message(overrides: Partial<ChatMessage>): ChatMessage {
  return {
    id: "message-1",
    sessionId: "session-1",
    role: "assistant",
    content: "response",
    status: "completed",
    createdAt: "2026-07-17T00:00:00.000Z",
    updatedAt: "2026-07-17T00:00:01.000Z",
    ...overrides,
  };
}

describe("session report aggregation", () => {
  it("keeps reported tokens separate from estimated characters and ranks tools", () => {
    const report = aggregateSessionReport([
      message({ id: "user", role: "user", content: "hello" }),
      message({ tokenUsage: { input: 10, output: 20 }, toolUse: [{ id: "a", name: "read", status: "completed" }] }),
      message({ id: "failed", status: "failed", toolUse: [{ id: "b", name: "read", status: "failed" }] }),
    ]);
    expect(report.reportedInputTokens).toBe(10);
    expect(report.reportedOutputTokens).toBe(20);
    expect(report.estimatedInputCharacters).toBe(5);
    expect(report.toolRanking[0]).toEqual({ name: "read", count: 2 });
    expect(report.failedCount).toBe(1);
    expect(report.statusCounts).toEqual({ pending: 0, streaming: 0, completed: 2, failed: 1, cancelled: 0 });
    expect(report.timeline).toEqual(expect.arrayContaining([
      expect.objectContaining({ id: "user-completion", kind: "completion" }),
      expect.objectContaining({ id: "message-1-completion", kind: "completion" }),
    ]));
  });
});
