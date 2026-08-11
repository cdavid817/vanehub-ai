import { describe, expect, it } from "vitest";
import { resolveMessageSpeaker } from "./message-speaker";
import type { ExpertRole } from "../types/expert-role";
import type { AgentRegistryEntry, SessionSeat } from "../types/agent";

const role = (id: string, displayName: string, extra: Partial<ExpertRole> = {}): ExpertRole => ({
  id,
  displayName,
  avatar: "🔍",
  color: "#C77D3A",
  responsibility: "审查",
  instruction: "…",
  skillIds: [],
  reviewPolicy: { peerReviewer: true, requireDifferentFamily: true },
  preferredProviders: [],
  origin: "builtin",
  createdAt: "t",
  updatedAt: "t",
  ...extra,
});

const agent = (id: string, displayName: string): AgentRegistryEntry => ({
  id,
  displayName,
  provider: id,
  launch: { kind: "cli", command: id },
  supportedInteractionModes: ["cli"],
  availabilityState: "available",
  capabilityTags: [],
  agentOrigin: "builtin",
});

const seats: SessionSeat[] = [
  { agentId: "claude-code", roleId: "builtin-architect" },
  { agentId: "codex-cli", roleId: "builtin-reviewer" },
];
const roles = [role("builtin-architect", "架构师"), role("builtin-reviewer", "代码审查")];
const agents = [agent("claude-code", "Claude Code"), agent("codex-cli", "Codex CLI")];

describe("resolveMessageSpeaker", () => {
  it("resolves a seat into its role identity and Agent name", () => {
    const speaker = resolveMessageSpeaker({ seatIndex: 1, seats, roles, agents });
    expect(speaker).toEqual({
      agentId: "codex-cli",
      avatar: "🔍",
      color: "#C77D3A",
      roleName: "代码审查",
      agentName: "Codex CLI",
      crossFamilyReviewer: true,
    });
  });

  // A seat may carry no role; it is still a distinct speaker and must not render as anonymous.
  it("falls back to the Agent alone when the seat has no role", () => {
    const speaker = resolveMessageSpeaker({
      seatIndex: 0,
      seats: [{ agentId: "claude-code", roleId: null }],
      roles,
      agents,
    });
    expect(speaker?.roleName).toBeNull();
    expect(speaker?.agentName).toBe("Claude Code");
    expect(speaker?.crossFamilyReviewer).toBe(false);
  });

  // Messages predating seats carry no speaker; single-Agent sessions must keep their old rendering.
  it("returns null when there is no seat index", () => {
    expect(resolveMessageSpeaker({ seatIndex: undefined, seats, roles, agents })).toBeNull();
  });

  it("returns null for a seat index that no longer exists", () => {
    // A seat removed mid-session leaves its messages behind; they must not crash the thread.
    expect(resolveMessageSpeaker({ seatIndex: 7, seats, roles, agents })).toBeNull();
  });

  it("still resolves when the role was deleted from settings", () => {
    const speaker = resolveMessageSpeaker({ seatIndex: 1, seats, roles: [], agents });
    expect(speaker?.roleName).toBeNull();
    expect(speaker?.agentName).toBe("Codex CLI");
  });

  it("keeps captured identity after the roster order and role registry change", () => {
    const historical: SessionSeat = {
      seatId: "seat-reviewer",
      agentId: "codex-cli",
      roleId: "deleted-role",
      leftAt: "2026-08-10T01:00:00Z",
      roleSnapshot: {
        roleName: "Original reviewer",
        avatar: "🧭",
        color: "#123456",
        responsibility: "Review",
        agentName: "Original Codex",
        modelFamily: "openai",
        crossFamilyReviewer: true,
      },
    };
    const speaker = resolveMessageSpeaker({
      speakerSeatId: "seat-reviewer",
      seatIndex: 0,
      seats: [seats[0], historical],
      roles: [],
      agents: [],
    });
    expect(speaker).toMatchObject({
      roleName: "Original reviewer",
      agentName: "Original Codex",
      avatar: "🧭",
      color: "#123456",
    });
  });
});
