import { useEffect, useRef, type ReactNode } from "react";
import { VirtualList, type VirtualListHandle } from "../ui/virtual-list/VirtualList";
import type { Session } from "../types/agent";

/** specs/main-layout-ui/spec.md's own threshold ("at least one thousand sessions") — below it,
 *  the plain `.map()` path every existing test already exercises is unchanged; virtualization is
 *  a new, separately-tested code path that only large views ever reach. */
export const SESSION_LIST_VIRTUALIZE_THRESHOLD = 1000;

/** A `SessionCard` row is a two-line card (agent identity + title, then state/indicators/date)
 *  inside `py-2.5` padding — close enough for `useVirtualizer`'s initial guess; real measurement
 *  (`measureElement`, wired inside `MeasuredVirtualList`) corrects it after the first paint. */
const ESTIMATED_ROW_HEIGHT_PX = 60;

/**
 * 7.10/7.11: virtualizes once a view is large enough to need it, using the session id as the
 * stable item key spec.md itself doesn't literally require but this repo's own virtualization
 * primitive does (`getItemKey`) — the same key `SessionCard`'s own `data-session-id` already
 * carries — and keeps the active session as the scroll anchor across a re-render (a filter
 * change, a route round trip) rather than leaving the scroll position wherever it happened to be.
 */
export function SessionRowList({ activeSessionId, ariaLabel, card, sessions }: {
  activeSessionId: string | null;
  ariaLabel: string;
  card: (session: Session) => ReactNode;
  sessions: Session[];
}) {
  const virtualized = sessions.length >= SESSION_LIST_VIRTUALIZE_THRESHOLD;
  const containerRef = useRef<HTMLDivElement>(null);
  const virtualListRef = useRef<VirtualListHandle>(null);

  useEffect(() => {
    if (!activeSessionId) return;
    if (virtualized) {
      const index = sessions.findIndex((session) => session.id === activeSessionId);
      if (index >= 0) virtualListRef.current?.scrollToIndex(index, "auto");
      return;
    }
    containerRef.current
      ?.querySelector<HTMLElement>(`[data-session-id="${CSS.escape(activeSessionId)}"]`)
      ?.scrollIntoView({ block: "nearest" });
    // Only the identity that changed should re-anchor scroll — re-running on every `sessions`
    // reference (e.g. a fresh sort with the same members) would fight a reader's own scrolling.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeSessionId, virtualized]);

  if (!virtualized) {
    return <div className="grid gap-1" ref={containerRef}>{sessions.map(card)}</div>;
  }
  return (
    <VirtualList
      ariaLabel={ariaLabel}
      className="h-full"
      estimateSize={() => ESTIMATED_ROW_HEIGHT_PX}
      getItemKey={(session) => session.id}
      items={sessions}
      overscan={8}
      ref={virtualListRef}
      renderItem={card}
    />
  );
}
