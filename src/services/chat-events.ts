import type { ChatMessage, ChatStreamEvent } from "../types/chat";

function mergeRichBlock(message: ChatMessage, block: NonNullable<ChatMessage["richBlocks"]>[number]) {
  const blocks = message.richBlocks ?? [];
  const index = blocks.findIndex((candidate) => candidate.id === block.id);
  if (index === -1) return [...blocks, block];
  return blocks.map((candidate, candidateIndex) => (candidateIndex === index ? block : candidate));
}

/**
 * Apply a single streaming event, returning a new message array. Each call walks the whole
 * array (O(n)) to find and replace the targeted message, so this is correct but expensive
 * when an agent emits thousands of `token` events per turn. Prefer {@link applyChatEvents}
 * when a batch of events is available — it collapses the work into one array traversal.
 */
export function applyChatEvent(messages: ChatMessage[], event: ChatStreamEvent): ChatMessage[] {
  if (event.type === "turn_status") return messages;
  return messages.map((message) => applyEventToMessage(message, event));
}

/**
 * Apply a batch of stream events in a single traversal of the message array, so a burst of
 * `token` events for the same turn costs one O(n) pass instead of N O(n) passes (and N
 * `setQueryData`-triggered re-renders). Events are folded onto a shallow-cloned array; only
 * messages that an event actually touches are replaced, leaving the rest referentially equal
 * so memoized children skip re-rendering.
 */
export function applyChatEvents(messages: ChatMessage[], events: readonly ChatStreamEvent[]): ChatMessage[] {
  if (events.length === 0) return messages;
  // Index events by the message they target so we can apply all updates to a message in one
  // pass. `turn_status` has no messageId and leaves the thread untouched.
  const eventsByMessage = new Map<string, ChatStreamEvent[]>();
  for (const event of events) {
    if (event.type === "turn_status") continue;
    const bucket = eventsByMessage.get(event.messageId);
    if (bucket) bucket.push(event);
    else eventsByMessage.set(event.messageId, [event]);
  }
  if (eventsByMessage.size === 0) return messages;

  let changed = false;
  const next = messages.map((message) => {
    const bucket = eventsByMessage.get(message.id);
    if (!bucket) return message;
    changed = true;
    return bucket.reduce(applyEventToMessage, message);
  });
  return changed ? next : messages;
}

function applyEventToMessage(message: ChatMessage, event: ChatStreamEvent): ChatMessage {
  if (event.type === "turn_status") return message;
  if (message.id !== event.messageId) return message;
  const updatedAt = new Date().toISOString();
  switch (event.type) {
    case "token":
      return { ...message, content: `${message.content}${event.contentDelta}`, updatedAt };
    case "thinking":
      return { ...message, thinkingContent: `${message.thinkingContent ?? ""}${event.contentDelta}`, updatedAt };
    case "tool_use":
      return { ...message, toolUse: [...(message.toolUse ?? []), event.toolUse], updatedAt };
    case "rich_block":
      return { ...message, richBlocks: mergeRichBlock(message, event.block), updatedAt };
    case "completed":
      return { ...message, status: "completed", tokenUsage: event.tokenUsage, updatedAt };
    case "failed":
      return { ...message, status: "failed", error: event.error, updatedAt };
    case "cancelled":
      return { ...message, status: "cancelled", updatedAt };
    case "started":
      return message;
  }
}
