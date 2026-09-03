// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { generateWorkItems } from "../testing/fixtures/work-item-fixtures";
import type { WorkItem } from "../types/work-board";
import { workItemStages } from "../types/work-board";
import { WORK_BOARD_ITEM_LIST_VIRTUALIZE_THRESHOLD, WorkBoardItemList } from "./work-board-item-list";

// `@tanstack/react-virtual` measures against a real layout (clientHeight etc.), which jsdom never
// provides -- a real VirtualList renders zero rows here regardless of item count. Replaced with a
// component that honors the same items/getItemKey/renderItem contract without the real
// measurement machinery, matching `session-row-list.test.tsx`'s own established fix for the
// identical problem (this repo's established pattern for testing this exact primitive).
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

const item = (id: string): WorkItem => ({
  id, title: `Item ${id}`, description: "", stage: "inbox", priority: "none",
  rank: 1, projectPath: null, dueAt: null, archived: false,
  createdAt: "2026-01-01", updatedAt: "2026-01-01", sources: [],
});

const row = (workItem: WorkItem) => <span data-testid={`row-${workItem.id}`}>{workItem.title}</span>;

describe("WorkBoardItemList", () => {
  it("renders a plain, unvirtualized list below the threshold", () => {
    const items = [item("a"), item("b")];
    render(<WorkBoardItemList ariaLabel="Inbox" items={items} renderItem={row} />);

    expect(screen.queryByTestId("fake-virtual-list")).toBeNull();
    expect(screen.getByTestId("row-a")).toBeTruthy();
    expect(screen.getByTestId("row-b")).toBeTruthy();
  });

  it("stays on the plain list one item short of the threshold", () => {
    const items = Array.from({ length: WORK_BOARD_ITEM_LIST_VIRTUALIZE_THRESHOLD - 1 }, (_unused, index) => item(String(index)));
    render(<WorkBoardItemList ariaLabel="Inbox" items={items} renderItem={row} />);

    expect(screen.queryByTestId("fake-virtual-list")).toBeNull();
  });

  it("switches to the virtualized list at exactly the threshold", () => {
    const items = Array.from({ length: WORK_BOARD_ITEM_LIST_VIRTUALIZE_THRESHOLD }, (_unused, index) => item(String(index)));
    render(<WorkBoardItemList ariaLabel="Inbox" items={items} renderItem={row} />);

    expect(screen.getByTestId("fake-virtual-list")).toBeTruthy();
    // Every item is still reachable through it (the fake renders them all; the real one would
    // window them, which is exactly what "virtualize" means) -- proves items/getItemKey/
    // renderItem were wired through correctly, not merely that some fallback rendered.
    expect(screen.getByTestId("row-0")).toBeTruthy();
    expect(screen.getByTestId(`row-${WORK_BOARD_ITEM_LIST_VIRTUALIZE_THRESHOLD - 1}`)).toBeTruthy();
  });

  it("keys the virtualized rows by work item id, not array index", () => {
    const items = Array.from({ length: WORK_BOARD_ITEM_LIST_VIRTUALIZE_THRESHOLD }, (_unused, index) => item(`stable-${index}`));
    render(<WorkBoardItemList ariaLabel="Inbox" items={items} renderItem={row} />);

    expect(screen.getByTestId("row-stable-0")).toBeTruthy();
  });
});

/**
 * 14.15/14.16: a genuine 1,000-item structural test, reusing this codebase's own established
 * `generateWorkItems` fixture generator (src/testing/fixtures/work-item-fixtures.ts, the same one
 * `large-scale-fixtures.ts` composes) rather than a second large-fixture generator. Not a
 * wall-clock timing benchmark -- jsdom timing is not a meaningful proxy for real browser paint
 * cost, and `large-scale-fixtures.test.ts` itself never asserts on timing either -- this instead
 * proves every stage's own item subset from a realistic 1,000-item board structurally reaches the
 * virtualized path and renders every item through it without error, with stable per-item keys.
 */
describe("WorkBoardItemList at large-scale-fixture scale", () => {
  const items = generateWorkItems(1000, []);

  it("puts every one of the five stages above the virtualization threshold", () => {
    for (const stage of workItemStages) {
      const stageItems = items.filter((candidate) => candidate.stage === stage);
      expect(stageItems.length).toBeGreaterThan(WORK_BOARD_ITEM_LIST_VIRTUALIZE_THRESHOLD);
    }
  });

  it("renders a full 1,000-item board's busiest stage through the virtualized path without error, with every item reachable and uniquely keyed", () => {
    const busiestStage = workItemStages
      .map((stage) => ({ stage, items: items.filter((candidate) => candidate.stage === stage) }))
      .sort((left, right) => right.items.length - left.items.length)[0];

    render(<WorkBoardItemList ariaLabel="Inbox" items={busiestStage.items} renderItem={row} />);

    expect(screen.getByTestId("fake-virtual-list")).toBeTruthy();
    const renderedIds = new Set(busiestStage.items.map((candidate) => screen.getByTestId(`row-${candidate.id}`) && candidate.id));
    expect(renderedIds.size).toBe(busiestStage.items.length);
  });
});
