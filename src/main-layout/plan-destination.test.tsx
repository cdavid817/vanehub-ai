// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { PlanDestination } from "./plan-destination";

const capturedGoalCenterProps = vi.hoisted(() => ({ current: {} as Record<string, unknown> }));

// 15.1: richer than a bare loader-identity stub -- also exposes `componentProps` so this file can
// assert goalId/onSelectGoal are actually threaded through, the same depth runs-destination.test.tsx
// uses for scheduleId/onSelectSchedule (19.3).
vi.mock("../components/lazy-feature", () => ({
  LazyFeature: ({ componentProps, loader }: { componentProps: Record<string, unknown>; loader: () => Promise<unknown> }) => {
    capturedGoalCenterProps.current = componentProps;
    return (
      <div
        data-goal-id={String(componentProps.goalId)}
        data-loader={loader.name || loader.toString().slice(0, 40)}
        data-props={Object.keys(componentProps).sort().join(",")}
        data-testid="lazy-feature"
      />
    );
  },
}));

describe("PlanDestination", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("renders both sections as tabs and marks the active one", () => {
    render(<PlanDestination location={{ section: "board", viewId: undefined, workItemId: undefined }} onSectionChange={vi.fn()} />);
    const tabs = screen.getAllByRole("tab");
    expect(tabs).toHaveLength(2);
    expect(screen.getByRole("tab", { name: "Todo Board" }).getAttribute("aria-selected")).toBe("true");
    expect(screen.getByRole("tab", { name: "Goal Center" }).getAttribute("aria-selected")).toBe("false");
  });

  it("requests a section change when a tab is activated", () => {
    const onSectionChange = vi.fn();
    render(<PlanDestination location={{ section: "board", viewId: undefined, workItemId: undefined }} onSectionChange={onSectionChange} />);
    fireEvent.click(screen.getByRole("tab", { name: "Goal Center" }));
    expect(onSectionChange).toHaveBeenCalledWith({ section: "goals" });
  });

  // 20.7: this tablist previously had no keyboard model at all beyond native Tab-through-each-
  // button -- confirms the useTabList wiring, not just its own already-tested algorithm.
  it("20.7: moves to the next tab and moves focus with it on ArrowRight", () => {
    const onSectionChange = vi.fn();
    render(<PlanDestination location={{ section: "board", viewId: undefined, workItemId: undefined }} onSectionChange={onSectionChange} />);
    const boardTab = screen.getByRole("tab", { name: "Todo Board" });
    boardTab.focus();

    fireEvent.keyDown(boardTab, { key: "ArrowRight" });

    const goalsTab = screen.getByRole("tab", { name: "Goal Center" });
    expect(onSectionChange).toHaveBeenCalledWith({ section: "goals" });
    expect(document.activeElement).toBe(goalsTab);
  });

  it("renders exactly one lazy-loaded panel per section, board or goals but not both", () => {
    const board = render(<PlanDestination location={{ section: "board", viewId: undefined, workItemId: undefined }} onSectionChange={vi.fn()} />);
    expect(board.getAllByTestId("lazy-feature")).toHaveLength(1);
    board.unmount();

    render(<PlanDestination location={{ section: "goals", goalId: undefined }} onSectionChange={vi.fn()} />);
    expect(screen.getAllByTestId("lazy-feature")).toHaveLength(1);
  });

  it("15.1: routes goals to GoalCenter with goalId selection and onSelectGoal wired", () => {
    render(<PlanDestination location={{ section: "goals", goalId: undefined }} onSectionChange={vi.fn()} />);
    expect(screen.getByTestId("lazy-feature").dataset.props).toBe("goalId,onSelectGoal");
  });

  it("15.1: threads the route's own goalId through as GoalCenter's current selection", () => {
    render(<PlanDestination location={{ section: "goals", goalId: "goal-42" }} onSectionChange={vi.fn()} />);
    expect(screen.getByTestId("lazy-feature").dataset.goalId).toBe("goal-42");
  });

  it("15.1: requests a goals section change with the reported id when GoalCenter calls onSelectGoal", () => {
    const onSectionChange = vi.fn();
    render(<PlanDestination location={{ section: "goals", goalId: undefined }} onSectionChange={onSectionChange} />);

    const onSelectGoal = capturedGoalCenterProps.current.onSelectGoal as (goalId: string | undefined) => void;
    onSelectGoal("goal-99");

    expect(onSectionChange).toHaveBeenCalledWith({ section: "goals", goalId: "goal-99" });
  });
});
