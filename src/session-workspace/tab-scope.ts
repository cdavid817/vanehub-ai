import type { SessionSeat } from "../types/agent";
import { sessionSurfaceDefinition, showsSessionSeatSwitcher, type SessionSurfaceId } from "./session-surface-registry";

export type TabScope = "seat" | "session";

/**
 * Whether a surface shows one seat's work or the whole session's.
 *
 * Derived from the capability registry rather than a second list. Three seats mean three
 * terminals, three shells, and three log streams, so those views need to say whose they are; the
 * project-level views do not, because the working tree, documents, and report describe the
 * project every seat shares.
 *
 * The execution trace stays session-scoped on purpose. It shows a whole round including the
 * handoffs between seats, so splitting it per seat would destroy the thing it exists to show; it
 * distinguishes seats by colour instead.
 */
export function tabScope(id: SessionSurfaceId): TabScope {
  return sessionSurfaceDefinition(id).scope === "session" ? "session" : "seat";
}

/**
 * The switcher appears only where it means something. In a one-seat session it would be a control
 * with a single option, so that session keeps exactly the interface it has today.
 */
export function showsSeatSwitcher(id: SessionSurfaceId, seatCount: number): boolean {
  return showsSessionSeatSwitcher(id, seatCount);
}

/**
 * The seat a surface's service query should actually carry, which is not the same thing as the
 * seat the switcher is highlighting: a session-scoped surface must not silently narrow because a
 * control that does not apply to it happens to have a selection.
 *
 * Returns null rather than a guessed id when the seat predates stable seat ids — scoping a query
 * to the wrong participant is worse than not scoping it.
 */
export function effectiveSeatId(
  id: SessionSurfaceId,
  seats: SessionSeat[],
  selectedIndex: number | null,
): string | null {
  if (selectedIndex === null || tabScope(id) !== "seat" || seats.length <= 1) return null;
  return seats[selectedIndex]?.seatId ?? null;
}
