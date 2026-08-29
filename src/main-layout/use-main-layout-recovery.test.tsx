// @vitest-environment jsdom

import { act, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NotificationProvider } from "../notifications/notification-provider";
import type { SessionRecoverySummary, SessionStateEvent } from "../services/agent-service";
import { renderWithAppProviders } from "../test/render";
import type { Session } from "../types/agent";
import type { ChatMessage } from "../types/chat";

const service = vi.hoisted(() => ({
  acknowledgeSessionRecovery: vi.fn(),
  getActiveSession: vi.fn(),
  getSessionChatConfig: vi.fn(),
  getSessionRecoverySummary: vi.fn(),
  listAgents: vi.fn(),
  listArchivedSessions: vi.fn(),
  listMessages: vi.fn(),
  listSessionCategories: vi.fn(),
  listSessionDocuments: vi.fn(),
  listSessions: vi.fn(),
  saveSessionChatConfig: vi.fn(),
  sendMessage: vi.fn(),
  subscribeMessageEvents: vi.fn(),
  subscribeSessionEvents: vi.fn(),
}));

vi.mock("../services/runtime-agent-client", () => ({ agentService: service }));
vi.mock("../services/runtime-permissions-client", () => ({
  permissionsService: { subscribePendingApprovalEvents: vi.fn().mockResolvedValue(vi.fn()) },
}));
vi.mock("../services/runtime-settings-client", () => ({
  settingsService: { reportClientLogEvent: vi.fn().mockResolvedValue(undefined) },
}));

import { useMainLayoutModel } from "./use-main-layout-model";

function session(overrides: Partial<Session> = {}): Session {
  return {
    id: "session-1",
    title: "Interrupted",
    agentId: "onepiece",
    interactionMode: "api",
    personalizationMode: "standard", lifecycleState: "failed",
    recoveryStatus: "action_required",
    recoveryRevision: 1,
    stateRevision: 2,
    historyRevision: 3,
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

function summary(value: Session): SessionRecoverySummary {
  return {
    session: value,
    latestReport: {
      reportId: `report-${value.recoveryRevision}`,
      sessionId: value.id,
      recoveryRevision: value.recoveryRevision,
      trigger: "startup",
      observedLifecycle: value.lifecycleState,
      observedExecutionRunId: null,
      decision: value.recoveryStatus === "quarantined" ? "quarantined" : "action_required",
      reasonCodes: ["unfinished_tool_activity"],
      evidenceRefs: [],
      createdAt: "2026-08-09T00:00:00Z",
    },
  };
}

const partialMessage = {
  id: "message-1",
  sessionId: "session-1",
  role: "assistant",
  content: "retained partial output",
  status: "failed",
  createdAt: "2026-08-09T00:00:00Z",
  updatedAt: "2026-08-09T00:00:00Z",
  sessionSequence: 1,
  executionRunId: "run-1",
} satisfies ChatMessage;

function RecoveryHarness() {
  const model = useMainLayoutModel();
  return (
    <div>
      <output data-testid="status">{model.activeSession?.recoveryStatus ?? "loading"}</output>
      <output data-testid="revision">{model.activeSession?.recoveryRevision ?? -1}</output>
      <output data-testid="stoppable">{String(model.isStreaming)}</output>
      <output>{model.messages.map((message) => message.content).join("|")}</output>
      <input aria-label="draft" onChange={(event) => model.setDraft(event.target.value)} value={model.draft} />
      <button onClick={model.submit} type="button">send</button>
      <button onClick={() => { void model.acknowledgeRecovery().catch(() => undefined); }} type="button">acknowledge</button>
    </div>
  );
}

describe("useMainLayoutModel recovery refresh", () => {
  let currentSession: Session;
  let sessionEventHandler: ((event: SessionStateEvent) => void) | null;

  beforeEach(() => {
    currentSession = session();
    sessionEventHandler = null;
    Object.values(service).forEach((mock) => mock.mockReset());
    service.listAgents.mockResolvedValue([]);
    service.listSessions.mockImplementation(async () => [currentSession]);
    service.listArchivedSessions.mockResolvedValue([]);
    service.listSessionCategories.mockResolvedValue([]);
    service.getActiveSession.mockImplementation(async () => currentSession);
    service.getSessionRecoverySummary.mockImplementation(async () => summary(currentSession));
    service.listMessages.mockResolvedValue([partialMessage]);
    service.listSessionDocuments.mockResolvedValue({
      context: { availability: "unavailable", rootName: null, reason: null },
      items: [],
      truncated: false,
      nextCursor: null,
    });
    service.getSessionChatConfig.mockRejectedValue(new Error("No persisted config"));
    service.saveSessionChatConfig.mockResolvedValue(undefined);
    service.subscribeMessageEvents.mockResolvedValue(vi.fn());
    service.subscribeSessionEvents.mockImplementation(async (handler: (event: SessionStateEvent) => void) => {
      sessionEventHandler = handler;
      return vi.fn();
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  function renderHarness() {
    return renderWithAppProviders(
      <NotificationProvider><RecoveryHarness /></NotificationProvider>,
    );
  }

  it("loads authoritative recovery without an event, preserves content, and blocks send and stop", async () => {
    const intervalSpy = vi.spyOn(globalThis, "setInterval");
    const { user } = renderHarness();

    await waitFor(() => expect(screen.getByTestId("status").textContent).toBe("action_required"));
    expect(intervalSpy).not.toHaveBeenCalledWith(expect.any(Function), 5_000);
    await waitFor(() => expect(screen.getByText(partialMessage.content)).toBeTruthy());
    expect(screen.getByTestId("stoppable").textContent).toBe("false");
    await user.type(screen.getByRole("textbox", { name: "draft" }), "follow up");
    await user.click(screen.getByRole("button", { name: "send" }));
    expect(service.sendMessage).not.toHaveBeenCalled();
  });

  it("polls only transient reconciliation and skips recovery summaries for clean sessions", async () => {
    const intervalSpy = vi.spyOn(globalThis, "setInterval");
    currentSession = session({ recoveryStatus: "reconciling" });
    const first = renderHarness();

    await waitFor(() => expect(screen.getByTestId("status").textContent).toBe("reconciling"));
    expect(intervalSpy).toHaveBeenCalledWith(expect.any(Function), 5_000);
    expect(service.getSessionRecoverySummary).toHaveBeenCalledTimes(1);
    first.unmount();

    intervalSpy.mockClear();
    service.getSessionRecoverySummary.mockClear();
    currentSession = session({ recoveryStatus: "clean" });
    renderHarness();

    await waitFor(() => expect(screen.getByTestId("status").textContent).toBe("clean"));
    expect(intervalSpy).not.toHaveBeenCalledWith(expect.any(Function), 5_000);
    expect(service.getSessionRecoverySummary).not.toHaveBeenCalled();
  });

  it("refreshes authoritative state after a stale acknowledgement and a revision-gap event", async () => {
    service.acknowledgeSessionRecovery.mockImplementation(async () => {
      currentSession = session({ recoveryRevision: 2 });
      throw new Error("recovery revision conflict: current revision is 2");
    });
    const { user } = renderHarness();
    await waitFor(() => expect(screen.getByTestId("revision").textContent).toBe("1"));

    await user.click(screen.getByRole("button", { name: "acknowledge" }));
    await waitFor(() => expect(screen.getByTestId("revision").textContent).toBe("2"));

    currentSession = session({ recoveryStatus: "quarantined", recoveryRevision: 5 });
    act(() => sessionEventHandler?.({
      kind: "recovery-quarantined",
      sessionId: currentSession.id,
      recoveryRevision: 5,
    }));
    await waitFor(() => expect(screen.getByTestId("revision").textContent).toBe("5"));
    expect(screen.getByTestId("status").textContent).toBe("quarantined");
    expect(service.listSessionCategories).toHaveBeenCalledTimes(1);
    expect(service.listArchivedSessions).toHaveBeenCalledTimes(1);
  });
});
