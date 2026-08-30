// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { PlanDestination } from "./plan-destination";

vi.mock("../components/lazy-feature", () => ({
  LazyFeature: ({ loader }: { loader: () => Promise<unknown> }) => (
    <div data-loader={loader.name || loader.toString().slice(0, 40)} data-testid="lazy-feature" />
  ),
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

  it("renders exactly one lazy-loaded panel per section, board or goals but not both", () => {
    const board = render(<PlanDestination location={{ section: "board", viewId: undefined, workItemId: undefined }} onSectionChange={vi.fn()} />);
    expect(board.getAllByTestId("lazy-feature")).toHaveLength(1);
    board.unmount();

    render(<PlanDestination location={{ section: "goals", goalId: undefined }} onSectionChange={vi.fn()} />);
    expect(screen.getAllByTestId("lazy-feature")).toHaveLength(1);
  });
});
