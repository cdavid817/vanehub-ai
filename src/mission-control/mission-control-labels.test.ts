import { describe, expect, it } from "vitest";
import type { AgentRegistryEntry } from "../types/agent";
import { resolveAgentDisplayName } from "./mission-control-labels";

const agent = (id: string, displayName: string): AgentRegistryEntry => ({
  id, displayName, provider: "test", launch: { kind: "cli" }, supportedInteractionModes: ["cli"],
  availabilityState: "available", capabilityTags: [], agentOrigin: "user",
});

describe("resolveAgentDisplayName", () => {
  it("returns null when there is no agentId at all -- distinct from 'no match found'", () => {
    expect(resolveAgentDisplayName([agent("claude-code", "Claude Code")], null)).toBeNull();
  });

  it("resolves a matching registry entry to its own display name", () => {
    const agents = [agent("claude-code", "Claude Code"), agent("codex-cli", "Codex CLI")];
    expect(resolveAgentDisplayName(agents, "codex-cli")).toBe("Codex CLI");
  });

  it("falls back to the raw id -- honestly, not silently -- when no registry entry matches", () => {
    // A real, expected case: `MissionControlRunSummary.agentId` is not reliably a real registry id
    // even when present (see this module's own doc comment for why), so callers must be able to
    // tell "resolved" apart from "shown raw" without the id being hidden or blanked out.
    expect(resolveAgentDisplayName([agent("claude-code", "Claude Code")], "unregistered-owner-id")).toBe("unregistered-owner-id");
  });

  it("falls back to the raw id when the registry is empty", () => {
    expect(resolveAgentDisplayName([], "claude-code")).toBe("claude-code");
  });
});
