import { describe, expect, it } from "vitest";
import { deriveSessionPresentation, type DeriveSessionPresentationInput } from "./session-presentation";
import type { Session } from "../types/agent";

function session(overrides: Partial<Session> = {}): Session {
  return {
    id: "session-1",
    title: "CLI work",
    agentId: "claude-code",
    interactionMode: "cli",
    personalizationMode: "standard",
    lifecycleState: "running",
    recoveryStatus: "clean",
    recoveryRevision: 0,
    stateRevision: 0,
    historyRevision: 0,
    activeExecutionRunId: null,
    folder: "D:\\code\\vanehub-ai",
    projectPath: "D:\\code\\vanehub-ai",
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
    createdAt: "2026-08-27T00:00:00.000Z",
    updatedAt: "2026-08-27T00:00:00.000Z",
    ...overrides,
  };
}

function input(overrides: Partial<DeriveSessionPresentationInput> = {}): DeriveSessionPresentationInput {
  return {
    session: session(),
    isStreaming: false,
    isSending: false,
    turnStatus: null,
    recoverySummary: null,
    acknowledgingRecovery: false,
    messageCount: 0,
    messagesPartial: false,
    messagesLoading: false,
    ...overrides,
  };
}

describe("deriveSessionPresentation", () => {
  it("reports no-session lifecycle and a none primary action when nothing is open", () => {
    const presentation = deriveSessionPresentation(input({ session: null }));
    expect(presentation.lifecycle).toBe("no-session");
    expect(presentation.primaryAction).toEqual({ kind: "none" });
    expect(presentation.recovery).toBeNull();
  });

  it("offers send when nothing blocks it and nothing is streaming", () => {
    const presentation = deriveSessionPresentation(input());
    expect(presentation.primaryAction).toEqual({ kind: "send" });
  });

  it("offers stop whenever streaming, even if a block reason would otherwise apply", () => {
    const presentation = deriveSessionPresentation(input({
      isStreaming: true,
      session: session({ archived: true }),
    }));
    expect(presentation.primaryAction).toEqual({ kind: "stop" });
  });

  it("offers recover, carrying the acknowledging flag, while recovery is not clean", () => {
    const presentation = deriveSessionPresentation(input({
      acknowledgingRecovery: true,
      session: session({ recoveryStatus: "action_required" }),
    }));
    expect(presentation.primaryAction).toEqual({ kind: "recover", acknowledging: true });
    expect(presentation.recovery).toEqual({ acknowledging: true, status: "action_required", summary: null });
  });

  it("reports blocked/archived for an archived session that is not streaming", () => {
    const presentation = deriveSessionPresentation(input({ session: session({ archived: true }) }));
    expect(presentation.primaryAction).toEqual({ kind: "blocked", reason: "archived" });
  });

  it("reports blocked/active-execution for the gap where a run exists but nothing is streaming yet", () => {
    // A real, pre-existing gap in the app's own submit()/stop() guards: an execution run has
    // started but no message has status "streaming" for it yet, so neither send nor stop applies.
    const presentation = deriveSessionPresentation(input({
      isStreaming: false,
      session: session({ activeExecutionRunId: "run-1" }),
    }));
    expect(presentation.primaryAction).toEqual({ kind: "blocked", reason: "active-execution" });
  });

  it("passes participantTurn and messageState through without reinterpreting them", () => {
    const presentation = deriveSessionPresentation(input({
      messageCount: 42,
      messagesLoading: true,
      messagesPartial: true,
      turnStatus: { depth: 1, holderName: "Claude", kind: "agent", maxDepth: 3 },
    }));
    expect(presentation.participantTurn).toEqual({ depth: 1, holderName: "Claude", kind: "agent", maxDepth: 3 });
    expect(presentation.messageState).toEqual({ count: 42, loading: true, partial: true });
  });

  it("keeps recovery null once acknowledged back to clean", () => {
    const presentation = deriveSessionPresentation(input({ session: session({ recoveryStatus: "clean" }) }));
    expect(presentation.recovery).toBeNull();
  });
});
