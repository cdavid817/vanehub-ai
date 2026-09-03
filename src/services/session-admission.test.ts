import { describe, expect, it } from "vitest";
import type { Session } from "../types/agent";
import type { ChatMessage } from "../types/chat";
import { canSendToSession, hasLiveSessionGeneration, sessionSendBlockReason } from "./session-admission";

function session(overrides: Partial<Session> = {}): Session {
  return {
    id: "session-1",
    title: "Session",
    agentId: "onepiece",
    interactionMode: "api",
    personalizationMode: "standard", lifecycleState: "idle",
    recoveryStatus: "clean",
    recoveryRevision: 0,
    stateRevision: 0,
    historyRevision: 0,
    activeExecutionRunId: null,
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
    createdAt: "2026-08-09T00:00:00Z",
    updatedAt: "2026-08-09T00:00:00Z",
    ...overrides,
  };
}

describe("session admission", () => {
  it.each(["idle", "failed", "stopped"] as const)("allows a clean unclaimed %s session", (lifecycleState) => {
    expect(canSendToSession(session({ lifecycleState }))).toBe(true);
  });

  it.each(["reconciling", "action_required", "quarantined"] as const)(
    "blocks a %s recovery state",
    (recoveryStatus) => {
      expect(sessionSendBlockReason(session({ recoveryStatus }))).toBe("recovery");
    },
  );

  it("blocks archived and actively claimed sessions", () => {
    expect(sessionSendBlockReason(session({ archived: true }))).toBe("archived");
    expect(sessionSendBlockReason(session({ activeExecutionRunId: "run-1" }))).toBe("active-execution");
  });

  it("shows stop only when a streaming message belongs to the active execution claim", () => {
    const streamingMessage = {
      id: "message-1",
      sessionId: "session-1",
      role: "assistant",
      content: "partial content",
      status: "streaming",
      createdAt: "2026-08-09T00:00:00Z",
      updatedAt: "2026-08-09T00:00:00Z",
      sessionSequence: 1,
      executionRunId: "run-1",
    } satisfies ChatMessage;

    expect(hasLiveSessionGeneration(session({ activeExecutionRunId: "run-1" }), [streamingMessage])).toBe(true);
    expect(hasLiveSessionGeneration(session({ activeExecutionRunId: null }), [streamingMessage])).toBe(false);
    expect(hasLiveSessionGeneration(session({ activeExecutionRunId: "run-2" }), [streamingMessage])).toBe(false);
  });
});
