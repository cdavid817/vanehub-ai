import { describe, expect, it } from "vitest";
import { buildSeatContext } from "./seat-context";

const turns = [
  { speaker: "架构师", content: "方案是走 JWT。" },
  { speaker: "实现者", content: "已经实现完了。" },
];

describe("buildSeatContext", () => {
  // Resuming is the cheap path: the Agent's own session already holds the history, so re-injecting
  // it would pay for the same context twice.
  it("injects nothing when the seat's provider session can be resumed", () => {
    const context = buildSeatContext({ providerSessionId: "sess-1", turns, maxChars: 1000 });
    expect(context).toEqual({ mode: "resume", text: "" });
  });

  it("injects attributed prior turns when there is no session to resume", () => {
    const context = buildSeatContext({ providerSessionId: null, turns, maxChars: 1000 });
    expect(context.mode).toBe("inject");
    expect(context.text).toContain("[架构师 说] 方案是走 JWT。");
    expect(context.text).toContain("[实现者 说] 已经实现完了。");
  });

  // A seat added mid-session has no history of its own, so this is how it learns what happened.
  it("keeps the most recent turns when the budget is tight", () => {
    const context = buildSeatContext({ providerSessionId: null, turns, maxChars: 30 });
    expect(context.text).toContain("实现者");
    expect(context.text).not.toContain("架构师");
    expect(context.text.length).toBeLessThanOrEqual(30);
  });

  it("injects nothing when there are no prior turns", () => {
    const context = buildSeatContext({ providerSessionId: null, turns: [], maxChars: 1000 });
    expect(context).toEqual({ mode: "inject", text: "" });
  });
});
