import { useEffect, useRef, useState } from "react";
import type { MeasuredVirtualListHandle } from "../measured-virtual-list";

export interface VirtualizedMessageWindow {
  listRef: React.RefObject<MeasuredVirtualListHandle | null>;
  /** True while the reader is near the bottom — mirrors `ConversationWindowModel`'s own field
   *  and meaning, computed from `MeasuredVirtualList`'s `onAtEndChange` instead of a raw
   *  `scrollTop` read, since the scroll container here is virtualized. */
  autoScroll: boolean;
  /** Wired straight to `MeasuredVirtualList`'s `onAtEndChange` prop. */
  onAtEndChange: (atEnd: boolean) => void;
  scrollToBottom: () => void;
}

/**
 * The virtualized counterpart to `useConversationWindowModel` (task 10.12; design.md Decision 10
 * phase 2). Deliberately a separate, smaller model rather than a variant of that one: the
 * non-virtualized model's `ResizeObserver`-based height-delta anchor exists to keep the reader's
 * exact scroll position stable while *reading history* as off-screen content changes height —
 * `@tanstack/react-virtual`'s own `measureElement` already renumbers row offsets as each row's
 * real height is measured, which is that same job, done natively, for the virtualized case. What
 * is left for this model to own is strictly the "am I still following the bottom, and if so keep
 * me there" policy — the one behavior `measureElement` has no opinion on.
 *
 * `items`, not just a count: matches `useConversationWindowModel`'s own reasoning exactly — a
 * streaming update changes the array *reference* every token without changing its length, and
 * this needs to reassert `scrollToIndex` on every one of those to keep pinned to the growing last
 * row's bottom edge, not just when a whole new row is appended.
 */
export function useVirtualizedMessageWindow<T>(items: T[]): VirtualizedMessageWindow {
  const listRef = useRef<MeasuredVirtualListHandle>(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const autoScrollRef = useRef(true);

  useEffect(() => {
    if (!autoScrollRef.current || items.length === 0) return;
    listRef.current?.scrollToIndex(items.length - 1, "end");
  }, [items]);

  function onAtEndChange(atEnd: boolean) {
    autoScrollRef.current = atEnd;
    setAutoScroll(atEnd);
  }

  function scrollToBottom() {
    if (items.length === 0) return;
    listRef.current?.scrollToIndex(items.length - 1, "end");
    autoScrollRef.current = true;
    setAutoScroll(true);
  }

  return { autoScroll, listRef, onAtEndChange, scrollToBottom };
}
