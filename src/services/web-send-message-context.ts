import type { Session } from "../types/agent";
import type { ChatMessage } from "../types/chat";
import type { ChatMessagingService } from "./chat-messaging-service";
import type { normalizeChatConfigForSession, withEffectiveExecutionPolicy } from "./chat-configuration";

export type WebSendMessageInput = Parameters<ChatMessagingService["sendMessage"]>[0];
export type WebSendMessageConfig = ReturnType<typeof normalizeChatConfigForSession>;
export type WebEffectiveExecutionPolicy = ReturnType<
  typeof withEffectiveExecutionPolicy
>["effectiveExecutionPolicy"];
export type WebSendMessageTimeoutId = ReturnType<typeof setTimeout>;

export const WEB_MOCK_QUESTION_TRIGGER = "[ask-me]";
export const WEB_MOCK_PLAN_EXIT_TRIGGER = "[plan-done]";

/** Captures values before scheduling so callbacks cannot observe a later mutable session snapshot. */
export interface WebSendMessageContext {
  readonly input: WebSendMessageInput;
  readonly session: Session;
  readonly config: WebSendMessageConfig;
  readonly effectiveExecutionPolicy: WebEffectiveExecutionPolicy;
  readonly userMessage: ChatMessage;
  readonly assistantMessage: ChatMessage;
  readonly tokens: readonly string[];
  readonly memoryEnabled: boolean;
  readonly toolAssistedExtractionEnabled: boolean;
  readonly automaticCompactionEnabled: boolean;
}

export function scheduleWebSendMessageTimeout(
  timeoutIds: WebSendMessageTimeoutId[],
  callback: () => void,
  delay: number,
): void {
  timeoutIds.push(setTimeout(callback, delay));
}
