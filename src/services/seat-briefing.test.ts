import { describe, expect, it } from "vitest";
import { buildSeatBriefing } from "./seat-briefing";

const architect = {
  mention: "架构师",
  roleName: "架构师",
  agentName: "Claude Code",
  modelFamily: "anthropic" as const,
  responsibility: "负责系统设计与技术选型",
  instruction: "你是本次协作会话中的架构师。",
};
const reviewer = {
  mention: "代码审查",
  roleName: "代码审查",
  agentName: "Codex CLI",
  modelFamily: "openai" as const,
  responsibility: "负责审查改动的正确性与安全性",
  instruction: "你是本次协作会话中的代码审查者。",
};

describe("buildSeatBriefing", () => {
  it("leads with the seat's own role instruction", () => {
    const text = buildSeatBriefing({ self: architect, others: [reviewer], maxMentions: 2, maxDepth: 15 });
    expect(text.startsWith("你是本次协作会话中的架构师。")).toBe(true);
  });

  // Without the roster an Agent cannot hand off: it does not know a teammate exists.
  it("publishes every other seat with its mention, responsibility, and family", () => {
    const text = buildSeatBriefing({ self: architect, others: [reviewer], maxMentions: 2, maxDepth: 15 });
    expect(text).toContain("@代码审查");
    expect(text).toContain("负责审查改动的正确性与安全性");
    expect(text).toContain("Codex CLI");
    expect(text).toContain("openai");
  });

  it("never lists the seat itself in the roster", () => {
    const text = buildSeatBriefing({ self: architect, others: [reviewer], maxMentions: 2, maxDepth: 15 });
    expect(text).not.toContain("@架构师");
  });

  // The line-leading rule is the whole routing contract; an Agent that does not know it will
  // mention teammates mid-sentence and nothing will happen.
  it("states the line-leading rule and the bounds", () => {
    const text = buildSeatBriefing({ self: architect, others: [reviewer], maxMentions: 2, maxDepth: 15 });
    expect(text).toContain("行首");
    expect(text).toContain("2");
    expect(text).toContain("15");
  });

  it("says so plainly when the seat is working alone", () => {
    const text = buildSeatBriefing({ self: architect, others: [], maxMentions: 2, maxDepth: 15 });
    expect(text).toContain("你是这个会话里唯一的参与者");
    // No teammate to hand off to — but handing back to the human is still available, so assert on
    // the roster's absence rather than on the absence of any "@" at all.
    expect(text).not.toContain("@代码审查");
    expect(text).not.toContain("本次会话的其他参与者");
  });

  it("explains how to hand back to the human with an intent", () => {
    const text = buildSeatBriefing({ self: architect, others: [reviewer], maxMentions: 2, maxDepth: 15 });
    expect(text).toContain("@用户");
    // All three intents must be described, or agents will only ever use the blocking one.
    expect(text).toContain("handoff");
    expect(text).toContain("fyi");
    expect(text).toContain("done");
  });
});
