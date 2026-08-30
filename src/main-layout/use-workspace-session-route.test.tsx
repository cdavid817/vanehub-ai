// @vitest-environment jsdom

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { NotificationProvider } from "../notifications/notification-provider";
import type { Session } from "../types/agent";
import { useWorkspaceSessionRoute } from "./use-workspace-session-route";
import type { WorkbenchLocation } from "./workbench-route";

afterEach(cleanup);

function session(id: string, overrides: Partial<Session> = {}): Session {
  return {
    id,
    title: id,
    agentId: "claude-code",
    interactionMode: "cli",
    personalizationMode: "standard", lifecycleState: "idle",
    recoveryStatus: "clean",
    recoveryRevision: 0,
    stateRevision: 0,
    historyRevision: 0,
    activeExecutionRunId: null,
    folder: null,
    projectPath: "D:\\code",
    worktreePath: null,
    worktreeName: null,
    worktreeBranch: null,
    remoteWorkspace: null,
    remoteSshConnectionId: null,
    remoteSshConnectionRevision: null,
    runtimeSessionId: null,
    categoryId: null,
    source: { kind: "desktop", connector: null },
    pinned: false,
    archived: false,
    createdAt: "2026-08-14T00:00:00.000Z",
    updatedAt: "2026-08-14T00:00:00.000Z",
    ...overrides,
  };
}

function sessionsLocation(overrides: Partial<Extract<WorkbenchLocation, { destination: "sessions" }>> = {}): WorkbenchLocation {
  return { creatingSession: false, destination: "sessions", sessionId: null, ...overrides };
}

function renderRoute(options: Parameters<typeof useWorkspaceSessionRoute>[0]) {
  function Probe() {
    useWorkspaceSessionRoute(options);
    return null;
  }
  return render(<NotificationProvider><Probe /></NotificationProvider>);
}

const defaults = {
  activeSessionId: null,
  archivedSessions: [] as Session[],
  onNavigate: vi.fn(),
  sessions: [] as Session[],
  switchSession: vi.fn(),
};

describe("useWorkspaceSessionRoute", () => {
  it("adopts the backend's active session into an addressless route", () => {
    const onNavigate = vi.fn();
    renderRoute({ ...defaults, activeSessionId: "session-1", location: sessionsLocation(), onNavigate });

    expect(onNavigate).toHaveBeenCalledWith(
      { creatingSession: false, destination: "sessions", sessionId: "session-1" },
      { replace: true },
    );
  });

  it("switches to the session the route names", () => {
    const switchSession = vi.fn();
    const target = session("session-2");
    renderRoute({
      ...defaults,
      activeSessionId: "session-1",
      location: sessionsLocation({ sessionId: "session-2" }),
      sessions: [session("session-1"), target],
      switchSession,
    });

    expect(switchSession).toHaveBeenCalledWith(target);
  });

  it("does nothing when the route already matches the active session", () => {
    const onNavigate = vi.fn();
    const switchSession = vi.fn();
    renderRoute({
      ...defaults,
      activeSessionId: "session-1",
      location: sessionsLocation({ sessionId: "session-1" }),
      onNavigate,
      sessions: [session("session-1")],
      switchSession,
    });

    expect(onNavigate).not.toHaveBeenCalled();
    expect(switchSession).not.toHaveBeenCalled();
  });

  /** A deep link arrives before the session list does; bouncing then would be wrong. */
  it("waits for the session list before declaring a route session missing", () => {
    const onNavigate = vi.fn();
    renderRoute({ ...defaults, location: sessionsLocation({ sessionId: "session-missing" }), onNavigate });

    expect(onNavigate).not.toHaveBeenCalled();
  });

  it("falls back once the list has arrived without the route session", () => {
    const onNavigate = vi.fn();
    renderRoute({
      ...defaults,
      location: sessionsLocation({ sessionId: "session-missing" }),
      onNavigate,
      sessions: [session("session-1")],
    });

    expect(onNavigate).toHaveBeenCalledWith(
      { creatingSession: false, destination: "sessions", sessionId: null },
      { replace: true },
    );
  });

  it("stays out of the way on other destinations and while creating", () => {
    const onNavigate = vi.fn();
    renderRoute({
      ...defaults,
      activeSessionId: "session-1",
      location: { destination: "runs", section: "attention", runId: undefined },
      onNavigate,
    });
    expect(onNavigate).not.toHaveBeenCalled();

    cleanup();
    renderRoute({ ...defaults, activeSessionId: "session-1", location: sessionsLocation({ creatingSession: true }), onNavigate });
    expect(onNavigate).not.toHaveBeenCalled();
  });
});
