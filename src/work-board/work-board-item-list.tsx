import type { ReactNode } from "react";
import { VirtualList } from "../ui/virtual-list/VirtualList";
import type { WorkItem } from "../types/work-board";

/**
 * 14.15: below this many items in one column/group, the plain `.map()` path every existing test
 * already exercises is unchanged -- mirrors `SessionRowList`'s own `SESSION_LIST_VIRTUALIZE_
 * THRESHOLD` precedent (src/main-layout/session-row-list.tsx). Lower than that file's 1000: a
 * `WorkBoardCard` is a heavier DOM subtree per row (priority badge, meta chips, a capped source
 * list, a stage-menu/More action row) than a two-line session row, so the per-row cost that
 * motivates virtualizing at all arrives at a smaller count here. Chosen so every stage in the
 * 1,000-item large-scale fixture (weighted 25/25/20/15/15 across five stages -- see
 * work-item-fixtures.ts) lands comfortably above it, so a structural perf test against that
 * fixture genuinely exercises the virtualized path in every stage, not just the busiest one.
 */
export const WORK_BOARD_ITEM_LIST_VIRTUALIZE_THRESHOLD = 40;

/** A `WorkBoardCard` row is roughly a title line, an optional description, a meta-chip row, a
 *  capped source list, and an action row inside `p-3` padding -- close enough for
 *  `useVirtualizer`'s initial guess; real measurement (`measureElement`, wired inside
 *  `MeasuredVirtualList`) corrects it after first paint. */
const ESTIMATED_CARD_HEIGHT_PX = 180;

export interface WorkBoardItemListProps {
  ariaLabel: string;
  className?: string;
  items: WorkItem[];
  renderItem: (item: WorkItem) => ReactNode;
}

/**
 * Shared virtualize-or-plain item list for Board columns, List groups, and the compact Stage List
 * (14.13/14.15), reusing `src/ui/virtual-list/VirtualList` (task 3.10's own wrapper around
 * `MeasuredVirtualList`) rather than a fourth ad hoc rendering path -- `EntityList` was considered
 * and rejected: it imposes a single `role="listbox"`/`aria-activedescendant` roving-selection
 * model (one active row at a time) that fights both this card's own multiple interactive controls
 * (stage menu, More menu, drag handle) and 14.12's own multi-checkbox batch selection, neither of
 * which `SessionRowList`'s plain `VirtualList` usage has to contend with either.
 *
 * `renderItem` is expected to attach its own `key={item.id}` (every existing caller in this
 * directory already does, e.g. `WorkBoardColumn`'s `items.map((item) => <WorkBoardCard key=
 * {item.id} .../>)`), so the plain branch below stays a direct `items.map(renderItem)` -- the same
 * shape `SessionRowList` itself uses. The virtualized branch keys each row by `item.id` via
 * `getItemKey`, so a card's pending-mutation state, drag handle, and batch-mode checkbox all stay
 * attached to the same DOM identity as items are filtered, reordered, or scrolled through the
 * virtualized window, never reassigned by array index.
 */
export function WorkBoardItemList({ ariaLabel, className, items, renderItem }: WorkBoardItemListProps) {
  if (items.length < WORK_BOARD_ITEM_LIST_VIRTUALIZE_THRESHOLD) {
    return <div className={className}>{items.map(renderItem)}</div>;
  }
  return (
    <VirtualList
      ariaLabel={ariaLabel}
      className={className}
      estimateSize={() => ESTIMATED_CARD_HEIGHT_PX}
      getItemKey={(item) => item.id}
      itemClassName="pb-2"
      items={items}
      overscan={6}
      renderItem={renderItem}
    />
  );
}
