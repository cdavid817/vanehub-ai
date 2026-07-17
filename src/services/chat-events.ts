import type { ChatMessage, ChatStreamEvent } from "../types/chat";

export function applyChatEvent(messages: ChatMessage[], event: ChatStreamEvent): ChatMessage[] {
  return messages.map((message) => {
    if (message.id !== event.messageId) return message;
    const updatedAt = new Date().toISOString();
    switch (event.type) {
      case "token":
        return { ...message, content: `${message.content}${event.contentDelta}`, updatedAt };
      case "thinking":
        return { ...message, thinkingContent: `${message.thinkingContent ?? ""}${event.contentDelta}`, updatedAt };
      case "tool_use":
        return { ...message, toolUse: [...(message.toolUse ?? []), event.toolUse], updatedAt };
      case "completed":
        return { ...message, status: "completed", tokenUsage: event.tokenUsage, updatedAt };
      case "failed":
        return { ...message, status: "failed", error: event.error, updatedAt };
      case "cancelled":
        return { ...message, status: "cancelled", updatedAt };
      case "started":
        return message;
    }
  });
}
