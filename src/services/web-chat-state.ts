import type { ChatMessage, ChatStreamEvent } from "../types/chat";
import type { Session } from "../types/agent";
import { upsertToolUse } from "./tool-use";
import { nowIso } from "./web-mock-clock";
import {
  findWebAgentRun,
  isTerminalWebRunState,
  updateWebAgentRun,
} from "./web-agent-run-state";
import { findWebSession, updateWebSession } from "./web-session-state";

/** One in-flight generation per session: the assistant message it is filling and every timer it scheduled. */
export interface WebActiveStream {
  messageId: string;
  timeoutIds: Array<ReturnType<typeof setTimeout>>;
}

// These bindings are owned here and never exported. An exported mutable binding re-imported from
// two modules gives two divergent copies of the mock world, which surfaces as one UI panel showing
// stale data while another shows fresh. Callers reach the state through the accessors below, which
// read the live binding on every call and so cannot fork.
let nextMessageId = 1;
const messagesBySession = new Map<string, ChatMessage[]>();
const subscribersBySession = new Map<string, Set<(event: ChatStreamEvent) => void>>();
const activeStreams = new Map<string, WebActiveStream>();

/** Every call site read and advanced in one step, so the accessor does too. */
export function createWebMessageId(): string {
  const id = `web-message-${nextMessageId}`;
  nextMessageId += 1;
  return id;
}

/** Returns the live array, matching what a direct read of the binding returned. */
export function getWebSessionMessages(sessionId: string): ChatMessage[] {
  return messagesBySession.get(sessionId) ?? [];
}

export function setWebSessionMessages(sessionId: string, nextMessages: ChatMessage[]): void {
  messagesBySession.set(sessionId, nextMessages);
}

export function deleteWebSessionMessages(sessionId: string): void {
  messagesBySession.delete(sessionId);
}

/** Feedback is addressed by message id alone, so its lookup spans every session's bucket. */
export function listWebSessionMessageBuckets(): ChatMessage[][] {
  return Array.from(messagesBySession.values());
}

function upsertMessage(message: ChatMessage) {
  const messages = getWebSessionMessages(message.sessionId);
  const index = messages.findIndex((candidate) => candidate.id === message.id);
  if (index === -1) {
    setWebSessionMessages(message.sessionId, [...messages, message]);
    return;
  }
  const nextMessages = [...messages];
  nextMessages[index] = message;
  setWebSessionMessages(message.sessionId, nextMessages);
}

export function emitWebChatEvent(event: ChatStreamEvent): void {
  const subscribers = subscribersBySession.get(event.sessionId);
  subscribers?.forEach((handler) => handler(event));
}

function finishWebGeneration(sessionId: string, lifecycleState: Session["lifecycleState"]) {
  const session = findWebSession(sessionId);
  const runId = session.activeExecutionRunId;
  if (runId) {
    const run = findWebAgentRun(runId);
    if (run && !isTerminalWebRunState(run.state)) {
      updateWebAgentRun(
        run.id,
        run.version,
        lifecycleState === "idle" ? "completed" : lifecycleState === "failed" ? "failed" : "cancelled",
      );
    }
  }
  updateWebSession(sessionId, {
    lifecycleState,
    activeExecutionRunId: null,
    stateRevision: session.stateRevision + 1,
  });
}

function applyStreamEvent(event: ChatStreamEvent) {
  // The turn status belongs to the session, not to any message.
  if (event.type === "turn_status") return;
  const messages = getWebSessionMessages(event.sessionId);
  const message = messages.find((candidate) => candidate.id === event.messageId);
  if (!message) return;
  const timestamp = nowIso();
  if (event.type === "token") {
    upsertMessage({ ...message, content: `${message.content}${event.contentDelta}`, updatedAt: timestamp });
  } else if (event.type === "thinking") {
    upsertMessage({
      ...message,
      thinkingContent: `${message.thinkingContent ?? ""}${event.contentDelta}`,
      updatedAt: timestamp,
    });
  } else if (event.type === "tool_use") {
    upsertMessage({ ...message, toolUse: upsertToolUse(message.toolUse ?? [], event.toolUse), updatedAt: timestamp });
  } else if (event.type === "rich_block") {
    const blocks = message.richBlocks ?? [];
    const blockIndex = blocks.findIndex((block) => block.id === event.block.id);
    const richBlocks =
      blockIndex === -1
        ? [...blocks, event.block]
        : blocks.map((block, index) => (index === blockIndex ? event.block : block));
    upsertMessage({ ...message, richBlocks, updatedAt: timestamp });
  } else if (event.type === "completed") {
    upsertMessage({ ...message, status: "completed", tokenUsage: event.tokenUsage, updatedAt: timestamp });
    activeStreams.delete(event.sessionId);
    finishWebGeneration(event.sessionId, "idle");
  } else if (event.type === "failed") {
    upsertMessage({ ...message, status: "failed", error: event.error, updatedAt: timestamp });
    activeStreams.delete(event.sessionId);
    finishWebGeneration(event.sessionId, "failed");
  } else if (event.type === "cancelled") {
    upsertMessage({ ...message, status: "cancelled", updatedAt: timestamp });
    activeStreams.delete(event.sessionId);
    finishWebGeneration(event.sessionId, "stopped");
  }
}

export function publishWebChatEvent(event: ChatStreamEvent): void {
  applyStreamEvent(event);
  emitWebChatEvent(event);
}

export function hasWebActiveStream(sessionId: string): boolean {
  return activeStreams.has(sessionId);
}

export function setWebActiveStream(sessionId: string, stream: WebActiveStream): void {
  activeStreams.set(sessionId, stream);
}

export function deleteWebActiveStream(sessionId: string): void {
  activeStreams.delete(sessionId);
}

export function cancelWebActiveStream(sessionId: string): boolean {
  const activeStream = activeStreams.get(sessionId);
  if (!activeStream) return false;
  activeStream.timeoutIds.forEach((timeoutId) => clearTimeout(timeoutId));
  activeStreams.delete(sessionId);
  publishWebChatEvent({ type: "cancelled", sessionId, messageId: activeStream.messageId });
  return true;
}

export function subscribeWebChatEvents(
  sessionId: string,
  handler: (event: ChatStreamEvent) => void,
): () => void {
  const subscribers = subscribersBySession.get(sessionId) ?? new Set<(event: ChatStreamEvent) => void>();
  subscribers.add(handler);
  subscribersBySession.set(sessionId, subscribers);
  return () => {
    const currentSubscribers = subscribersBySession.get(sessionId);
    currentSubscribers?.delete(handler);
    if (currentSubscribers?.size === 0) {
      subscribersBySession.delete(sessionId);
    }
  };
}

export function deleteWebChatSubscribers(sessionId: string): void {
  subscribersBySession.delete(sessionId);
}
