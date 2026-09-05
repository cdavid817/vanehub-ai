import {
  cancelWebActiveStream,
  deleteWebChatSubscribers,
  deleteWebSessionMessages,
} from "./web-chat-state";
import { deleteWebSessionChatConfig } from "./web-chat-config-state";
import { deleteWebRecoveryReports } from "./web-session-recovery-state";
import {
  emitWebSessionEvent,
  findWebSession,
  getWebActiveSessionId,
  listWebSessions,
  replaceWebSessions,
  setWebActiveSessionId,
} from "./web-session-state";

/** Sessions an in-flight simulated deletion holds, keyed to the operation that holds them. */
const claims = new Map<string, string>();

export const WEB_SESSION_CLAIMED = "session_deletion_in_progress";

export function claimWebSessions(sessionIds: string[], operationId: string): string | null {
  const conflict = sessionIds.find((id) => claims.has(id) && claims.get(id) !== operationId);
  if (conflict) return conflict;
  for (const id of sessionIds) claims.set(id, operationId);
  return null;
}

export function releaseWebSessionClaims(sessionIds: string[], operationId: string): void {
  for (const id of sessionIds) {
    if (claims.get(id) === operationId) claims.delete(id);
  }
}

export function webSessionClaim(sessionId: string): string | null {
  return claims.get(sessionId) ?? null;
}

export function assertWebSessionUnclaimed(sessionId: string): void {
  if (claims.has(sessionId)) throw new Error(`validation error: ${WEB_SESSION_CLAIMED}`);
}

export function resetWebSessionClaims(): void {
  claims.clear();
}

/**
 * Removes one session's mock rows — messages, recovery reports, subscribers, chat config, and the
 * session itself — and clears the active selection only when it named that session. Returns
 * whether it did, so a caller can publish the change exactly once and only when true.
 */
export function deleteWebSessionRecord(sessionId: string): boolean {
  findWebSession(sessionId);
  cancelWebActiveStream(sessionId);
  deleteWebSessionMessages(sessionId);
  deleteWebRecoveryReports(sessionId);
  deleteWebChatSubscribers(sessionId);
  deleteWebSessionChatConfig(sessionId);
  replaceWebSessions(listWebSessions().filter((session) => session.id !== sessionId));
  if (getWebActiveSessionId() === sessionId) {
    setWebActiveSessionId(null);
    emitWebSessionEvent({ kind: "active-session-changed", sessionId: null });
    return true;
  }
  return false;
}
