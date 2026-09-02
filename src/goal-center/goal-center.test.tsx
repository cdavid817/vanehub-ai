// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import type { Goal } from "../contracts/goal";

const mocks = vi.hoisted(() => ({
  accept: vi.fn(),
  activate: vi.fn(),
  create: vi.fn(),
  deleteGoal: vi.fn(),
  goals: [] as Goal[],
  link: vi.fn(),
  list: vi.fn(),
}));

vi.mock("../services/runtime-goal-client", () => ({
  goalService: {
    abandonGoal: vi.fn(),
    acceptGoal: mocks.accept,
    activateGoal: mocks.activate,
    createGoal: mocks.create,
    deleteGoal: mocks.deleteGoal,
    getGoal: vi.fn(),
    linkGoalTarget: mocks.link,
    listGoals: mocks.list,
    reopenGoal: vi.fn(),
    unlinkGoalTarget: vi.fn(),
    updateGoal: vi.fn(),
  },
}));

import { GoalCenter } from "./goal-center";

function fixture(overrides: Partial<Goal> = {}): Goal {
  return {
    id: "goal-1", title: "发布目标系统", description: "", acceptanceNotes: "",
    status: "active", derivedStatus: "active", projectPath: null,
    createdAt: "2026-08-15T00:00:00Z", updatedAt: "2026-08-15T00:00:00Z",
    counted: 1, terminal: 0, unresolvable: 0,
    links: [{ targetKind: "plan", targetId: "plan-1", progress: "active" }],
    ...overrides,
  };
}

async function openFirstGoal() {
  render(<GoalCenter />);
  const entry = await screen.findByRole("button", { name: /发布目标系统/ });
  fireEvent.click(entry);
}

describe("GoalCenter", () => {
  beforeEach(() => {
    mocks.accept.mockReset();
    mocks.activate.mockReset();
    mocks.create.mockReset();
    mocks.deleteGoal.mockReset();
    mocks.link.mockReset();
    mocks.list.mockReset();
    mocks.goals = [fixture()];
    mocks.list.mockImplementation(() => Promise.resolve(mocks.goals));
  });

  it("disables acceptance and names the blocker while a child is still running", async () => {
    await openFirstGoal();

    const accept = await screen.findByRole("button", { name: "验收" });
    expect(accept.hasAttribute("disabled")).toBe(true);
    expect(screen.getAllByText("还有子项在进行中。").length).toBeGreaterThan(0);
  });

  it("enables acceptance once every counted child has finished", async () => {
    mocks.goals = [fixture({
      derivedStatus: "awaiting_acceptance", terminal: 1,
      links: [{ targetKind: "plan", targetId: "plan-1", progress: "terminal" }],
    })];
    await openFirstGoal();

    const accept = await screen.findByRole("button", { name: "验收" });
    expect(accept.hasAttribute("disabled")).toBe(false);
    expect(screen.getAllByText("待验收").length).toBeGreaterThan(0);

    mocks.accept.mockResolvedValue(fixture({ status: "achieved", derivedStatus: "achieved" }));
    fireEvent.click(accept);
    await waitFor(() => expect(mocks.accept).toHaveBeenCalledWith("goal-1"));
  });

  it("surfaces a child that no longer exists instead of silently dropping it", async () => {
    mocks.goals = [fixture({
      derivedStatus: "awaiting_acceptance", counted: 1, terminal: 1, unresolvable: 1,
      links: [
        { targetKind: "plan", targetId: "plan-gone", progress: "unresolvable" },
        { targetKind: "loop", targetId: "loop-1", progress: "terminal" },
      ],
    })];
    await openFirstGoal();

    expect(await screen.findByRole("status")).toBeTruthy();
    expect(screen.getAllByText("已失效").length).toBe(1);
  });

  it("marks a linked session as not counted toward acceptance", async () => {
    mocks.goals = [fixture({
      links: [{ targetKind: "session", targetId: "session-1", progress: "active" }],
      counted: 0,
    })];
    await openFirstGoal();

    expect(await screen.findByText("不计入")).toBeTruthy();
  });

  it("links a target through the service boundary", async () => {
    await openFirstGoal();
    mocks.link.mockResolvedValue(fixture());

    fireEvent.change(await screen.findByLabelText("目标 ID"), { target: { value: " loop-9 " } });
    fireEvent.click(screen.getByRole("button", { name: "关联" }));

    await waitFor(() => expect(mocks.link).toHaveBeenCalledWith("goal-1", "loop", "loop-9"));
  });

  it("prompts for a selection before any goal is opened", async () => {
    render(<GoalCenter />);

    expect(await screen.findByText("选择一个目标查看其子项。")).toBeTruthy();
  });

  it("does not reload the goal list after a single goal's own successful mutation", async () => {
    mocks.accept.mockResolvedValue(fixture({
      derivedStatus: "awaiting_acceptance", terminal: 1,
      links: [{ targetKind: "plan", targetId: "plan-1", progress: "terminal" }],
    }));
    mocks.goals = [fixture({
      derivedStatus: "awaiting_acceptance", terminal: 1,
      links: [{ targetKind: "plan", targetId: "plan-1", progress: "terminal" }],
    })];
    await openFirstGoal();
    const loadCallsAfterMount = mocks.list.mock.calls.length;

    fireEvent.click(await screen.findByRole("button", { name: "验收" }));
    await waitFor(() => expect(mocks.accept).toHaveBeenCalledWith("goal-1"));

    expect(mocks.list.mock.calls.length).toBe(loadCallsAfterMount);
  });

  it("creates a goal by appending the server's response, without a full list reload", async () => {
    mocks.create.mockResolvedValue(fixture({ id: "goal-2", title: "新目标", counted: 0, terminal: 0, links: [] }));
    render(<GoalCenter />);
    await screen.findByRole("button", { name: /发布目标系统/ });
    const loadCallsAfterMount = mocks.list.mock.calls.length;

    fireEvent.click(screen.getByRole("button", { name: "新建目标" }));
    fireEvent.change(screen.getByLabelText("标题"), { target: { value: "新目标" } });
    fireEvent.click(screen.getByRole("button", { name: "创建" }));

    await screen.findByRole("button", { name: /新目标/ });
    expect(mocks.create).toHaveBeenCalledWith(expect.objectContaining({ title: "新目标" }));
    expect(mocks.list.mock.calls.length).toBe(loadCallsAfterMount);
  });

  it("does not disable a newly selected goal's own actions while a different goal's mutation is still pending", async () => {
    const first = fixture({ status: "draft", derivedStatus: "draft" });
    const second = fixture({ id: "goal-2", title: "第二个目标", status: "achieved", derivedStatus: "achieved" });
    mocks.goals = [first, second];
    let resolveActivate: (goal: Goal) => void = () => {};
    mocks.activate.mockImplementation(() => new Promise((resolve) => { resolveActivate = resolve; }));

    render(<GoalCenter />);
    const rowOne = () => screen.getByRole("button", { name: /发布目标系统/ });
    fireEvent.click(await screen.findByRole("button", { name: /发布目标系统/ }));
    fireEvent.click(await screen.findByRole("button", { name: "启用" }));

    // goal-1's own activate is pending -- its own row shows the ambient (decorative) pending cue.
    await waitFor(() => expect(within(rowOne()).getByTitle("保存中...")).toBeTruthy());

    // Switching to goal-2 must not carry goal-1's pending state along: its own action is enabled.
    fireEvent.click(screen.getByRole("button", { name: /第二个目标/ }));
    const reopen = (await screen.findByRole("button", { name: "重开" })) as HTMLButtonElement;
    expect(reopen.disabled).toBe(false);

    // goal-1's own row still shows it is pending even though it is no longer the selected goal.
    expect(within(rowOne()).getByTitle("保存中...")).toBeTruthy();

    resolveActivate(fixture({ status: "active", derivedStatus: "active" }));
    await waitFor(() => expect(within(rowOne()).queryByTitle("保存中...")).toBeNull());
  });

  it("rolls back an optimistic delete and shows the goal's own dismissible error on failure", async () => {
    let rejectDelete: (reason: unknown) => void = () => {};
    mocks.deleteGoal.mockImplementation(() => new Promise((_resolve, reject) => { rejectDelete = reject; }));
    await openFirstGoal();

    fireEvent.click(await screen.findByRole("button", { name: "删除" }));
    await waitFor(() => expect(screen.queryByRole("button", { name: /发布目标系统/ })).toBeNull());

    rejectDelete(new Error("删除失败"));

    await waitFor(() => expect(screen.getByRole("button", { name: /发布目标系统/ })).toBeTruthy());
    expect(screen.getByRole("alert").textContent).toContain("删除失败");

    fireEvent.click(screen.getByRole("button", { name: "关闭" }));
    await waitFor(() => expect(screen.queryByRole("alert")).toBeNull());
  });
});
