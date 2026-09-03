import { describe, expect, it } from "vitest";
import type { AgentRegistryEntry } from "../types/agent";
import { scheduledTaskAgentCapability } from "./scheduled-task-agent-capability";

function buildAgent(overrides: Partial<AgentRegistryEntry> = {}): AgentRegistryEntry {
  return { id: "codex-cli", displayName: "Codex CLI", supportedInteractionModes: ["cli"], availabilityState: "available", ...overrides } as AgentRegistryEntry;
}

describe("scheduledTaskAgentCapability", () => {
  it("returns null (no capability problem) when the agent is present and available", () => {
    expect(scheduledTaskAgentCapability(buildAgent({ availabilityState: "available" }))).toBeNull();
  });

  it("returns 'missing' when the agent id is not in the registry at all", () => {
    expect(scheduledTaskAgentCapability(undefined)).toEqual({ reason: "missing" });
  });

  // 19.6: distinct from "missing" -- the agent is still a known registry entry, just not currently
  // usable, which every non-"available" AvailabilityState represents the same way.
  it.each(["unavailable", "needs-auth", "unknown"] as const)("returns 'unavailable' when the agent's own availabilityState is %s", (availabilityState) => {
    expect(scheduledTaskAgentCapability(buildAgent({ availabilityState }))).toEqual({ reason: "unavailable" });
  });
});
