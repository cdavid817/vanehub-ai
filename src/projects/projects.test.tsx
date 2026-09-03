// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import type { KnownProject, KnownRemoteWorkspace, Session } from "../types/agent";
import type { SshConnection } from "../types/ssh-connection";

const mocks = vi.hoisted(() => ({
  connections: vi.fn(),
  inspectProject: vi.fn(),
  listGoals: vi.fn(),
  listWorkItems: vi.fn(),
  projects: vi.fn(),
  remoteWorkspaces: vi.fn(),
  sessions: vi.fn(),
  testConnection: vi.fn(),
}));

vi.mock("../services/runtime-agent-client", () => ({
  agentService: {
    inspectProject: mocks.inspectProject,
    listKnownProjects: mocks.projects,
    listKnownRemoteWorkspaces: mocks.remoteWorkspaces,
    listSessions: mocks.sessions,
  },
}));

vi.mock("../services/runtime-ssh-connection-client", () => ({
  sshConnectionService: { listConnections: mocks.connections, testConnection: mocks.testConnection },
}));

// WorkspaceDetail's own related-Plan-links section (13.7) calls these two existing services once
// a workspace is selected -- mocked here the same way as agentService/sshConnectionService above,
// so a selection test in this file does not depend on the real Web/Tauri work-board or goal client.
vi.mock("../services/runtime-work-board-client", () => ({
  workBoardService: { listWorkItems: mocks.listWorkItems },
}));

vi.mock("../services/runtime-goal-client", () => ({
  goalService: { listGoals: mocks.listGoals },
}));

import { Projects } from "./projects";

const localProject: KnownProject = { path: "D:\\repo\\app", displayName: "app", isGit: true, lastOpenedAt: "2026-08-20T00:00:00.000Z" };
const remoteWorkspace: KnownRemoteWorkspace = {
  displayName: "dev.example.com:app", host: "dev.example.com", lastOpenedAt: "2026-08-10T00:00:00.000Z",
  path: "/work/app", port: 22, uri: "ssh://vane@dev.example.com/work/app", user: "vane",
};
const matchedConnection: SshConnection = {
  authMode: "key", createdAt: "2026-08-01T00:00:00.000Z", defaultPath: "/work/app", hasPassword: false,
  host: "dev.example.com", hostTrust: null, id: "conn-1", keyPath: "/home/vane/.ssh/id_ed25519",
  lastConnectedAt: null, lastError: null, name: "Dev box", port: 22, revision: 1,
  testStatus: "not-tested", updatedAt: "2026-08-01T00:00:00.000Z", user: "vane",
};

function session(overrides: Partial<Session> = {}): Session {
  return {
    id: "session-1", personalizationMode: "standard", title: "Session", agentId: "claude-code",
    interactionMode: "cli", lifecycleState: "idle", recoveryStatus: "clean",
    recoveryRevision: 0, stateRevision: 0, historyRevision: 0, activeExecutionRunId: null,
    folder: null, projectPath: null, worktreePath: null, worktreeName: null, worktreeBranch: null,
    remoteWorkspace: null, remoteSshConnectionId: null, remoteSshConnectionRevision: null,
    runtimeSessionId: null, categoryId: null, pinned: false, archived: false,
    createdAt: "2026-08-01T00:00:00.000Z", updatedAt: "2026-08-01T00:00:00.000Z", ...overrides,
  };
}

function renderProjects(overrides: Partial<{
  onContinueSession: (sessionId: string) => void;
  onNewSession: (workspace: { workspaceId: string; kind: "local" | "ssh" }) => void;
  onOpenSshSettings: () => void;
}> = {}) {
  return render(
    <Projects
      onContinueSession={vi.fn()}
      onNewSession={vi.fn()}
      onOpenSshSettings={vi.fn()}
      {...overrides}
    />,
  );
}

describe("Projects", () => {
  beforeEach(() => {
    mocks.connections.mockReset().mockResolvedValue([]);
    mocks.inspectProject.mockReset().mockResolvedValue({ displayName: "app", gitRoot: localProject.path, isGit: true, path: localProject.path });
    mocks.listGoals.mockReset().mockResolvedValue([]);
    mocks.listWorkItems.mockReset().mockResolvedValue([]);
    mocks.projects.mockReset().mockResolvedValue([localProject]);
    mocks.remoteWorkspaces.mockReset().mockResolvedValue([remoteWorkspace]);
    mocks.sessions.mockReset().mockResolvedValue([]);
    mocks.testConnection.mockReset();
  });

  it("renders a local row and an unmatched SSH row without fabricating trust or availability", async () => {
    renderProjects();

    expect(await screen.findByText("app")).toBeTruthy();
    expect(screen.getByText("dev.example.com:app")).toBeTruthy();
    // No SshConnection matches this remote workspace's host/port/user, so the row must show the
    // honest "not confirmed" state rather than a guessed "trusted"/"available".
    expect(screen.getAllByText("信任未知").length).toBeGreaterThan(0);
    expect(screen.getAllByText("已断开").length).toBeGreaterThan(0);
  });

  it("shows the empty state when nothing is known yet", async () => {
    mocks.projects.mockResolvedValue([]);
    mocks.remoteWorkspaces.mockResolvedValue([]);

    renderProjects();

    expect(await screen.findByText("暂无项目或工作区")).toBeTruthy();
  });

  it("switches to the unavailable view and shows only non-available rows", async () => {
    renderProjects();
    await screen.findByText("app");

    fireEvent.click(screen.getByRole("tab", { name: "不可用" }));

    await waitFor(() => expect(screen.queryByText("app")).toBeNull());
    expect(screen.getByText("dev.example.com:app")).toBeTruthy();
  });

  it("shows the detail panel's own no-selection placeholder before any workspace is chosen", async () => {
    renderProjects();
    await screen.findByText("app");

    expect(screen.getByTestId("workspace-detail-empty")).toBeTruthy();
    expect(screen.queryByTestId("workspace-detail")).toBeNull();
  });

  it("selecting a workspace card shows its own detail panel", async () => {
    renderProjects();
    await screen.findByText("app");

    // WorkspaceCard's own data-testid, keyed by the same workspaceId workspace-aggregation.ts
    // derives from KnownProject.path -- confirms selection is driven by the real card, not by
    // clicking arbitrary text that happens to say "app".
    fireEvent.click(screen.getByTestId(`workspace-${localProject.path}`));

    const detail = await screen.findByTestId("workspace-detail");
    expect(within(detail).getByText("app")).toBeTruthy();
    expect(within(detail).getByText(localProject.path)).toBeTruthy();
    expect(screen.queryByTestId("workspace-detail-empty")).toBeNull();
  });

  it("wires the primary New Session action to onNewSession with the selected workspace when there is no recent session", async () => {
    const onNewSession = vi.fn();
    renderProjects({ onNewSession });
    await screen.findByText("app");
    fireEvent.click(screen.getByTestId(`workspace-${localProject.path}`));
    const detail = await screen.findByTestId("workspace-detail");

    fireEvent.click(within(detail).getByRole("button", { name: "新建会话" }));

    expect(onNewSession).toHaveBeenCalledWith(expect.objectContaining({ kind: "local", workspaceId: localProject.path }));
  });

  it("wires the primary Continue Session action to onContinueSession with the recent session's id once one exists", async () => {
    mocks.sessions.mockResolvedValue([session({ id: "s-1", projectPath: localProject.path })]);
    const onContinueSession = vi.fn();
    renderProjects({ onContinueSession });
    await screen.findByText("app");
    fireEvent.click(screen.getByTestId(`workspace-${localProject.path}`));
    const detail = await screen.findByTestId("workspace-detail");

    fireEvent.click(within(detail).getByRole("button", { name: "继续会话" }));

    expect(onContinueSession).toHaveBeenCalledWith("s-1");
  });

  it("wires Settings to onOpenSshSettings for an SSH row, from the More menu", async () => {
    const onOpenSshSettings = vi.fn();
    renderProjects({ onOpenSshSettings });
    await screen.findByText("app");
    fireEvent.click(screen.getByTestId(`workspace-${remoteWorkspace.uri}`));
    const detail = await screen.findByTestId("workspace-detail");

    fireEvent.click(within(detail).getByRole("button", { name: "更多操作" }));
    fireEvent.click(within(detail).getByRole("menuitem", { name: "SSH 连接设置" }));

    expect(onOpenSshSettings).toHaveBeenCalledTimes(1);
  });

  it("leaves Reconnect disabled with an honest reason when no SshConnection matched this row", async () => {
    renderProjects();
    await screen.findByText("app");
    fireEvent.click(screen.getByTestId(`workspace-${remoteWorkspace.uri}`));
    const detail = await screen.findByTestId("workspace-detail");

    fireEvent.click(within(detail).getByRole("button", { name: "更多操作" }));
    // The disabled reason renders inside the same <button> as the label (ActionMenu.tsx), so it
    // folds into the computed accessible name -- a prefix match, not the exact label string.
    const reconnectItem = within(detail).getByRole("menuitem", { name: /^重新连接/ });

    expect(reconnectItem.getAttribute("aria-disabled")).toBe("true");
    expect(within(detail).getByText("该工作区没有关联的已保存 SSH 连接。")).toBeTruthy();
    fireEvent.click(reconnectItem);
    expect(mocks.testConnection).not.toHaveBeenCalled();
  });

  it("calls SshConnectionService.testConnection and reloads the list when Reconnect is used on a matched row", async () => {
    mocks.connections.mockResolvedValue([matchedConnection]);
    mocks.testConnection.mockResolvedValue({ message: "ok", status: "succeeded", testedAt: "2026-08-20T00:00:00.000Z" });
    renderProjects();
    await screen.findByText("app");
    fireEvent.click(screen.getByTestId(`workspace-${remoteWorkspace.uri}`));
    const detail = await screen.findByTestId("workspace-detail");
    expect(mocks.projects).toHaveBeenCalledTimes(1);

    fireEvent.click(within(detail).getByRole("button", { name: "更多操作" }));
    const reconnectItem = within(detail).getByRole("menuitem", { name: "重新连接" });
    expect(reconnectItem.getAttribute("aria-disabled")).toBe("false");
    fireEvent.click(reconnectItem);

    await waitFor(() => expect(mocks.testConnection).toHaveBeenCalledWith(matchedConnection.id));
    // A successful reconnect refreshes the whole workspace list, the same as the header's own
    // Refresh button -- otherwise the trust/availability badges this test's row shows would stay
    // stale even though the connection was just confirmed reachable.
    await waitFor(() => expect(mocks.projects).toHaveBeenCalledTimes(2));
  });
});
