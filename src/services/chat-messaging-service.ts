import type {
  ChatMessage,
  ChatStreamEvent,
  MessageFeedback,
  SaveMessageFeedbackInput,
} from "../types/chat";

export interface ChatMessagingService {
  listMessages(input: { sessionId: string; limit?: number; beforeId?: string }): Promise<ChatMessage[]>;
  saveMessageFeedback(input: SaveMessageFeedbackInput): Promise<MessageFeedback>;
  /**
   * Delivers the user's answer to a tool call waiting in `awaiting_input`. Resolves to whether a
   * live waiter received it, so the caller can distinguish a delivered answer from one aimed at a
   * question that already resolved or whose generation is gone.
   */
  resolveAgentQuestion(sessionId: string, callId: string, answer: string): Promise<boolean>;
  /**
   * Delivers the user's decision on an `exit_plan_mode` call waiting in `awaiting_input`. Resolves
   * to whether a live waiter received it — the caller must only change the session's execution
   * mode when it did, so an approval aimed at a dead generation cannot leave a session
   * write-capable on the strength of a decision the model never saw.
   */
  resolvePlanExit(sessionId: string, callId: string, approved: boolean): Promise<boolean>;
  stopGeneration(sessionId: string): Promise<void>;
  subscribeMessageEvents(
    sessionId: string,
    handler: (event: ChatStreamEvent) => void,
  ): Promise<() => void>;
}
