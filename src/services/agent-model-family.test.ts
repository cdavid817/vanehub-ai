import { describe, expect, it } from "vitest";
import { withModelFamily } from "./agent-model-family";
import type { AgentRegistryEntry } from "../types/agent";

function agent(id: string, provider: string): AgentRegistryEntry {
  return {
    id,
    displayName: id,
    provider,
    launch: { kind: "cli", command: id },
    supportedInteractionModes: ["cli"],
    availabilityState: "available",
    capabilityTags: [],
    agentOrigin: "builtin",
  };
}

describe("withModelFamily", () => {
  it("annotates each agent without dropping its existing fields", () => {
    const [annotated] = withModelFamily([agent("claude-code", "Anthropic")]);
    expect(annotated.modelFamily).toBe("anthropic");
    expect(annotated.displayName).toBe("claude-code");
    expect(annotated.availabilityState).toBe("available");
  });

  it("annotates agents of different families distinctly", () => {
    const annotated = withModelFamily([
      agent("claude-code", "Anthropic"),
      agent("codex-cli", "OpenAI"),
      agent("opencode", "OpenCode"),
    ]);
    expect(annotated.map((entry) => entry.modelFamily)).toEqual(["anthropic", "openai", "unknown"]);
  });
});
