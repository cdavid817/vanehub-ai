// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import type { KnownProject, KnownRemoteWorkspace } from "../types/agent";

const mocks = vi.hoisted(() => ({
  connections: vi.fn(),
  inspectProject: vi.fn(),
  listGoals: vi.fn(),
  listWorkItems: vi.fn(),
  projects: vi.fn(),
  remoteWorkspaces: vi.fn(),
  sessions: vi.fn(),
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
  sshConnectionService: { listConnections: mocks.connections },
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

describe("Projects", () => {
  beforeEach(() => {
    mocks.connections.mockReset().mockResolvedValue([]);
    mocks.inspectProject.mockReset().mockResolvedValue({ displayName: "app", gitRoot: localProject.path, isGit: true, path: localProject.path });
    mocks.listGoals.mockReset().mockResolvedValue([]);
    mocks.listWorkItems.mockReset().mockResolvedValue([]);
    mocks.projects.mockReset().mockResolvedValue([localProject]);
    mocks.remoteWorkspaces.mockReset().mockResolvedValue([remoteWorkspace]);
    mocks.sessions.mockReset().mockResolvedValue([]);
  });

  it("renders a local row and an unmatched SSH row without fabricating trust or availability", async () => {
    render(<Projects />);

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

    render(<Projects />);

    expect(await screen.findByText("暂无项目或工作区")).toBeTruthy();
  });

  it("switches to the unavailable view and shows only non-available rows", async () => {
    render(<Projects />);
    await screen.findByText("app");

    fireEvent.click(screen.getByRole("tab", { name: "不可用" }));

    await waitFor(() => expect(screen.queryByText("app")).toBeNull());
    expect(screen.getByText("dev.example.com:app")).toBeTruthy();
  });

  it("shows the detail panel's own no-selection placeholder before any workspace is chosen", async () => {
    render(<Projects />);
    await screen.findByText("app");

    expect(screen.getByTestId("workspace-detail-empty")).toBeTruthy();
    expect(screen.queryByTestId("workspace-detail")).toBeNull();
  });

  it("selecting a workspace card shows its own detail panel", async () => {
    render(<Projects />);
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
});
