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

    // 15.3: Delete moved from an always-visible button into the More menu.
    fireEvent.click(screen.getByRole("button", { name: "更多操作" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "删除" }));
    await waitFor(() => expect(screen.queryByRole("button", { name: /发布目标系统/ })).toBeNull());

    rejectDelete(new Error("删除失败"));

    await waitFor(() => expect(screen.getByRole("button", { name: /发布目标系统/ })).toBeTruthy());
    expect(screen.getByRole("alert").textContent).toContain("删除失败");

    fireEvent.click(screen.getByRole("button", { name: "关闭" }));
    await waitFor(() => expect(screen.queryByRole("alert")).toBeNull());
  });

  describe("15.3: primary action and More menu", () => {
    it.each([
      ["draft", "启用"],
      ["active", "验收"],
      ["achieved", "重开"],
      ["abandoned", "启用"],
    ] as const)("shows the %s goal's primary action as %s, not as a row of every permitted action", async (status, primaryLabel) => {
      mocks.goals = [fixture({ status, derivedStatus: status })];
      await openFirstGoal();

      expect(await screen.findByRole("button", { name: primaryLabel })).toBeTruthy();
      expect(screen.queryByRole("button", { name: "编辑" })).toBeNull();
      expect(screen.queryByRole("button", { name: "删除" })).toBeNull();
      expect(screen.queryByRole("button", { name: "放弃" })).toBeNull();
    });

    it("reaches edit, delete, and abandon through the More menu for an active goal", async () => {
      await openFirstGoal();

      fireEvent.click(screen.getByRole("button", { name: "更多操作" }));
      expect(screen.getByRole("menuitem", { name: "编辑" })).toBeTruthy();
      expect(screen.getByRole("menuitem", { name: "删除" })).toBeTruthy();
      expect(screen.getByRole("menuitem", { name: "放弃" })).toBeTruthy();
    });

    it("omits Abandon from the More menu once a goal is already abandoned", async () => {
      mocks.goals = [fixture({ status: "abandoned", derivedStatus: "abandoned" })];
      await openFirstGoal();

      fireEvent.click(screen.getByRole("button", { name: "更多操作" }));
      expect(screen.queryByRole("menuitem", { name: "放弃" })).toBeNull();
    });

    it("opens the edit sheet pre-filled with the goal's own values from the More menu", async () => {
      await openFirstGoal();

      fireEvent.click(screen.getByRole("button", { name: "更多操作" }));
      fireEvent.click(screen.getByRole("menuitem", { name: "编辑" }));

      expect((await screen.findByLabelText("标题") as HTMLInputElement).value).toBe("发布目标系统");
    });
  });

  describe("15.1: route-backed selection", () => {
    it("reports a clicked goal's id back through onSelectGoal", async () => {
      const onSelectGoal = vi.fn();
      render(<GoalCenter onSelectGoal={onSelectGoal} />);

      fireEvent.click(await screen.findByRole("button", { name: /发布目标系统/ }));
      expect(onSelectGoal).toHaveBeenCalledWith("goal-1");
    });

    it("pre-selects the goal named by the goalId prop on initial render", async () => {
      mocks.goals = [fixture(), fixture({ id: "goal-2", title: "第二个目标" })];
      render(<GoalCenter goalId="goal-2" />);

      expect(await screen.findByRole("heading", { name: "第二个目标" })).toBeTruthy();
    });

    it("clears the reported selection through onSelectGoal when the selected goal is deleted", async () => {
      const onSelectGoal = vi.fn();
      mocks.deleteGoal.mockResolvedValue(undefined);
      render(<GoalCenter onSelectGoal={onSelectGoal} />);
      fireEvent.click(await screen.findByRole("button", { name: /发布目标系统/ }));

      fireEvent.click(screen.getByRole("button", { name: "更多操作" }));
      fireEvent.click(screen.getByRole("menuitem", { name: "删除" }));

      await waitFor(() => expect(onSelectGoal).toHaveBeenCalledWith(undefined));
    });
  });
});
