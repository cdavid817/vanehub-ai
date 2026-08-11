import { describe, expect, it } from "vitest";
import { appendMessageIfMissing, createOptimisticUserMessage, removeMessageById } from "./optimistic-message";

describe("optimistic user messages", () => {
  it("creates an immediately renderable user message with submitted references", () => {
    const message = createOptimisticUserMessage({
      content: "Review the plan",
      fileReferences: [{ id: "readme", name: "README.md", path: "D:\\repo\\README.md" }],
      id: "optimistic:session-1:1",
      now: new Date("2026-08-11T00:00:00.000Z"),
      sessionId: "session-1",
    });
    expect(message).toMatchObject({ role: "user", status: "completed", createdAt: "2026-08-11T00:00:00.000Z" });
    expect(appendMessageIfMissing([], message)).toEqual([message]);
    expect(appendMessageIfMissing([message], message)).toEqual([message]);
  });

  it("removes only the rejected temporary message during rollback", () => {
    const optimistic = createOptimisticUserMessage({ content: "Retry me", fileReferences: [], id: "temp", sessionId: "session-1" });
    const canonical = { ...optimistic, id: "canonical" };
    expect(removeMessageById([canonical, optimistic], optimistic.id)).toEqual([canonical]);
  });
});
