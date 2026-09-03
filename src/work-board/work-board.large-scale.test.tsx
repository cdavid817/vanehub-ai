// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { generateWorkItems } from "../testing/fixtures/work-item-fixtures";
import type { MoveWorkItemInput, WorkItem } from "../types/work-board";

const mocks = vi.hoisted(() => ({
  archive: vi.fn(),
  compact: false,
  items: [] as WorkItem[],
  list: vi.fn(),
  move: vi.fn(),
}));

vi.mock("../hooks/use-media-query", () => ({ useMediaQuery: () => mocks.compact }));
vi.mock("../services/runtime-work-board-client", () => ({
  workBoardService: {
    archiveWorkItem: mocks.archive,
    createWorkItem: vi.fn(),
    deleteWorkItem: vi.fn(),
    linkWorkItemSource: vi.fn(),
    listWorkItems: mocks.list,
    moveWorkItem: mocks.move,
    restoreWorkItem: vi.fn(),
    updateWorkItem: vi.fn(),
  },
}));
// Same established fix as work-board-item-list.test.tsx: jsdom's real MeasuredVirtualList renders
// zero rows, so every stage column here (each comfortably above WORK_BOARD_ITEM_LIST_VIRTUALIZE_
// THRESHOLD=40 at 1,000-item scale) would otherwise show nothing at all. This proves query/mutation/
// drag-alternative behavior through the *real* WorkBoard -> WorkBoardColumn -> WorkBoardCard path;
// the separate DOM-bounded windowing claim itself is proven in work-board-item-list.large-scale.
// test.tsx, which fakes one level lower (`@tanstack/react-virtual`) instead of this whole wrapper.
vi.mock("../ui/virtual-list/VirtualList", () => ({
  VirtualList: ({ ariaLabel, getItemKey, items, renderItem }: {
    ariaLabel: string; getItemKey: (item: WorkItem) => string;
    items: readonly WorkItem[]; renderItem: (item: WorkItem) => React.ReactNode;
  }) => (
    <div aria-label={ariaLabel} data-testid="fake-virtual-list">
      {items.map((item) => <div key={getItemKey(item)}>{renderItem(item)}</div>)}
    </div>
  ),
}));

import { WorkBoard } from "./work-board";

beforeEach(() => {
  localStorage.clear();
  mocks.compact = false;
  mocks.items = generateWorkItems(1000, []);
  mocks.list.mockReset().mockImplementation(async ({ archived = false }: { archived?: boolean } = {}) =>
    mocks.items.filter((item) => item.archived === archived));
  mocks.move.mockReset().mockImplementation(async ({ workItemId, stage }: MoveWorkItemInput) => {
    const item = mocks.items.find((candidate) => candidate.id === workItemId);
    if (item) item.stage = stage;
    return item;
  });
  mocks.archive.mockReset().mockImplementation(async (id: string) => {
    const item = mocks.items.find((candidate) => candidate.id === id);
    if (item) item.archived = true;
    return item;
  });
});

/** Two distinct, deterministic non-archived "inbox" items from the 1,000-item fixture: real card
 *  ids to interact with (`targetItem`) and to prove non-interference for (`unrelatedItem`), rather
 *  than fabricated ones -- both land in the same "收件箱" column so both are reachable without any
 *  extra scrolling/virtualization concerns beyond what the module-level mock above already handles. */
function pickInboxItems() {
  const inbox = mocks.items.filter((item) => item.stage === "inbox" && !item.archived);
  return { targetItem: inbox[0], unrelatedItem: inbox[1] };
}

describe("WorkBoard at 1,000-item scale (21.9)", () => {
  it("fetches the 1,000-item board exactly once, not once per rendered card", async () => {
    const { targetItem } = pickInboxItems();
    render(<WorkBoard />);

    await screen.findByTestId(`work-item-${targetItem.id}`);

    expect(mocks.list).toHaveBeenCalledTimes(1);
  });

  it("moving one card among 1,000 touches only that card -- no full reload, no unrelated re-fetch", async () => {
    const { targetItem, unrelatedItem } = pickInboxItems();
    render(<WorkBoard />);
    await screen.findByTestId(`work-item-${targetItem.id}`);
    const loadCallsAfterMount = mocks.list.mock.calls.length;

    const targetCard = () => screen.getByTestId(`work-item-${targetItem.id}`);
    // WorkItemStageMenu's trigger has no separate aria-label -- its accessible name IS the
    // currently displayed stage (work-board.test.tsx's own established convention for this control).
    fireEvent.click(within(targetCard()).getByRole("button", { name: "收件箱" }));
    fireEvent.click(screen.getByRole("option", { name: "已计划" }));

    // Re-queried by testid fresh, not a captured reference: an optimistic move re-parents the card
    // into a different stage column's own DOM subtree immediately, matching work-board.test.tsx's
    // own established reasoning for the identical situation.
    await waitFor(() => expect(within(screen.getByTestId(`work-item-${targetItem.id}`)).getByRole("button", { name: "已计划" })).toBeTruthy());
    expect(mocks.move).toHaveBeenCalledWith(expect.objectContaining({ workItemId: targetItem.id, stage: "planned" }));

    // No board-wide reload was triggered by this single card's own mutation, even at 1,000-item scale.
    expect(mocks.list.mock.calls.length).toBe(loadCallsAfterMount);
    // An unrelated card among the other 999 kept its own original stage untouched throughout --
    // the mutation's optimistic + reconciled state only ever replaced the target's own array slot
    // (use-work-board-actions.ts's `setItems` `.map()`), never rebuilt the list wholesale.
    expect(within(screen.getByTestId(`work-item-${unrelatedItem.id}`)).getByRole("button", { name: "收件箱" })).toBeTruthy();
  });

  it("opens the stage picker for one card among 1,000 synchronously, listing all five stages", async () => {
    const { targetItem } = pickInboxItems();
    render(<WorkBoard />);
    await screen.findByTestId(`work-item-${targetItem.id}`);

    fireEvent.click(within(screen.getByTestId(`work-item-${targetItem.id}`)).getByRole("button", { name: "收件箱" }));

    // Deliberately synchronous -- no `await`/`waitFor` between the click and this assertion: proves
    // the non-drag stage-picker alternative is already fully populated in the same tick it opens,
    // at 1,000-card scale, not lazily fetched or rendered a frame later. Scoped to the opened
    // listbox itself, not the whole page -- WorkBoardToolbar's own native <select> filters each
    // carry `role="option"` <option> children too (native <select> is otherwise role="combobox",
    // not "listbox"), so an unscoped query would double-count against this component's own popup.
    expect(within(screen.getByRole("listbox")).getAllByRole("option")).toHaveLength(5);
  });
});
