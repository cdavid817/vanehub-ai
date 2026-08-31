import type { ReactNode } from "react";
import { VirtualList } from "../ui/virtual-list/VirtualList";
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
 * 7.10: virtualizes once a view is large enough to need it, using the session id as the stable
 * item key spec.md itself doesn't literally require but this repo's own virtualization primitive
 * does (`getItemKey`) — the same key `SessionCard`'s own `data-session-id` already carries.
 */
export function SessionRowList({ ariaLabel, card, sessions }: {
  ariaLabel: string;
  card: (session: Session) => ReactNode;
  sessions: Session[];
}) {
  if (sessions.length < SESSION_LIST_VIRTUALIZE_THRESHOLD) {
    return <div className="grid gap-1">{sessions.map(card)}</div>;
  }
  return (
    <VirtualList
      ariaLabel={ariaLabel}
      className="h-full"
      estimateSize={() => ESTIMATED_ROW_HEIGHT_PX}
      getItemKey={(session) => session.id}
      items={sessions}
      overscan={8}
      renderItem={card}
    />
  );
}
