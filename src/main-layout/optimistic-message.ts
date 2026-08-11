import type { ChatFileReference, ChatMessage } from "../types/chat";

export function createOptimisticUserMessage({
  content,
  fileReferences,
  id,
  now = new Date(),
  sessionId,
}: {
  content: string;
  fileReferences: ChatFileReference[];
  id: string;
  now?: Date;
  sessionId: string;
}): ChatMessage {
  const timestamp = now.toISOString();
  return {
    content,
    createdAt: timestamp,
    fileReferences,
    id,
    role: "user",
    sessionId,
    status: "completed",
    updatedAt: timestamp,
  };
}

export function appendMessageIfMissing(messages: ChatMessage[] | undefined, message: ChatMessage) {
  const current = messages ?? [];
  return current.some((candidate) => candidate.id === message.id) ? current : [...current, message];
}

export function removeMessageById(messages: ChatMessage[] | undefined, messageId: string) {
  return (messages ?? []).filter((message) => message.id !== messageId);
}
