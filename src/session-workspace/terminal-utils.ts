import type { ChatMessage } from "../types/chat";

export function toolUseCount(messages: readonly ChatMessage[]) {
  return messages.reduce((total, message) => total + (message.toolUse?.length ?? 0), 0);
}
