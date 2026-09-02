// @vitest-environment jsdom

import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import type { Goal } from "../contracts/goal";
import type { WorkItem } from "../types/work-board";
import type { WorkspaceSummary } from "./workspace-summary";

const mocks = vi.hoisted(() => ({
  listGoals: vi.fn(),
  listWorkItems: vi.fn(),
}));

vi.mock("../services/runtime-work-board-client", () => ({
  workBoardService: { listWorkItems: mocks.listWorkItems },
}));

vi.mock("../services/runtime-goal-client", () => ({
  goalService: { listGoals: mocks.listGoals },
}));

import { WorkspaceDetail } from "./workspace-detail";

function workspace(overrides: Partial<WorkspaceSummary> = {}): WorkspaceSummary {
  return {
    availability: "available", displayName: "app", displayPath: "D:\\repo\\app",
    kind: "local", workspaceId: "D:\\repo\\app", ...overrides,
  };
}

function workItem(overrides: Partial<WorkItem> = {}): WorkItem {
  return {
    archived: false, createdAt: "2026-08-01T00:00:00.000Z", description: "", dueAt: null,
    id: "item-1", priority: "none", projectPath: null, rank: 0, sources: [], stage: "inbox",
    title: "Fix bug", updatedAt: "2026-08-01T00:00:00.000Z", ...overrides,
  };
}

function goal(overrides: Partial<Goal> = {}): Goal {
  return {
    acceptanceNotes: "", counted: 0, createdAt: "2026-08-01T00:00:00.000Z", derivedStatus: "active",
    description: "", id: "goal-1", links: [], projectPath: null, status: "active", terminal: 0,
    title: "Ship it", unresolvable: 0, updatedAt: "2026-08-01T00:00:00.000Z", ...overrides,
  };
}

describe("WorkspaceDetail", () => {
  beforeEach(() => {
    mocks.listGoals.mockReset().mockResolvedValue([]);
    mocks.listWorkItems.mockReset().mockResolvedValue([]);
  });

  it("shows the no-selection placeholder and fetches nothing when workspace is null", async () => {
    render(<WorkspaceDetail workspace={null} />);
    expect(screen.getByTestId("workspace-detail-empty")).toBeTruthy();
    expect(screen.queryByTestId("workspace-detail")).toBeNull();
    await Promise.resolve();
    expect(mocks.listWorkItems).not.toHaveBeenCalled();
    expect(mocks.listGoals).not.toHaveBeenCalled();
  });

  it("renders identity, and shows trust as not-applicable for a local workspace rather than a fabricated badge", async () => {
    render(<WorkspaceDetail workspace={workspace({ trust: undefined })} />);
    expect(screen.getByText("app")).toBeTruthy();
    expect(screen.getByText("D:\\repo\\app")).toBeTruthy();
    expect(screen.getByText("本地项目")).toBeTruthy();
    expect(screen.getByText("本地路径没有信任概念。")).toBeTruthy();
    // None of the trust badge copy renders when trust is absent -- it must not guess "unknown".
    expect(screen.queryByText("信任未知")).toBeNull();
    expect(screen.queryByText("已信任")).toBeNull();
    await waitFor(() => expect(mocks.listWorkItems).toHaveBeenCalled());
  });

  it("renders an ssh workspace's real trust badge instead of the local not-applicable copy", async () => {
    render(<WorkspaceDetail workspace={workspace({ kind: "ssh", trust: "trusted", workspaceId: "ssh://vane@dev.example.com/work/app" })} />);
    expect(screen.getByText("SSH 远程工作区")).toBeTruthy();
    expect(screen.getByText("已信任")).toBeTruthy();
    expect(screen.queryByText("本地路径没有信任概念。")).toBeNull();
    await waitFor(() => expect(mocks.listWorkItems).toHaveBeenCalled());
  });

  it("renders git.repository true/false honestly, and never fabricates a branch or dirty state", async () => {
    const { rerender } = render(<WorkspaceDetail workspace={workspace({ git: { repository: true } })} />);
    expect(screen.getByText("是 Git 仓库")).toBeTruthy();
    expect(screen.getByText("本版本尚未采集分支、工作区改动与 worktree 路径。")).toBeTruthy();

    rerender(<WorkspaceDetail workspace={workspace({ git: { repository: false } })} />);
    expect(screen.getByText("不是 Git 仓库")).toBeTruthy();
    await waitFor(() => expect(mocks.listWorkItems).toHaveBeenCalled());
  });

  it("renders git as unknown (never checked) for an ssh workspace, since git is never derived for that kind", async () => {
    render(<WorkspaceDetail workspace={workspace({ git: undefined, kind: "ssh" })} />);
    expect(screen.getByText("远程工作区尚未检测")).toBeTruthy();
    await waitFor(() => expect(mocks.listWorkItems).toHaveBeenCalled());
  });

  it("renders the recent session when present, and an honest none-state when absent", async () => {
    const { rerender } = render(<WorkspaceDetail workspace={workspace({ recentSession: undefined })} />);
    expect(screen.getByText("暂无近期会话")).toBeTruthy();

    rerender(<WorkspaceDetail workspace={workspace({
      recentSession: { id: "s-1", lifecycleState: "running", title: "Fixing tests", updatedAt: "2026-08-20T00:00:00.000Z" },
    })} />);
    expect(screen.getByText("Fixing tests")).toBeTruthy();
    await waitFor(() => expect(mocks.listWorkItems).toHaveBeenCalled());
  });

  it("always shows active Runs and Quality as honest unavailable states, never a fabricated count", async () => {
    render(<WorkspaceDetail workspace={workspace()} />);
    expect(screen.getByText("活动运行")).toBeTruthy();
    expect(screen.getByText("暂不可用：当前后端无法按工作区统计运行数。")).toBeTruthy();
    expect(screen.getByText("关联的质量评估")).toBeTruthy();
    expect(screen.getByText("暂不可用：评估任务与工作区之间尚无可关联字段。")).toBeTruthy();
    await waitFor(() => expect(mocks.listWorkItems).toHaveBeenCalled());
  });

  it("joins related work items and goals by the selected workspace's own id", async () => {
    mocks.listWorkItems.mockResolvedValue([
      workItem({ id: "match", projectPath: "D:\\repo\\app", title: "Matching item" }),
      workItem({ id: "other", projectPath: "D:\\repo\\other", title: "Unrelated item" }),
    ]);
    mocks.listGoals.mockResolvedValue([
      goal({ id: "match", derivedStatus: "active", projectPath: "D:\\repo\\app", title: "Matching goal" }),
      goal({ id: "other", projectPath: null, title: "Unrelated goal" }),
    ]);

    render(<WorkspaceDetail workspace={workspace()} />);

    expect(await screen.findByText("Matching item")).toBeTruthy();
    expect(await screen.findByText("Matching goal")).toBeTruthy();
    expect(screen.queryByText("Unrelated item")).toBeNull();
    expect(screen.queryByText("Unrelated goal")).toBeNull();
    expect(mocks.listWorkItems).toHaveBeenCalledWith({ archived: false });
  });

  it("shows an honest empty state when the real join finds no related Plan items", async () => {
    render(<WorkspaceDetail workspace={workspace()} />);
    expect(await screen.findByText("未找到关联的工作项或目标。")).toBeTruthy();
  });
});
