import type { Session, SessionLifecycleState } from "../types/agent";
import type { ChatMessage } from "../types/chat";

export type FloatingAssistantStatus = SessionLifecycleState | "unavailable";

export function resolveFloatingAssistantStatus(
  session: Session | null,
  messages: ChatMessage[],
): FloatingAssistantStatus {
  if (!session) return "unavailable";
  if (messages.some((message) => message.status === "streaming")) {
    return session.lifecycleState === "starting" ? "starting" : "running";
  }
  const latestAssistant = [...messages].reverse().find((message) => message.role === "assistant");
  if (latestAssistant?.status === "failed") return "failed";
  if (latestAssistant?.status === "cancelled") return "stopped";
  return session.lifecycleState;
}
