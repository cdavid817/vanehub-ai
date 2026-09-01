import {
  forwardRef,
  useImperativeHandle,
  useRef,
  type CSSProperties,
  type ForwardedRef,
  type ReactElement,
  type ReactNode,
} from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { cn } from "../lib/utils";

export interface MeasuredVirtualListHandle {
  measure: () => void;
  scrollToIndex: (index: number, align?: "auto" | "center" | "end" | "start") => void;
  scrollToStart: () => void;
}

export interface MeasuredVirtualListProps<T> {
  ariaLabel: string;
  className?: string;
  /**
   * Extra style for the sized content box. Rows are absolutely positioned and so cannot
   * widen it themselves — a horizontally scrollable list has to state its own width here.
   */
  contentStyle?: CSSProperties;
  estimateSize: () => number;
  getItemKey: (item: T, index: number) => string;
  itemClassName?: string;
  items: readonly T[];
  /**
   * Reports whether the viewport sits at the start of the list.
   *
   * Optional, and every existing caller ignores it. A list that follows new rows needs to know
   * whether the reader is still at the edge those rows arrive at, and that is a fact only the
   * scroll container has — inferring it from row counts would be wrong the moment a row is taller
   * than the estimate.
   */
  onAtStartChange?: (atStart: boolean) => void;
  /** The other edge of `onAtStartChange`, for a list whose new rows arrive at the *end* instead
   *  of the start (task 10.12: the chat transcript, oldest-first, follows the bottom). */
  onAtEndChange?: (atEnd: boolean) => void;
  overscan: number;
  renderItem: (item: T, index: number) => ReactNode;
  testId?: string;
  /** Every existing caller renders a plain list; a selectable one (e.g. `src/ui/entity-list`) needs "listbox". */
  role?: "list" | "listbox";
  /** For a `role="listbox"` caller managing `aria-activedescendant` instead of per-item DOM focus. */
  onKeyDown?: (event: React.KeyboardEvent) => void;
  activeDescendantId?: string;
}

/**
 * How far from the top still counts as "at the start".
 *
 * Not zero: a scroll container settles a pixel or two off, and a reader who is visibly at the top
 * would otherwise be treated as having scrolled away.
 */
const AT_START_TOLERANCE_PX = 8;

/**
 * Much more generous than `AT_START_TOLERANCE_PX`: this edge answers "is the reader still
 * following new rows closely enough to keep auto-following," not "are they exactly at the edge" —
 * a small scroll jitter should not silently drop out of follow (matches
 * `use-conversation-window-model.ts`'s own `NEAR_BOTTOM_THRESHOLD_PX`, the non-virtualized
 * chat path's equivalent tolerance for the identical judgment call).
 */
const AT_END_TOLERANCE_PX = 96;

function MeasuredVirtualListInner<T>(
  {
    activeDescendantId,
    ariaLabel,
    className,
    contentStyle,
    estimateSize,
    getItemKey,
    itemClassName,
    items,
    onAtEndChange,
    onAtStartChange,
    onKeyDown,
    overscan,
    renderItem,
    role = "list",
    testId,
  }: MeasuredVirtualListProps<T>,
  ref: ForwardedRef<MeasuredVirtualListHandle>,
) {
  const scrollElementRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: items.length,
    estimateSize,
    getItemKey: (index) => getItemKey(items[index], index),
    getScrollElement: () => scrollElementRef.current,
    overscan,
  });
  const virtualItems = virtualizer.getVirtualItems();

  useImperativeHandle(ref, () => ({
    measure: () => virtualizer.measure(),
    scrollToIndex: (index, align = "auto") => virtualizer.scrollToIndex(index, { align }),
    scrollToStart: () => virtualizer.scrollToOffset(0, { align: "start" }),
  }), [virtualizer]);

  return (
    <div
      aria-activedescendant={activeDescendantId}
      aria-label={ariaLabel}
      className={cn("relative overflow-y-auto", className)}
      data-rendered-count={virtualItems.length}
      data-testid={testId}
      data-virtual-count={items.length}
      onKeyDown={onKeyDown}
      onScroll={onAtStartChange || onAtEndChange
        ? (event) => {
            if (onAtStartChange) onAtStartChange(event.currentTarget.scrollTop <= AT_START_TOLERANCE_PX);
            if (onAtEndChange) {
              const { clientHeight, scrollHeight, scrollTop } = event.currentTarget;
              onAtEndChange(scrollHeight - scrollTop - clientHeight <= AT_END_TOLERANCE_PX);
            }
          }
        : undefined}
      ref={scrollElementRef}
      role={role}
      tabIndex={0}
    >
      <div className="relative w-full" style={{ ...virtualContentStyle(virtualizer.getTotalSize()), ...contentStyle }}>
        {virtualItems.map((virtualItem) => (
          <div
            className={cn("absolute left-0 top-0 w-full", itemClassName)}
            data-index={virtualItem.index}
            key={virtualItem.key}
            ref={virtualizer.measureElement}
            role="presentation"
            style={virtualItemStyle(virtualItem.start)}
          >
            {renderItem(items[virtualItem.index], virtualItem.index)}
          </div>
        ))}
      </div>
    </div>
  );
}

function virtualContentStyle(height: number): CSSProperties {
  return { height };
}

function virtualItemStyle(start: number): CSSProperties {
  return { transform: `translateY(${start}px)` };
}

export const MeasuredVirtualList = forwardRef(MeasuredVirtualListInner) as <T>(
  props: MeasuredVirtualListProps<T> & { ref?: ForwardedRef<MeasuredVirtualListHandle> },
) => ReactElement;
