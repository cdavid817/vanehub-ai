// @vitest-environment jsdom

import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "../i18n";
import type { GoalLink } from "../contracts/goal";
import { GoalRelationshipSections } from "./goal-relationship-sections";

// Labels below are zh-CN: this codebase's test harness defaults to zh-CN, not English
// (goal-center.test.tsx's own established convention -- see e.g. its "已失效"/"不计入" assertions,
// reused verbatim below since this component renders the same `goals.linkProgress.*` keys).
function links(count: number, overrides: Partial<GoalLink> = {}): GoalLink[] {
  return Array.from({ length: count }, (_, index) => ({
    targetKind: "loop", targetId: `loop-${index + 1}`, progress: "active", ...overrides,
  }));
}

describe("GoalRelationshipSections", () => {
  it("groups links by kind with a per-group count in the header", () => {
    render(<GoalRelationshipSections links={[
      { targetKind: "loop", targetId: "loop-1", progress: "active" },
      { targetKind: "work_item", targetId: "item-1", progress: "terminal" },
    ]} onUnlink={vi.fn()} pending={false} />);

    expect(screen.getByText("循环")).toBeTruthy();
    expect(screen.getByText("看板项")).toBeTruthy();
    expect(screen.getByText("loop-1")).toBeTruthy();
    expect(screen.getByText("item-1")).toBeTruthy();
  });

  it("renders an unresolvable link explicitly rather than dropping it from the view", () => {
    render(<GoalRelationshipSections links={[
      { targetKind: "loop", targetId: "loop-gone", progress: "unresolvable" },
    ]} onUnlink={vi.fn()} pending={false} />);

    expect(screen.getByText("loop-gone")).toBeTruthy();
    expect(screen.getByText("已失效")).toBeTruthy();
  });

  it("marks a session link as not counted rather than showing its raw progress state", () => {
    render(<GoalRelationshipSections links={[
      { targetKind: "session", targetId: "session-1", progress: "active" },
    ]} onUnlink={vi.fn()} pending={false} />);

    expect(screen.getByText("不计入")).toBeTruthy();
  });

  it("calls onUnlink with the link's own kind and id", () => {
    const onUnlink = vi.fn();
    render(<GoalRelationshipSections links={[{ targetKind: "loop", targetId: "loop-1", progress: "active" }]} onUnlink={onUnlink} pending={false} />);

    fireEvent.click(screen.getByRole("button", { name: "解除关联" }));
    expect(onUnlink).toHaveBeenCalledWith("loop", "loop-1");
  });

  it("disables every unlink control while a mutation is pending", () => {
    render(<GoalRelationshipSections links={[{ targetKind: "loop", targetId: "loop-1", progress: "active" }]} onUnlink={vi.fn()} pending={true} />);
    expect((screen.getByRole("button", { name: "解除关联" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("caps a large group at 20 rows and offers to show the rest on request", () => {
    render(<GoalRelationshipSections links={links(25)} onUnlink={vi.fn()} pending={false} />);

    const group = screen.getByText("循环").closest("div") as HTMLElement;
    expect(within(group).getAllByText(/^loop-\d+$/)).toHaveLength(20);
    expect(screen.queryByText("loop-25")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "显示另外 5 项" }));

    expect(screen.getByText("loop-25")).toBeTruthy();
    expect(screen.queryByRole("button", { name: /显示另外 \d+ 项/ })).toBeNull();
  });

  it("does not offer to show more when a group is already at or under the cap", () => {
    render(<GoalRelationshipSections links={links(20)} onUnlink={vi.fn()} pending={false} />);
    expect(screen.queryByRole("button", { name: /显示另外 \d+ 项/ })).toBeNull();
  });
});
