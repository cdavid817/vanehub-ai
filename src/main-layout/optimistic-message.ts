import type { ChatConfig, ChatFileReference, ChatMessage } from "../types/chat";

export interface SendMessageMutationInput {
  config: ChatConfig;
  content: string;
  fileReferences: ChatFileReference[];
  sessionId: string;
}

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
    sessionSequence: 0,
    status: "completed",
    updatedAt: timestamp,
    executionRunId: null,
  };
}

export function appendMessageIfMissing(messages: ChatMessage[] | undefined, message: ChatMessage) {
  const current = messages ?? [];
  return current.some((candidate) => candidate.id === message.id) ? current : [...current, message];
}

export function removeMessageById(messages: ChatMessage[] | undefined, messageId: string) {
  return (messages ?? []).filter((message) => message.id !== messageId);
}
