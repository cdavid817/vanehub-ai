import type { SessionSeat } from "../types/agent";
import type { SessionTabId } from "./session-tab-bar";

export type TabScope = "seat" | "session";

/**
 * Whether a tab shows one seat's work or the whole session's.
 *
 * Three seats mean three terminals, three shells, and three log streams, so those views need to say
 * whose they are. The project-level views do not: the working tree, documents, and report describe
 * the project, which every seat shares.
 *
 * The execution trace stays session-scoped on purpose. It shows a whole round including the
 * handoffs between seats, so splitting it per seat would destroy the thing it exists to show; it
 * distinguishes seats by colour instead.
 */
const seatScopedTabs = new Set<SessionTabId>(["terminal", "shell", "logs"]);

export function tabScope(tab: SessionTabId): TabScope {
  return seatScopedTabs.has(tab) ? "seat" : "session";
}

/**
 * The switcher appears only where it means something. In a one-seat session it would be a control
 * with a single option, so that session keeps exactly the interface it has today.
 */
export function showsSeatSwitcher(tab: SessionTabId, seatCount: number): boolean {
  return tabScope(tab) === "seat" && seatCount > 1;
}

/**
 * The seat a tab's service query should actually carry, which is not the same thing as the seat
 * the switcher is highlighting: a session-scoped tab must not silently narrow because a control
 * that does not apply to it happens to have a selection.
 *
 * Returns null rather than a guessed id when the seat predates stable seat ids — scoping a query
 * to the wrong participant is worse than not scoping it.
 */
export function effectiveSeatId(
  tab: SessionTabId,
  seats: SessionSeat[],
  selectedIndex: number | null,
): string | null {
  if (selectedIndex === null || tabScope(tab) !== "seat" || seats.length <= 1) return null;
  return seats[selectedIndex]?.seatId ?? null;
}
