// @vitest-environment jsdom

import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import type { Goal } from "../contracts/goal";
import { generateGoals } from "../testing/fixtures/goal-fixtures";
import { FIXTURE_COUNTS } from "../testing/fixtures/large-scale-fixtures";

const mocks = vi.hoisted(() => ({ list: vi.fn() }));

vi.mock("../services/runtime-goal-client", () => ({
  goalService: {
    abandonGoal: vi.fn(),
    acceptGoal: vi.fn(),
    activateGoal: vi.fn(),
    createGoal: vi.fn(),
    deleteGoal: vi.fn(),
    getGoal: vi.fn(),
    linkGoalTarget: vi.fn(),
    listGoals: mocks.list,
    reopenGoal: vi.fn(),
    unlinkGoalTarget: vi.fn(),
    updateGoal: vi.fn(),
  },
}));

import { GoalCenter } from "./goal-center";

/**
 * 21.6: `generateGoals` (task 0.9's own "500 deterministic Goals" fixture) had zero real
 * consumers anywhere in this codebase before this file -- only `large-scale-fixtures.ts`'s own
 * aggregator and its self-verification test imported it. Unlike every sibling destination
 * (Sessions, Work Board, Mission Control, Loop Center, Evaluation, Scheduled Tasks, all wired to
 * their own generator by 21.7/21.9-21.14), Goal Center's own list surface had never been rendered
 * at its designed fixture scale. `goal-center.tsx`'s list has no virtualization and
 * `goalService.listGoals()` takes no cursor/limit (confirmed by reading both directly) -- the
 * same shape 21.14 already found and accepted for Scheduled Tasks at its own smaller 100-item
 * scale -- so this is a DOM-bounded-render sanity/selection-integrity check, not a windowing
 * budget: there is no window to bound.
 */
describe("GoalCenter at large-scale-fixture scale", () => {
  const goals: Goal[] = generateGoals(FIXTURE_COUNTS.goals);

  beforeEach(() => {
    mocks.list.mockReset();
    mocks.list.mockResolvedValue(goals);
  });

  it("renders every one of 500 deterministically-generated goals exactly once, fetched only once", async () => {
    render(<GoalCenter />);
    const items = await screen.findAllByRole("listitem");
    expect(items).toHaveLength(FIXTURE_COUNTS.goals);
    expect(mocks.list).toHaveBeenCalledTimes(1);
  });

  it("selects a goal in the middle of 500 without disturbing any other row's selection state", async () => {
    render(<GoalCenter />);
    const items = await screen.findAllByRole("listitem");
    const targetButton = within(items[250]).getByRole("button");
    const otherButton = within(items[0]).getByRole("button");
    expect(targetButton.getAttribute("aria-current")).toBe("false");

    fireEvent.click(targetButton);

    expect(targetButton.getAttribute("aria-current")).toBe("true");
    expect(otherButton.getAttribute("aria-current")).toBe("false");
  });
});
