import { describe, expect, it } from "vitest";
import { seatsFromSession, sessionAgentIdFromSeats } from "./session-seats";
import type { Session, SessionSeat } from "../types/agent";

function session(overrides: Partial<Session> = {}): Session {
  return {
    id: "s1",
    title: "会话",
    agentId: "claude-code",
    interactionMode: "cli",
    lifecycleState: "idle",
    folder: null,
    projectPath: null,
    worktreePath: null,
    worktreeName: null,
    worktreeBranch: null,
    remoteWorkspace: null,
    remoteSshConnectionId: null,
    remoteSshConnectionRevision: null,
    runtimeSessionId: null,
    categoryId: null,
    pinned: false,
    archived: false,
    createdAt: "2026-08-06T00:00:00Z",
    updatedAt: "2026-08-06T00:00:00Z",
    ...overrides,
  };
}

const seat = (agentId: string, roleId: string | null = null): SessionSeat => ({ agentId, roleId });

describe("seatsFromSession", () => {
  // Sessions persisted before seats existed carry no seat list; presenting them as a one-seat
  // session is what keeps every existing session readable.
  it("presents a pre-seat session as one seat with no role", () => {
    expect(seatsFromSession(session({ seats: undefined }))).toEqual([
      { agentId: "claude-code", roleId: null },
    ]);
  });

  it("returns the stored seats when present, preserving order", () => {
    const seats = [seat("claude-code", "builtin-architect"), seat("codex-cli", "builtin-reviewer")];
    expect(seatsFromSession(session({ seats, agentId: "claude-code" }))).toEqual(seats);
  });

  it("still yields one seat when the stored list is empty", () => {
    expect(seatsFromSession(session({ seats: [] }))).toEqual([
      { agentId: "claude-code", roleId: null },
    ]);
  });
});

describe("sessionAgentIdFromSeats", () => {
  // The record's agentId mirrors seat 0 so the ~148 existing readers keep working untouched.
  it("mirrors the first seat", () => {
    expect(sessionAgentIdFromSeats([seat("codex-cli"), seat("claude-code")])).toBe("codex-cli");
  });

  it("falls back to the provided default when there are no seats", () => {
    expect(sessionAgentIdFromSeats([], "gemini-cli")).toBe("gemini-cli");
  });
});
