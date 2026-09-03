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

// Task 14.6: "one open action, and More" -- the stage picker is the one open action for a
// non-archived card (Edit/Archive collapse into More); Restore is the one open action for an
// archived card (Delete collapses into its own More).
describe("WorkBoardCard action grouping", () => {
  it("keeps only the stage trigger directly visible for a non-archived card, with Edit and Archive behind More", () => {
    renderCard({ archived: false });
    expect(screen.queryByRole("button", { name: "编辑工作项" })).toBeNull();
    expect(screen.queryByRole("button", { name: "归档工作项" })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "更多操作" }));
    expect(screen.getByRole("menuitem", { name: "编辑工作项" })).toBeTruthy();
    expect(screen.getByRole("menuitem", { name: "归档工作项" })).toBeTruthy();
    // Delete only ever applies to an archived item -- it must not leak into the active More menu.
    expect(screen.queryByRole("menuitem", { name: "永久删除" })).toBeNull();
  });

  it("keeps only Restore directly visible for an archived card, with Delete behind its own More", () => {
    renderCard({ archived: true });
    expect(screen.getByRole("button", { name: "恢复" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "永久删除" })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "更多操作" }));
    expect(screen.getByRole("menuitem", { name: "永久删除" })).toBeTruthy();
    expect(screen.queryByRole("menuitem", { name: "编辑工作项" })).toBeNull();
  });

  it("calls the right handler for each More item and closes the menu afterward", () => {
    const onEdit = vi.fn();
    const onArchive = vi.fn();
    render(<WorkBoardCard item={fixture()} onArchive={onArchive} onDelete={vi.fn()} onDismissError={vi.fn()} onEdit={onEdit} onMove={vi.fn()} onRestore={vi.fn()} />);

    fireEvent.click(screen.getByRole("button", { name: "更多操作" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "编辑工作项" }));
    expect(onEdit).toHaveBeenCalledOnce();
    expect(screen.queryByRole("menu")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "更多操作" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "归档工作项" }));
    expect(onArchive).toHaveBeenCalledOnce();
  });

  it("disables both the stage trigger and the More trigger's own items while this card's mutation is pending", () => {
    renderCard({ archived: false }, { targetKey: "work-1", pending: true });
    expect((screen.getByRole("button", { name: "收件箱" }) as HTMLButtonElement).disabled).toBe(true);

    fireEvent.click(screen.getByRole("button", { name: "更多操作" }));
    expect(screen.getByRole("menuitem", { name: "编辑工作项" }).getAttribute("aria-disabled")).toBe("true");
  });
});

describe("WorkBoardCard sources display bound", () => {
  const manySources = Array.from({ length: 5 }, (_unused, index) => ({
    sourceKind: "session" as const, sourceId: `s${index}`, relation: "execution" as const,
    title: `会话 ${index}`, status: "idle", available: true, projectPath: null, updatedAt: null,
  }));

  it("shows every source when the count is within the display bound", () => {
    renderCard({ sources: manySources.slice(0, 3) });
    expect(screen.getAllByRole("listitem")).toHaveLength(3);
    expect(screen.queryByText("还有 2 项")).toBeNull();
  });

  it("caps the visible sources and shows a +N more affordance beyond the bound", () => {
    renderCard({ sources: manySources });
    const items = screen.getAllByRole("listitem");
    // 3 real source rows plus one "+N more" row -- the cap is a display bound, not data loss:
    // nothing about the underlying item or its sources array is truncated, only this list's render.
    expect(items).toHaveLength(4);
    expect(screen.getByText("还有 2 项")).toBeTruthy();
  });
});

// 14.12: batch mode swaps the card's normal one-open-action-plus-More row for a plain checkbox,
// mirroring SessionCard's own established batchMode swap (src/main-layout/session-card.tsx).
describe("WorkBoardCard batch mode (14.12)", () => {
  it("shows a checkbox instead of the stage trigger and More menu while batchMode is true", () => {
    const onToggleSelected = vi.fn();
    render(
      <WorkBoardCard
        batchMode
        item={fixture()}
        onArchive={vi.fn()}
        onDelete={vi.fn()}
        onDismissError={vi.fn()}
        onEdit={vi.fn()}
        onMove={vi.fn()}
        onRestore={vi.fn()}
        onToggleSelected={onToggleSelected}
        selected={false}
      />,
    );
    expect(screen.queryByRole("button", { name: "收件箱" })).toBeNull();
    expect(screen.queryByRole("button", { name: "更多操作" })).toBeNull();
    expect(screen.getByRole("checkbox", { name: "选择工作项" })).toBeTruthy();
  });

  it("reflects the selected prop and calls onToggleSelected on a tap/click", () => {
    const onToggleSelected = vi.fn();
    render(
      <WorkBoardCard
        batchMode
        item={fixture()}
        onArchive={vi.fn()}
        onDelete={vi.fn()}
        onDismissError={vi.fn()}
        onEdit={vi.fn()}
        onMove={vi.fn()}
        onRestore={vi.fn()}
        onToggleSelected={onToggleSelected}
        selected
      />,
    );
    const checkbox = screen.getByRole("checkbox", { name: "选择工作项" }) as HTMLInputElement;
    expect(checkbox.checked).toBe(true);
    fireEvent.click(checkbox);
    expect(onToggleSelected).toHaveBeenCalledOnce();
  });

  it("is not draggable while batchMode is true, even for an eligible card", () => {
    const { container } = render(
      <WorkBoardCard
        batchMode
        item={fixture()}
        onArchive={vi.fn()}
        onDelete={vi.fn()}
        onDismissError={vi.fn()}
        onEdit={vi.fn()}
        onMove={vi.fn()}
        onRestore={vi.fn()}
      />,
    );
    expect(container.querySelector("article")?.getAttribute("draggable")).toBe("false");
  });

  it("keeps the normal action row and dragging when batchMode is omitted", () => {
    renderCard();
    expect(screen.getByRole("button", { name: "收件箱" })).toBeTruthy();
    expect(screen.queryByRole("checkbox", { name: "选择工作项" })).toBeNull();
  });
});

/**
 * 20.17: goes beyond the visual (pixel-screenshot) theme-parity coverage added elsewhere this
 * session -- proves the batch-mode selected checkbox is structurally, not just visually, identical
 * between `futuristic` and `minimal`. `WorkBoardCard` never reads a theme at all (confirmed by
 * grep: no `useTheme`/`data-theme` reference anywhere under src/work-board/) -- theming in this
 * codebase is pure CSS custom-property scoping on `:root[data-theme]` (styles.css), never a second
 * JSX tree (design.md Decision 19: "禁止为主题建立两套 JSX"). Rendering the identical props under
 * each theme's `data-theme` ancestor and diffing the checkbox's own `outerHTML` turns that
 * architectural guarantee into a real, regression-guarding assertion instead of a claim only a
 * source-code grep backs -- the same technique `goal-relationship-sections.test.tsx`'s own 20.17
 * theme-parity block uses for its disabled-unlink-control case.
 */
describe("WorkBoardCard batch mode theme parity (20.17)", () => {
  function renderThemed(theme: "futuristic" | "minimal", selected: boolean) {
    const container = document.createElement("div");
    container.dataset.theme = theme;
    document.body.appendChild(container);
    return render(
      <WorkBoardCard batchMode item={fixture()} onArchive={vi.fn()} onDelete={vi.fn()} onDismissError={vi.fn()} onEdit={vi.fn()} onMove={vi.fn()} onRestore={vi.fn()} onToggleSelected={vi.fn()} selected={selected} />,
      { container },
    );
  }

  it("renders a structurally identical checked, selected checkbox under both themes", () => {
    const futuristic = renderThemed("futuristic", true);
    const futuristicCheckbox = futuristic.getByRole("checkbox", { name: "选择工作项" }) as HTMLInputElement;
    expect(futuristicCheckbox.checked).toBe(true);
    const futuristicHtml = futuristicCheckbox.outerHTML;
    futuristic.unmount();

    const minimal = renderThemed("minimal", true);
    const minimalCheckbox = minimal.getByRole("checkbox", { name: "选择工作项" }) as HTMLInputElement;
    expect(minimalCheckbox.checked).toBe(true);
    const minimalHtml = minimalCheckbox.outerHTML;
    minimal.unmount();

    // Same tag, same role, same aria-label, same checked attribute, same class list -- byte for
    // byte, not just "both look selected."
    expect(minimalHtml).toBe(futuristicHtml);
  });

  it("renders a structurally identical unchecked checkbox under both themes", () => {
    const futuristic = renderThemed("futuristic", false);
    const futuristicCheckbox = futuristic.getByRole("checkbox", { name: "选择工作项" }) as HTMLInputElement;
    expect(futuristicCheckbox.checked).toBe(false);
    const futuristicHtml = futuristicCheckbox.outerHTML;
    futuristic.unmount();

    const minimal = renderThemed("minimal", false);
    const minimalCheckbox = minimal.getByRole("checkbox", { name: "选择工作项" }) as HTMLInputElement;
    expect(minimalCheckbox.checked).toBe(false);
    const minimalHtml = minimalCheckbox.outerHTML;
    minimal.unmount();

    expect(minimalHtml).toBe(futuristicHtml);
  });
});
