// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import type { Goal } from "../contracts/goal";

const mocks = vi.hoisted(() => ({
  accept: vi.fn(),
  goals: [] as Goal[],
  link: vi.fn(),
  list: vi.fn(),
}));

vi.mock("../services/runtime-goal-client", () => ({
  goalService: {
    abandonGoal: vi.fn(),
    acceptGoal: mocks.accept,
    activateGoal: vi.fn(),
    createGoal: vi.fn(),
    deleteGoal: vi.fn(),
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
});
