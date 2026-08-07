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
