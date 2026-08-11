import type { Session } from "../types/agent";
import type { ChatMessage } from "../types/chat";

export type SessionSendBlockReason =
  | "no-session"
  | "archived"
  | "recovery"
  | "active-execution";

export function sessionSendBlockReason(session: Session | null): SessionSendBlockReason | null {
  if (!session) return "no-session";
  if (session.archived) return "archived";
  if (session.recoveryStatus !== "clean") return "recovery";
  if (session.activeExecutionRunId !== null) return "active-execution";
  return null;
}

export function canSendToSession(session: Session | null) {
  return sessionSendBlockReason(session) === null;
}

export function hasLiveSessionGeneration(session: Session | null, messages: ChatMessage[]) {
  const executionRunId = session?.activeExecutionRunId;
  if (!executionRunId) return false;
  return messages.some((message) =>
    message.status === "streaming" && message.executionRunId === executionRunId,
  );
}
