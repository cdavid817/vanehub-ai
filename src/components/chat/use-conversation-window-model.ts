import { useEffect, useLayoutEffect, useRef, useState, type UIEvent } from "react";

/** Below this many px from the bottom, a reader is treated as "still following" the conversation. */
const NEAR_BOTTOM_THRESHOLD_PX = 96;

function isNearBottom(element: HTMLElement) {
  return element.scrollHeight - element.scrollTop - element.clientHeight < NEAR_BOTTOM_THRESHOLD_PX;
}

/**
 * design.md Decision 10, phase 1: "先保留现有 DOM，实现稳定 key、锚点、测量和可测试滚动策略" — this
 * model owns the scroll/anchor policy `MessageList` used to implement inline, extracted so phase 2
 * (`@tanstack/react-virtual`'s dynamic measurement, task 10.12) has one seam to integrate at instead
 * of a rewrite. Deliberately height-delta based, not "prepend" vs "append" aware: prepending history
 * above the viewport and a streaming item growing below it are both just "content got taller", and
 * anchoring on the delta handles either without the model needing to know which one happened.
 */
export function anchoredScrollTop(autoScroll: boolean, previousHeight: number, currentHeight: number, currentTop: number) {
  if (autoScroll) return currentHeight;
  return Math.max(0, currentTop + currentHeight - previousHeight);
}

export interface ConversationWindowModel {
  /** Attach to the scrollable container. */
  scrollRef: React.RefObject<HTMLDivElement | null>;
  /** True while the reader is near the bottom — near-bottom growth follows; reading history holds its offset. */
  autoScroll: boolean;
  onScroll: (event: UIEvent<HTMLDivElement>) => void;
  /** Jumps to the latest message and resumes following it — the "new messages" control's action. */
  scrollToBottom: () => void;
  /**
   * Stable-key registration for task 10.10's "selected-item restoration" and "focus": a caller
   * with a `WorkbenchSelection`'s message/tool id (task 10.7, not yet wired to anything that
   * calls these) can scroll or move keyboard focus to that exact item once it exists, independent
   * of the near-bottom auto-anchor policy above.
   */
  registerItemRef: (key: string) => (element: HTMLElement | null) => void;
  scrollToKey: (key: string) => void;
  focusKey: (key: string) => void;
}

/**
 * `items` is the exact array `MessageList` re-renders with — including its own reference identity
 * matters here, not just its length: a streaming update sets a *new* array reference each chunk
 * (React Query's own `setQueriesData`), and the original inline effect this was extracted from
 * fired on that too, as a "snap immediately" complement to the ResizeObserver-based mechanism
 * above (which only smooths *height* changes, and a message that just appeared has not grown into
 * its real height yet on the very first paint). Narrowing this to a length or id-based dependency
 * would be a real behavior change, not a refactor — kept exactly as it was.
 */
export function useConversationWindowModel<T>(items: T[]): ConversationWindowModel {
  const scrollRef = useRef<HTMLDivElement>(null);
  const itemRefs = useRef(new Map<string, HTMLElement>());
  const [autoScroll, setAutoScroll] = useState(true);
  const autoScrollRef = useRef(true);

  useLayoutEffect(() => {
    const element = scrollRef.current;
    if (!element || typeof ResizeObserver === "undefined") return;
    let previousHeight = element.scrollHeight;
    let animationFrame = 0;
    const observer = new ResizeObserver(() => {
      cancelAnimationFrame(animationFrame);
      animationFrame = requestAnimationFrame(() => {
        const currentHeight = element.scrollHeight;
        if (currentHeight !== previousHeight) {
          element.scrollTop = anchoredScrollTop(autoScrollRef.current, previousHeight, currentHeight, element.scrollTop);
        }
        previousHeight = currentHeight;
      });
    });
    observer.observe(element);
    if (element.firstElementChild) observer.observe(element.firstElementChild);
    return () => {
      cancelAnimationFrame(animationFrame);
      observer.disconnect();
    };
  }, []);

  useEffect(() => {
    const element = scrollRef.current;
    if (!element || !autoScroll) return;
    element.scrollTop = element.scrollHeight;
  }, [autoScroll, items]);

  function onScroll(event: UIEvent<HTMLDivElement>) {
    const nextAutoScroll = isNearBottom(event.currentTarget);
    autoScrollRef.current = nextAutoScroll;
    setAutoScroll(nextAutoScroll);
  }

  function scrollToBottom() {
    const element = scrollRef.current;
    if (!element) return;
    element.scrollTop = element.scrollHeight;
    autoScrollRef.current = true;
    setAutoScroll(true);
  }

  function registerItemRef(key: string) {
    return (element: HTMLElement | null) => {
      if (element) itemRefs.current.set(key, element);
      else itemRefs.current.delete(key);
    };
  }

  function scrollToKey(key: string) {
    itemRefs.current.get(key)?.scrollIntoView({ block: "nearest" });
  }

  function focusKey(key: string) {
    itemRefs.current.get(key)?.focus();
  }

  return { autoScroll, focusKey, onScroll, registerItemRef, scrollRef, scrollToBottom, scrollToKey };
}
