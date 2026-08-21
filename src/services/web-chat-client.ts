import type { ChatMessagingService } from "./chat-messaging-service";
import type { ChatMessage } from "../types/chat";
import { normalizeChatConfigForSession, withEffectiveExecutionPolicy } from "./chat-configuration";
import { readWebAppSettings } from "./web-settings-client";
import {
  getWebDefaultPolicyTemplate,
  webPendingApprovals,
  webPrincipalTemplates,
} from "./web-permissions-mock-state";
import { nowIso } from "./web-mock-clock";
import { prependWebAgentRun, setWebAgentRunEvents } from "./web-agent-run-state";
import { selectWebRunner, webRunRunner } from "./web-agent-runner";
import { webRoutedSeatId } from "./web-session-seat-client";
import { scheduleWebSendMessageResponse } from "./web-send-message-response-scheduler";
import { scheduleWebSendMessageApiTools } from "./web-send-message-api-scheduler";
import { findWebSession, updateWebSession } from "./web-session-state";
import {
  cancelWebActiveStream,
  createWebMessageId,
  getWebSessionMessages,
  hasWebActiveStream,
  listWebSessionMessageBuckets,
  publishWebChatEvent,
  setWebActiveStream,
  setWebSessionMessages,
  subscribeWebChatEvents,
} from "./web-chat-state";

/**
 * Resolves a simulated tool call awaiting approval and publishes the resulting `tool_use` event
 * — the same behavior `resolveToolApproval` used to provide directly on this client, extracted
 * to a plain export so `web-permissions-client.ts` can call it (`permissions-approval`'s
 * `resolvePendingApproval` is the new frontend entry point; this is its Web/mock backing).
 */
export function resolveWebMockToolApproval(sessionId: string, callId: string, approved: boolean): boolean {
  findWebSession(sessionId);
  const pending = webPendingApprovals.get(callId);
  if (!pending || pending.sessionId !== sessionId) return false;
  webPendingApprovals.delete(callId);
  publishWebChatEvent({
    type: "tool_use",
    sessionId,
    messageId: pending.messageId,
    toolUse: {
      id: callId,
      name: pending.toolName,
      input: pending.input ?? { command: "echo mock" },
      output: approved ? pending.output ?? "mock\n" : "Denied by user.",
      status: approved ? "completed" : "failed",
    },
  });
  return true;
}

/**
 * Marker that makes the Web/mock runtime simulate a clarification round trip. Web/mock has no
 * model deciding when to ask, so the trigger stands in for that decision.
 */
export { WEB_MOCK_QUESTION_TRIGGER } from "./web-send-message-context";

/**
 * Marker that makes the Web/mock runtime simulate a request to leave plan mode. Same reason as the
 * question trigger: the request blocks until decided, so emitting one every turn would leave every
 * other mock conversation waiting on a card.
 */
export { WEB_MOCK_PLAN_EXIT_TRIGGER } from "./web-send-message-context";

/**
 * Web/mock backing for `resolveAgentQuestion`. Unlike the desktop runtime there is no blocked
 * generation to resume, so "delivered" means only that a tool block in this session was still
 * showing `awaiting_input` — the mock reports the round trip as simulated rather than implying a
 * real wait ended.
 */
function resolveSimulatedQuestion(sessionId: string, callId: string, answer: string): boolean {
  findWebSession(sessionId);
  const message = getWebSessionMessages(sessionId).find((entry) =>
    entry.toolUse?.some((tool) => tool.id === callId && tool.status === "awaiting_input"),
  );
  const pending = message?.toolUse?.find((tool) => tool.id === callId);
  if (!message || !pending) return false;
  publishWebChatEvent({
    type: "tool_use",
    sessionId,
    messageId: message.id,
    toolUse: { ...pending, output: answer, status: "completed" },
  });
  return true;
}

/**
 * Web/mock backing for `resolvePlanExit`. Same simulation as an answer: nothing is blocked, so
 * "delivered" means a matching tool block was still showing `awaiting_input`. The recorded output
 * differs by decision so the mock cannot make a decline look like an approval.
 */
function resolveSimulatedPlanExit(sessionId: string, callId: string, approved: boolean): boolean {
  findWebSession(sessionId);
  const message = getWebSessionMessages(sessionId).find((entry) =>
    entry.toolUse?.some((tool) => tool.id === callId && tool.status === "awaiting_input"),
  );
  const pending = message?.toolUse?.find((tool) => tool.id === callId);
  if (!message || !pending) return false;
  publishWebChatEvent({
    type: "tool_use",
    sessionId,
    messageId: message.id,
    toolUse: {
      ...pending,
      output: approved
        ? "The user approved your plan and this session has left plan mode."
        : "The user did not approve this plan. The session is still in plan mode.",
      status: approved ? "completed" : "failed",
    },
  });
  return true;
}

export const webChatClient: ChatMessagingService = {
  async sendMessage(input) {
    const session = findWebSession(input.sessionId);
    if (session.archived) throw new Error("Archived sessions cannot accept messages.");
    if (session.recoveryStatus !== "clean") {
      throw new Error(`Session recovery state ${session.recoveryStatus} blocks new messages.`);
    }
    if (session.activeExecutionRunId !== null) {
      throw new Error("A generation is already active for this session.");
    }
    const config = normalizeChatConfigForSession(session, input.config);
    const agentPolicy = webPrincipalTemplates.get(session.agentId) ?? getWebDefaultPolicyTemplate();
    const effectiveExecutionPolicy = withEffectiveExecutionPolicy(config, agentPolicy).effectiveExecutionPolicy;
    if (hasWebActiveStream(input.sessionId)) {
      throw new Error("A generation is already active for this session.");
    }
    const selectedRunner = selectWebRunner(input.sessionId, session.agentId, input.runner);
    const timestamp = nowIso();
    const activeSeats = (session.seats ?? []).filter((seat) => seat.leftAt == null);
    const existingMessages = getWebSessionMessages(input.sessionId);
    const nextSequence = existingMessages.reduce(
      (maximum, message) => Math.max(maximum, message.sessionSequence),
      0,
    ) + 1;
    const executionRunId = `web-run-${input.sessionId}-${Date.now()}`;
    prependWebAgentRun({
      id: executionRunId,
      owner: { ownerType: "agent_generation", ownerId: session.agentId },
      links: [{ linkType: "session", linkId: input.sessionId }],
      parentRunId: null,
      state: "running",
      recoveryPolicy: selectedRunner.selection.kind === "ssh" ? "owner_reconciles" : "not_recoverable",
      runner: webRunRunner(selectedRunner),
      retryCount: 0,
      maxRetries: 0,
      reasonCode: null,
      createdAt: timestamp,
      updatedAt: timestamp,
      version: 2,
      lastWitness: `web-simulated-runner-start:${selectedRunner.selection.kind}`,
    });
    setWebAgentRunEvents(executionRunId, []);
    const userMessage: ChatMessage = {
      id: createWebMessageId(),
      sessionId: input.sessionId,
      role: "user",
      content: input.content.trim(),
      status: "completed",
      fileReferences: input.fileReferences,
      createdAt: timestamp,
      updatedAt: timestamp,
      sessionSequence: nextSequence,
      executionRunId,
    };
    const assistantMessage: ChatMessage = {
      id: createWebMessageId(),
      sessionId: input.sessionId,
      role: "assistant",
      // The same routing rule the native runtime applies: a line-leading mention addresses that
      // seat, an unaddressed message continues with whoever last spoke, and an untouched thread
      // goes to the first seat.
      speakerSeatId: webRoutedSeatId(activeSeats, existingMessages, input.content.trim()),
      content: "",
      status: "streaming",
      createdAt: timestamp,
      updatedAt: timestamp,
      sessionSequence: nextSequence + 1,
      executionRunId,
    };
    setWebSessionMessages(input.sessionId, [...existingMessages, userMessage, assistantMessage]);
    updateWebSession(input.sessionId, {
      lifecycleState: "running",
      activeExecutionRunId: executionRunId,
      stateRevision: session.stateRevision + 1,
      historyRevision: session.historyRevision + 2,
    });
    const responseText = `Mock ${session.agentId} response: I received "${userMessage.content}". This is a streaming preview in Web mode.`;
    const settings = readWebAppSettings();
    const context = {
      input,
      session,
      config,
      effectiveExecutionPolicy,
      userMessage,
      assistantMessage,
      tokens: responseText.match(/.{1,6}/g) ?? [responseText],
      memoryEnabled: settings.memoryEnabled,
      toolAssistedExtractionEnabled: settings.memoryToolAssistedChatsEnabled,
      automaticCompactionEnabled: settings.automaticContextCompactionEnabled,
    };
    const timeoutIds: Array<ReturnType<typeof setTimeout>> = [];
    scheduleWebSendMessageResponse(context, timeoutIds);
    scheduleWebSendMessageApiTools(context, timeoutIds);
    setWebActiveStream(input.sessionId, { messageId: assistantMessage.id, timeoutIds });
    return assistantMessage;
  },

  async listMessages(input) {
    findWebSession(input.sessionId);
    const limit = input.limit ?? 50;
    const messages = getWebSessionMessages(input.sessionId);
    const endIndex = input.beforeId
      ? messages.findIndex((message) => message.id === input.beforeId)
      : messages.length;
    const boundedEndIndex = endIndex === -1 ? messages.length : endIndex;
    return messages.slice(Math.max(0, boundedEndIndex - limit), boundedEndIndex);
  },

  async saveMessageFeedback(input) {
    const message = listWebSessionMessageBuckets()
      .flat()
      .find((candidate) => candidate.id === input.messageId);
    if (!message || message.role !== "assistant" || message.status !== "completed") {
      throw new Error("message-not-eligible");
    }
    const currentRevision = message.feedback?.revision ?? 0;
    if (currentRevision !== input.expectedRevision) {
      throw new Error(`feedback-conflict:${currentRevision}`);
    }
    if (input.state === "corrected" && !input.correctionNote?.trim()) {
      throw new Error("invalid-feedback");
    }
    if (input.state === null) {
      message.feedback = { state: null, revision: currentRevision + 1 };
      return message.feedback;
    }
    message.feedback = {
      state: input.state,
      revision: currentRevision + 1,
      ...(input.correctionNote?.trim()
        ? { correctionNote: input.correctionNote.trim().slice(0, 1_000) }
        : {}),
    };
    return message.feedback;
  },

  /**
   * The Web runtime simulates the round trip: nothing is actually blocked on the answer, so this
   * reports delivery only when a matching tool block is still showing `awaiting_input` and marks
   * it completed with the answer, rather than claiming a real generation resumed.
   */
  async resolveAgentQuestion(sessionId: string, callId: string, answer: string) {
    return resolveSimulatedQuestion(sessionId, callId, answer);
  },

  async resolvePlanExit(sessionId: string, callId: string, approved: boolean) {
    return resolveSimulatedPlanExit(sessionId, callId, approved);
  },

  async stopGeneration(sessionId: string) {
    findWebSession(sessionId);
    if (!cancelWebActiveStream(sessionId)) return;
    updateWebSession(sessionId, { lifecycleState: "stopped" });
  },

  async subscribeMessageEvents(sessionId, handler) {
    return subscribeWebChatEvents(sessionId, handler);
  },
};
