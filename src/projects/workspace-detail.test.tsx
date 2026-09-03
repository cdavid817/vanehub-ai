// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import type { Goal } from "../contracts/goal";
import type { MutationState } from "../ui/async/mutation-state";
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

import { WorkspaceDetail, type WorkspaceDetailProps } from "./workspace-detail";

function workspace(overrides: Partial<WorkspaceSummary> = {}): WorkspaceSummary {
  return {
    availability: "available", displayName: "app", displayPath: "D:\\repo\\app",
    kind: "local", workspaceId: "D:\\repo\\app", ...overrides,
  };
}

function detailElement(overrides: Partial<WorkspaceDetailProps> = {}) {
  return (
    <WorkspaceDetail
      onContinueSession={vi.fn()}
      onNewSession={vi.fn()}
      onOpenSshSettings={vi.fn()}
      onReconnect={vi.fn()}
      workspace={null}
      {...overrides}
    />
  );
}

function renderDetail(overrides: Partial<WorkspaceDetailProps> = {}) {
  return render(detailElement(overrides));
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
    renderDetail();
    expect(screen.getByTestId("workspace-detail-empty")).toBeTruthy();
    expect(screen.queryByTestId("workspace-detail")).toBeNull();
    await Promise.resolve();
    expect(mocks.listWorkItems).not.toHaveBeenCalled();
    expect(mocks.listGoals).not.toHaveBeenCalled();
  });

  it("renders identity, and shows trust as not-applicable for a local workspace rather than a fabricated badge", async () => {
    renderDetail({ workspace: workspace({ trust: undefined }) });
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
    renderDetail({ workspace: workspace({ kind: "ssh", trust: "trusted", workspaceId: "ssh://vane@dev.example.com/work/app" }) });
    expect(screen.getByText("SSH 远程工作区")).toBeTruthy();
    expect(screen.getByText("已信任")).toBeTruthy();
    expect(screen.queryByText("本地路径没有信任概念。")).toBeNull();
    await waitFor(() => expect(mocks.listWorkItems).toHaveBeenCalled());
  });

  it("renders git.repository true/false honestly, and never fabricates a branch or dirty state", async () => {
    const { rerender } = renderDetail({ workspace: workspace({ git: { repository: true } }) });
    expect(screen.getByText("是 Git 仓库")).toBeTruthy();
    expect(screen.getByText("本版本尚未采集分支、工作区改动与 worktree 路径。")).toBeTruthy();

    rerender(detailElement({ workspace: workspace({ git: { repository: false } }) }));
    expect(screen.getByText("不是 Git 仓库")).toBeTruthy();
    await waitFor(() => expect(mocks.listWorkItems).toHaveBeenCalled());
  });

  it("renders git as unknown (never checked) for an ssh workspace, since git is never derived for that kind", async () => {
    renderDetail({ workspace: workspace({ git: undefined, kind: "ssh" }) });
    expect(screen.getByText("远程工作区尚未检测")).toBeTruthy();
    await waitFor(() => expect(mocks.listWorkItems).toHaveBeenCalled());
  });

  it("renders the recent session when present, and an honest none-state when absent", async () => {
    const { rerender } = renderDetail({ workspace: workspace({ recentSession: undefined }) });
    expect(screen.getByText("暂无近期会话")).toBeTruthy();

    rerender(detailElement({
      workspace: workspace({
        recentSession: { id: "s-1", lifecycleState: "running", title: "Fixing tests", updatedAt: "2026-08-20T00:00:00.000Z" },
      }),
    }));
    expect(screen.getByText("Fixing tests")).toBeTruthy();
    await waitFor(() => expect(mocks.listWorkItems).toHaveBeenCalled());
  });

  it("always shows active Runs and Quality as honest unavailable states, never a fabricated count", async () => {
    renderDetail({ workspace: workspace() });
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

    renderDetail({ workspace: workspace() });

    expect(await screen.findByText("Matching item")).toBeTruthy();
    expect(await screen.findByText("Matching goal")).toBeTruthy();
    expect(screen.queryByText("Unrelated item")).toBeNull();
    expect(screen.queryByText("Unrelated goal")).toBeNull();
    expect(mocks.listWorkItems).toHaveBeenCalledWith({ archived: false });
  });

  it("shows an honest empty state when the real join finds no related Plan items", async () => {
    renderDetail({ workspace: workspace() });
    expect(await screen.findByText("未找到关联的工作项或目标。")).toBeTruthy();
  });
});

function openMoreMenu() {
  fireEvent.click(screen.getByRole("button", { name: "更多操作" }));
}

describe("WorkspaceDetail actions (task 13.8)", () => {
  beforeEach(() => {
    mocks.listGoals.mockReset().mockResolvedValue([]);
    mocks.listWorkItems.mockReset().mockResolvedValue([]);
  });

  it("shows Continue Session as primary when a recent session exists, and calls onContinueSession with its id", () => {
    const onContinueSession = vi.fn();
    renderDetail({
      onContinueSession,
      workspace: workspace({ recentSession: { id: "s-1", lifecycleState: "running", title: "Fixing tests", updatedAt: "2026-08-20T00:00:00.000Z" } }),
    });

    fireEvent.click(screen.getByRole("button", { name: "继续会话" }));

    expect(onContinueSession).toHaveBeenCalledWith("s-1");
    // Not a fabricated always-on control: Continue Session has nothing to continue when the
    // workspace has no recent session at all (covered by the sibling test below), so it must not
    // also appear as a spare item in More once it *is* primary.
    expect(screen.queryByRole("menuitem", { name: "继续会话" })).toBeNull();
  });

  it("shows New Session as primary when there is no recent session, and calls onNewSession with the workspace", () => {
    const onNewSession = vi.fn();
    const target = workspace({ recentSession: undefined });
    renderDetail({ onNewSession, workspace: target });

    fireEvent.click(screen.getByRole("button", { name: "新建会话" }));

    expect(onNewSession).toHaveBeenCalledWith(target);
  });

  it("moves New Session into More (rather than dropping it) once Continue Session takes the primary slot", () => {
    const onNewSession = vi.fn();
    const target = workspace({ recentSession: { id: "s-1", lifecycleState: "idle", title: "Session", updatedAt: "2026-08-20T00:00:00.000Z" } });
    renderDetail({ onNewSession, workspace: target });

    openMoreMenu();
    fireEvent.click(screen.getByRole("menuitem", { name: "新建会话" }));

    expect(onNewSession).toHaveBeenCalledWith(target);
  });

  it("renders neither Reconnect nor Settings for a local workspace -- neither concept applies to a local path", () => {
    renderDetail({ workspace: workspace({ kind: "local" }) });

    openMoreMenu();

    expect(screen.queryByRole("menuitem", { name: "重新连接" })).toBeNull();
    expect(screen.queryByRole("menuitem", { name: "SSH 连接设置" })).toBeNull();
  });

  it("renders Reconnect genuinely disabled, with an honest reason, when no SshConnection matched this ssh row", () => {
    const onReconnect = vi.fn();
    renderDetail({
      onReconnect,
      workspace: workspace({ connectionId: undefined, kind: "ssh", workspaceId: "ssh://vane@dev.example.com/work/app" }),
    });

    openMoreMenu();
    // The disabled reason renders as a second text node inside the same <button> (ActionMenu.tsx),
    // which folds into its computed accessible name alongside the label -- a regex prefix match,
    // not the exact label string, is what a real disabled-with-reason item looks like here.
    const reconnectItem = screen.getByRole("menuitem", { name: /^重新连接/ });

    expect(reconnectItem.getAttribute("aria-disabled")).toBe("true");
    expect(screen.getByText("该工作区没有关联的已保存 SSH 连接。")).toBeTruthy();
    fireEvent.click(reconnectItem);
    expect(onReconnect).not.toHaveBeenCalled();
  });

  it("enables Reconnect and calls onReconnect with the matched connectionId once one exists", () => {
    const onReconnect = vi.fn();
    renderDetail({
      onReconnect,
      workspace: workspace({ connectionId: "conn-1", kind: "ssh", workspaceId: "ssh://vane@dev.example.com/work/app" }),
    });

    openMoreMenu();
    const reconnectItem = screen.getByRole("menuitem", { name: "重新连接" });
    expect(reconnectItem.getAttribute("aria-disabled")).toBe("false");
    fireEvent.click(reconnectItem);

    expect(onReconnect).toHaveBeenCalledWith("conn-1");
  });

  it("shows a Reconnecting label and disables Reconnect while its mutation is pending, even though a connection is matched", () => {
    const reconnectMutation: MutationState = { pending: true, targetKey: "workspace-1" };
    renderDetail({
      reconnectMutation,
      workspace: workspace({ connectionId: "conn-1", kind: "ssh", workspaceId: "ssh://vane@dev.example.com/work/app" }),
    });

    openMoreMenu();
    const reconnectItem = screen.getByRole("menuitem", { name: "正在重新连接" });

    expect(reconnectItem.getAttribute("aria-disabled")).toBe("true");
  });

  it("renders the reconnect mutation's error inline and forwards a dismiss", () => {
    const onDismissReconnectError = vi.fn();
    const reconnectMutation: MutationState = {
      error: { kind: "error", message: "Connection refused", retryable: false }, pending: false, targetKey: "workspace-1",
    };
    renderDetail({
      onDismissReconnectError,
      reconnectMutation,
      workspace: workspace({ connectionId: "conn-1", kind: "ssh", workspaceId: "ssh://vane@dev.example.com/work/app" }),
    });

    expect(screen.getByText("Connection refused")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "关闭" }));
    expect(onDismissReconnectError).toHaveBeenCalledTimes(1);
  });

  it("calls onOpenSshSettings when Settings is chosen from More", () => {
    const onOpenSshSettings = vi.fn();
    renderDetail({ onOpenSshSettings, workspace: workspace({ kind: "ssh", workspaceId: "ssh://vane@dev.example.com/work/app" }) });

    openMoreMenu();
    fireEvent.click(screen.getByRole("menuitem", { name: "SSH 连接设置" }));

    expect(onOpenSshSettings).toHaveBeenCalledTimes(1);
  });
});
