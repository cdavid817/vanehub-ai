// @vitest-environment jsdom

import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { generateWorkItems } from "../testing/fixtures/work-item-fixtures";
import type { WorkItem } from "../types/work-board";
import { workItemStages } from "../types/work-board";
import { WORK_BOARD_ITEM_LIST_VIRTUALIZE_THRESHOLD, WorkBoardItemList } from "./work-board-item-list";

/**
 * 21.9: `work-board-item-list.test.tsx`'s own "at large-scale-fixture scale" block already proves a
 * full 1,000-item board's busiest stage reaches the virtualized path and renders every item with a
 * unique key -- but its `VirtualList` mock renders every item too ("the fake renders them all; the
 * real one would window them," that file's own comment), so it never actually proved the DOM stays
 * *bounded*. This file mocks one level lower instead -- `@tanstack/react-virtual`'s own
 * `useVirtualizer` -- so the real `WorkBoardItemList` -> real `VirtualList` (`MeasuredVirtualList`)
 * path runs and performs its real `virtualItems.map(...)` windowing, against a fake but bounded
 * measurement result standing in for jsdom's real but useless zero-clientHeight one (same standing
 * limitation `session-sidebar.large-scale.test.tsx` documents for the identical primitive).
 */
const FAKE_VIRTUAL_WINDOW = 24; // Stands in for "visible cards + overscan" in a real viewport --
// comfortably below both this file's busiest-stage item count and the real overscan=6 the component
// requests, so a bounded DOM count here can only come from real windowing logic actually running.

interface FakeVirtualizerOptions {
  count: number;
  getItemKey: (index: number) => string;
}

vi.mock("@tanstack/react-virtual", () => ({
  useVirtualizer: (options: FakeVirtualizerOptions) => {
    const windowSize = Math.min(FAKE_VIRTUAL_WINDOW, options.count);
    const virtualItems = Array.from({ length: windowSize }, (_unused, index) => ({
      key: options.getItemKey(index),
      index,
      start: index * 180,
    }));
    return {
      getVirtualItems: () => virtualItems,
      getTotalSize: () => options.count * 180,
      measure: () => undefined,
      measureElement: () => undefined,
      scrollToIndex: () => undefined,
      scrollToOffset: () => undefined,
    };
  },
}));

const row = (item: WorkItem) => <span data-testid={`row-${item.id}`}>{item.title}</span>;

describe("WorkBoardItemList DOM budget at large-scale-fixture scale (21.9)", () => {
  it("keeps rendered cards bounded regardless of the underlying item count", () => {
    const items = generateWorkItems(1000, []);
    const busiestStage = workItemStages
      .map((stage) => items.filter((candidate) => candidate.stage === stage))
      .sort((left, right) => right.length - left.length)[0];
    expect(busiestStage.length).toBeGreaterThan(WORK_BOARD_ITEM_LIST_VIRTUALIZE_THRESHOLD);

    const { container } = render(<WorkBoardItemList ariaLabel="Inbox" items={busiestStage} renderItem={row} />);

    const rendered = container.querySelectorAll("[data-testid^='row-']");
    expect(rendered.length).toBe(FAKE_VIRTUAL_WINDOW);
    expect(rendered.length).toBeLessThan(busiestStage.length);
  });
});
