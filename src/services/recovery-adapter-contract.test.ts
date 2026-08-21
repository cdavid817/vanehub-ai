import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  SessionRecoveryAcknowledgement,
  SessionRecoverySummary,
  SessionStateEvent,
} from "./agent-service";

const { invokeMock, listenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: listenMock }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));

import { tauriAgentClient } from "./tauri-agent-client";
import {
  resetWebRecoverySessionsForTest,
  seedWebRecoverySessionForTest,
  webAgentClient,
} from "./web-agent-client";

type NativeEventListener = (event: { payload: unknown }) => void;

function expectRecoverySummaryContract(summary: SessionRecoverySummary) {
  expect(summary.session).toMatchObject({
    recoveryStatus: "action_required",
    recoveryRevision: 1,
    activeExecutionRunId: null,
  });
  expect(summary.latestReport).toMatchObject({
    sessionId: summary.session.id,
    recoveryRevision: 1,
    trigger: "startup",
    decision: "action_required",
    reasonCodes: ["unfinished_tool_activity"],
  });
  expect(summary.latestReport?.evidenceRefs[0]).toMatchObject({
    kind: "session",
    sessionId: summary.session.id,
  });
}

describe("recovery adapter contract", () => {
  let nativeListener: NativeEventListener | null;

  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockReset();
    nativeListener = null;
    listenMock.mockImplementation((_eventName: string, listener: NativeEventListener) => {
      nativeListener = listener;
      return Promise.resolve(vi.fn());
    });
  });

  afterEach(() => resetWebRecoverySessionsForTest());

  it("exposes equivalent normalized recovery fields from desktop and Web fixtures", async () => {
    const webSession = seedWebRecoverySessionForTest("action_required");
    const webSummary = await webAgentClient.getSessionRecoverySummary(webSession.id);
    invokeMock.mockResolvedValueOnce(webSummary);

    const desktopSummary = await tauriAgentClient.getSessionRecoverySummary(webSession.id);

    expectRecoverySummaryContract(webSummary);
    expectRecoverySummaryContract(desktopSummary);
    expect(invokeMock).toHaveBeenCalledWith("get_session_recovery_summary", {
      sessionId: webSession.id,
    });
  });

  it("keeps bounded reads and acknowledgement arguments identical", async () => {
    const session = seedWebRecoverySessionForTest("action_required");
    const reports = await webAgentClient.listSessionRecoveryReports(session.id, 1);
    const acknowledgement = await webAgentClient.acknowledgeSessionRecovery(session.id, 1);
    invokeMock
      .mockResolvedValueOnce(reports)
      .mockResolvedValueOnce(acknowledgement satisfies SessionRecoveryAcknowledgement);

    await expect(tauriAgentClient.listSessionRecoveryReports(session.id, 1)).resolves.toEqual(reports);
    await expect(tauriAgentClient.acknowledgeSessionRecovery(session.id, 1)).resolves.toEqual(acknowledgement);
    expect(invokeMock.mock.calls).toEqual([
      ["list_session_recovery_reports", { sessionId: session.id, limit: 1 }],
      ["acknowledge_session_recovery", { sessionId: session.id, expectedRecoveryRevision: 1 }],
    ]);
  });

  it("recovers a stuck runtime identically in both adapters", async () => {
    const session = seedWebRecoverySessionForTest("action_required");
    const webResult = await webAgentClient.recoverSession(session.id);
    invokeMock.mockResolvedValueOnce(webResult);

    await expect(tauriAgentClient.recoverSession(session.id)).resolves.toEqual(webResult);
    expect(webResult.lifecycleState).toBe("idle");
    // Recovery restores a usable state; it never claims to have started anything.
    expect(webResult.processStopped).toBe(false);
    expect(invokeMock).toHaveBeenCalledWith("recover_session", { sessionId: session.id });

    // Idempotent: a second run finds nothing left to release and still lands on idle.
    await expect(webAgentClient.recoverSession(session.id)).resolves.toEqual({
      cancelledMessageIds: [],
      processStopped: false,
      lifecycleState: "idle",
    });
  });

  it("refuses to recover an archived session in the Web adapter", async () => {
    const session = seedWebRecoverySessionForTest("action_required");
    await webAgentClient.archiveSession(session.id);

    await expect(webAgentClient.recoverSession(session.id)).rejects.toThrow(/[Aa]rchived/);
  });

  it("rejects stale revisions in both adapters", async () => {
    const session = seedWebRecoverySessionForTest("action_required");
    const staleError = new Error("recovery revision conflict: current revision is 1");
    invokeMock.mockRejectedValueOnce(staleError);

    await expect(webAgentClient.acknowledgeSessionRecovery(session.id, 0)).rejects.toThrow("current revision is 1");
    await expect(tauriAgentClient.acknowledgeSessionRecovery(session.id, 0)).rejects.toThrow("current revision is 1");
  });

  it("delivers only normalized desktop recovery event shapes", async () => {
    const handler = vi.fn<(event: SessionStateEvent) => void>();
    await tauriAgentClient.subscribeSessionEvents(handler);

    nativeListener?.({ payload: { kind: "recovery-completed", sessionId: "session-1" } });
    nativeListener?.({
      payload: {
        kind: "recovery-completed",
        sessionId: "session-1",
        recoveryRevision: 2,
      },
    });

    expect(listenMock).toHaveBeenCalledWith("session:event", expect.any(Function));
    expect(handler).toHaveBeenCalledOnce();
    expect(handler).toHaveBeenCalledWith({
      kind: "recovery-completed",
      sessionId: "session-1",
      recoveryRevision: 2,
    });
  });
});
