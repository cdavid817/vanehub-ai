import { describe, expect, it } from "vitest";
import { recommendReviewerAgents } from "./reviewer-recommendation";
import type { AgentWithModelFamily } from "./agent-model-family";

function agent(id: string, modelFamily: AgentWithModelFamily["modelFamily"]): AgentWithModelFamily {
  return {
    id,
    displayName: id,
    provider: id,
    launch: { kind: "cli", command: id },
    supportedInteractionModes: ["cli"],
    availabilityState: "available",
    capabilityTags: [],
    agentOrigin: "builtin",
    modelFamily,
  };
}

const claude = agent("claude-code", "anthropic");
const codex = agent("codex-cli", "openai");
const gemini = agent("gemini-cli", "google");
const opencode = agent("opencode", "unknown");

describe("recommendReviewerAgents", () => {
  it("prefers agents from a different family than the one under review", () => {
    const result = recommendReviewerAgents([claude, codex, gemini], "claude-code");
    expect(result.agents.map((entry) => entry.id)).toEqual(["codex-cli", "gemini-cli"]);
    expect(result.degraded).toBe(false);
  });

  // Regression guard: a strict version left users unable to assign any reviewer.
  it("falls back to same-family agents with an explicit notice when no cross-family agent exists", () => {
    const anotherAnthropic = agent("claude-alt", "anthropic");
    const result = recommendReviewerAgents([claude, anotherAnthropic], "claude-code");
    expect(result.agents.map((entry) => entry.id)).toEqual(["claude-alt"]);
    expect(result.degraded).toBe(true);
  });

  it("never recommends the agent under review", () => {
    const result = recommendReviewerAgents([claude, codex], "claude-code");
    expect(result.agents.map((entry) => entry.id)).not.toContain("claude-code");
  });

  // An unknown family cannot be ruled out as same-family, so it stays a genuine cross-family option.
  it("treats an unknown family as an acceptable cross-family reviewer", () => {
    const result = recommendReviewerAgents([claude, opencode], "claude-code");
    expect(result.agents.map((entry) => entry.id)).toEqual(["opencode"]);
    expect(result.degraded).toBe(false);
  });

  it("returns nothing when the only agent is the one under review", () => {
    const result = recommendReviewerAgents([claude], "claude-code");
    expect(result.agents).toEqual([]);
    expect(result.degraded).toBe(false);
  });

  it("excludes unavailable agents", () => {
    const offline = { ...codex, availabilityState: "unavailable" as const };
    const result = recommendReviewerAgents([claude, offline], "claude-code");
    expect(result.agents).toEqual([]);
  });
});
