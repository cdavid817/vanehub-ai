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
  estimateSize: () => number;
  getItemKey: (item: T, index: number) => string;
  itemClassName?: string;
  items: readonly T[];
  overscan: number;
  renderItem: (item: T, index: number) => ReactNode;
  testId?: string;
}

function MeasuredVirtualListInner<T>(
  {
    ariaLabel,
    className,
    estimateSize,
    getItemKey,
    itemClassName,
    items,
    overscan,
    renderItem,
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
      aria-label={ariaLabel}
      className={cn("relative overflow-y-auto", className)}
      data-rendered-count={virtualItems.length}
      data-testid={testId}
      data-virtual-count={items.length}
      ref={scrollElementRef}
      role="list"
      tabIndex={0}
    >
      <div className="relative w-full" style={virtualContentStyle(virtualizer.getTotalSize())}>
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
