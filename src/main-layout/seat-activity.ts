import type { ChatMessage } from "../types/chat";

export type SeatActivity = "idle" | "starting" | "thinking" | "tool" | "streaming" | "completed" | "failed";

export function seatActivity(
  messages: readonly ChatMessage[],
  seatId: string | undefined,
  speaking: boolean,
): SeatActivity {
  const latest = seatId
    ? messages.filter((message) => message.speakerSeatId === seatId && message.role === "assistant")
      .reduce<ChatMessage | null>((current, message) =>
        !current || message.sessionSequence > current.sessionSequence ? message : current, null)
    : null;
  if (speaking || latest?.status === "streaming") {
    if (latest?.status === "failed") return "failed";
    if (latest?.status === "streaming") {
      if (latest.toolUse?.some((tool) => tool.status === "running" || tool.status === "pending")) return "tool";
      if (latest.content.length > 0) return "streaming";
      if ((latest.thinkingContent?.length ?? 0) > 0) return "thinking";
    }
    return "starting";
  }
  if (latest?.status === "failed") return "failed";
  if (latest?.status === "completed") return "completed";
  return "idle";
}
