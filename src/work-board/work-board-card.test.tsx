// @vitest-environment jsdom

import { fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../i18n";
import type { MutationState } from "../ui/async/mutation-state";
import type { WorkItem } from "../types/work-board";
import { WorkBoardCard } from "./work-board-card";

const fixture = (overrides: Partial<WorkItem> = {}): WorkItem => ({
  id: "work-1",
  title: "发布版本",
  description: "",
  stage: "inbox",
  priority: "none",
  rank: 1_000,
  projectPath: null,
  dueAt: null,
  archived: false,
  createdAt: "2026-01-01",
  updatedAt: "2026-01-01",
  sources: [],
  ...overrides,
});

function renderCard(itemOverrides: Partial<WorkItem> = {}, mutation?: MutationState) {
  const onMove = vi.fn();
  const utils = render(
    <WorkBoardCard
      item={fixture(itemOverrides)}
      mutation={mutation}
      onArchive={vi.fn()}
      onDelete={vi.fn()}
      onDismissError={vi.fn()}
      onEdit={vi.fn()}
      onMove={onMove}
      onRestore={vi.fn()}
    />,
  );
  return { onMove, ...utils };
}

// Tasks 14.8-14.9 (design.md Decision 12): a card drag submits one stage mutation, and the
// non-drag path collapses the previous permanent prev-arrow/bare-select/next-arrow trio into one
// "Move to…" Menu/Listbox control (WorkItemStageMenu), driven by the same `onMove` callback.
describe("WorkBoardCard stage control", () => {
  it("replaces the old prev-arrow/bare-select/next-arrow trio with a single Move to trigger", () => {
    const { container } = renderCard();

    expect(container.querySelector("select")).toBeNull();
    expect(screen.queryByLabelText("工作阶段")).toBeNull();
    expect(screen.queryByRole("button", { name: "移至上一阶段" })).toBeNull();
    expect(screen.queryByRole("button", { name: "移至下一阶段" })).toBeNull();
    // Exactly one trigger remains, and it is a real disclosure control, not a fourth decoration.
    expect(screen.getByRole("button", { name: "收件箱" }).getAttribute("aria-haspopup")).toBe("listbox");
  });

  it("opens on trigger click and lists every stage with only the current one selected", () => {
    renderCard({ stage: "in_progress" });
    fireEvent.click(screen.getByRole("button", { name: "进行中" }));

    expect(screen.getAllByRole("option")).toHaveLength(5);
    const labels: Record<string, boolean> = {
      收件箱: false, 已计划: false, 进行中: true, 待审核: false, 已完成: false,
    };
    for (const [label, selected] of Object.entries(labels)) {
      expect(screen.getByRole("option", { name: label }).getAttribute("aria-selected")).toBe(String(selected));
    }
  });

  it("calls onMove with the chosen stage and closes the popover", () => {
    const { onMove } = renderCard({ stage: "inbox" });
    fireEvent.click(screen.getByRole("button", { name: "收件箱" }));
    fireEvent.click(screen.getByRole("option", { name: "待审核" }));

    expect(onMove).toHaveBeenCalledWith("review");
    expect(onMove).toHaveBeenCalledOnce();
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("does not call onMove when reselecting the already-current stage", () => {
    const { onMove } = renderCard({ stage: "inbox" });
    fireEvent.click(screen.getByRole("button", { name: "收件箱" }));
    fireEvent.click(screen.getByRole("option", { name: "收件箱" }));

    expect(onMove).not.toHaveBeenCalled();
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("is operable via keyboard alone: arrow keys roam the open list and Enter selects", async () => {
    const user = userEvent.setup();
    const { onMove } = renderCard({ stage: "inbox" });
    const trigger = screen.getByRole("button", { name: "收件箱" });

    fireEvent.click(trigger);
    const options = screen.getAllByRole("option");
    expect(document.activeElement).toBe(options[0]);

    await user.keyboard("{ArrowDown}");
    expect(document.activeElement).toBe(options[1]);

    await user.keyboard("{Enter}");
    expect(onMove).toHaveBeenCalledWith("planned");
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });

  it("closes on Escape without moving, returning focus to the trigger", () => {
    const { onMove } = renderCard({ stage: "inbox" });
    const trigger = screen.getByRole("button", { name: "收件箱" });
    fireEvent.click(trigger);

    fireEvent.keyDown(screen.getByRole("listbox"), { key: "Escape" });
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(document.activeElement).toBe(trigger);
    expect(onMove).not.toHaveBeenCalled();
  });

  it("disables the trigger while this card has its own mutation pending", () => {
    renderCard({ stage: "inbox" }, { targetKey: "work-1", pending: true });
    expect((screen.getByRole("button", { name: "收件箱" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("does not render a stage trigger for an archived card", () => {
    renderCard({ archived: true, stage: "done" });
    expect(screen.queryByRole("button", { name: "已完成" })).toBeNull();
  });
});

describe("WorkBoardCard stage control scoped within a card", () => {
  it("keeps each card's own trigger and options independently addressable", () => {
    render(
      <>
        <WorkBoardCard item={fixture({ id: "work-1", stage: "inbox" })} onArchive={vi.fn()} onDelete={vi.fn()} onDismissError={vi.fn()} onEdit={vi.fn()} onMove={vi.fn()} onRestore={vi.fn()} />
        <WorkBoardCard item={fixture({ id: "work-2", stage: "inbox" })} onArchive={vi.fn()} onDelete={vi.fn()} onDismissError={vi.fn()} onEdit={vi.fn()} onMove={vi.fn()} onRestore={vi.fn()} />
      </>,
    );
    const cardOne = screen.getByTestId("work-item-work-1");
    const cardTwo = screen.getByTestId("work-item-work-2");
    expect(screen.getAllByRole("button", { name: "收件箱" })).toHaveLength(2);

    fireEvent.click(within(cardOne).getByRole("button", { name: "收件箱" }));
    expect(within(cardOne).getByRole("listbox")).toBeTruthy();
    expect(within(cardTwo).queryByRole("listbox")).toBeNull();
  });
});
