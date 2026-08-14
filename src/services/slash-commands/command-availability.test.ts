import { describe, expect, it } from "vitest";
import type { Session } from "../../types/agent";
import { isMultiSeatCliSession, isOnePieceSession, slashCommandsEnabled } from "./command-availability";

const session = (overrides: Partial<Session>): Session => ({
  id: "session-1", title: "Session", agentId: "onepiece", interactionMode: "api",
  lifecycleState: "idle", recoveryStatus: "healthy", recoveryRevision: 0, stateRevision: 0,
  historyRevision: 0, activeExecutionRunId: null, folder: null, projectPath: null,
  worktreePath: null, worktreeName: null, worktreeBranch: null, remoteWorkspace: null,
  remoteSshConnectionId: null, remoteSshConnectionRevision: null, runtimeSessionId: null,
  categoryId: null, pinned: false, archived: false,
  createdAt: "2026-08-14T00:00:00Z", updatedAt: "2026-08-14T00:00:00Z",
  ...overrides,
} as Session);

describe("slash command availability", () => {
  it("recognises a OnePiece session", () => {
    expect(isOnePieceSession(session({ agentId: "onepiece" }))).toBe(true);
    expect(isOnePieceSession(session({ agentId: "claude-code" }))).toBe(false);
  });

  it("recognises a multi-seat CLI session", () => {
    const seats = [{ agentId: "claude-code", roleId: null }, { agentId: "codex-cli", roleId: null }];
    expect(isMultiSeatCliSession(session({ agentId: "claude-code", interactionMode: "cli", seats }))).toBe(true);
    expect(isMultiSeatCliSession(session({ agentId: "claude-code", interactionMode: "cli" }))).toBe(false);
    expect(isMultiSeatCliSession(session({ agentId: "onepiece", interactionMode: "api", seats }))).toBe(false);
  });

  it("ignores seats that have already left", () => {
    const seats = [
      { agentId: "claude-code", roleId: null },
      { agentId: "codex-cli", roleId: null, leftAt: "2026-08-14T00:00:00Z" },
    ];
    expect(isMultiSeatCliSession(session({ interactionMode: "cli", seats }))).toBe(false);
  });

  it("enables commands for OnePiece only in this phase", () => {
    const seats = [{ agentId: "claude-code", roleId: null }, { agentId: "codex-cli", roleId: null }];
    expect(slashCommandsEnabled(session({ agentId: "onepiece" }))).toBe(true);
    expect(slashCommandsEnabled(session({ agentId: "claude-code", interactionMode: "cli", seats }))).toBe(false);
    expect(slashCommandsEnabled(null)).toBe(false);
  });
});
