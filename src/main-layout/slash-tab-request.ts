import type { SessionTabId } from "../session-workspace/session-tab-bar";

export interface SlashTabRequest {
  tab: SessionTabId;
  nonce: number;
}

export interface SlashTabRequestState {
  request: SlashTabRequest | null;
  trackedSessionId: string | null;
}

/**
 * SessionTabs' reset-to-chat and tab-request effects both key on its `activeSession` prop — in
 * MainLayout that prop is `displayedSession`, the loop-inspected session while inspecting and the
 * sidebar's active session otherwise. Those two diverge: inspecting a different session's loop
 * changes `displayedSession` without the sidebar's active session ever changing. A pending
 * slash-tab request has to be invalidated on that same identity, or a change SessionTabs treats as
 * a session switch (entering or leaving inspection) leaves this state still holding a request for
 * a session no longer on screen — which SessionTabs' tab-request effect then reapplies right after
 * its sibling resets to chat.
 */
export function nextSlashTabRequestState(
  request: SlashTabRequest | null,
  trackedSessionId: string | null,
  displayedSessionId: string | null,
): SlashTabRequestState {
  if (displayedSessionId === trackedSessionId) return { request, trackedSessionId };
  return { request: null, trackedSessionId: displayedSessionId };
}
