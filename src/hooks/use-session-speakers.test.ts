import { describe, expect, it } from "vitest";
import type { AgentRegistryEntry, SessionSeat } from "../types/agent";
import type { ExpertRole } from "../types/expert-role";
import { sessionSpeakers } from "./use-session-speakers";

function role(id: string, displayName: string, color: string): ExpertRole {
  return {
    id,
    displayName,
    avatar: "🧭",
    color,
    responsibility: `${displayName}的职责`,
    instruction: `你是${displayName}。`,
    skillIds: [],
    reviewPolicy: { peerReviewer: false, requireDifferentFamily: false },
    preferredProviders: [],
    origin: "user",
    createdAt: "2026-08-07T00:00:00Z",
    updatedAt: "2026-08-07T00:00:00Z",
  };
}

function agent(id: string, displayName: string): AgentRegistryEntry {
  return {
    id,
    displayName,
    provider: "Anthropic",
    supportedInteractionModes: ["cli"],
    availabilityState: "available",
    unavailableReason: null,
    capabilityTags: [],
    launch: { kind: "cli", command: id, args: [], detectionKey: id },
    managedSdkDependencyId: null,
    toolTrustEnabled: false,
  } as unknown as AgentRegistryEntry;
}

const seats: SessionSeat[] = [
  { agentId: "claude-code", roleId: "role-architect" },
  { agentId: "codex-cli", roleId: "role-reviewer" },
];
const roles = [role("role-architect", "架构师", "#336699"), role("role-reviewer", "代码审查", "#996633")];
const agents = [agent("claude-code", "Claude Code"), agent("codex-cli", "Codex CLI")];

describe("sessionSpeakers", () => {
  it("resolves each seat to its role identity", () => {
    const speakers = sessionSpeakers({ agents, roles, seats });
    expect(speakers.get(0)).toMatchObject({
      roleName: "架构师",
      agentName: "Claude Code",
      color: "#336699",
    });
    expect(speakers.get(1)).toMatchObject({ roleName: "代码审查", agentName: "Codex CLI" });
  });

  // A single-seat session must render exactly as it did before seats existed.
  it("resolves nothing for a one-seat session", () => {
    const speakers = sessionSpeakers({
      agents,
      roles,
      seats: [{ agentId: "claude-code", roleId: null }],
    });
    expect(speakers.size).toBe(0);
  });

  it("resolves nothing when there are no seats", () => {
    expect(sessionSpeakers({ agents, roles, seats: [] }).size).toBe(0);
  });

  // Removing a seat mid-session leaves its messages behind; they must not break the thread.
  it("has no entry for a seat index that no longer exists", () => {
    const speakers = sessionSpeakers({ agents, roles, seats });
    expect(speakers.get(5)).toBeUndefined();
  });
});
