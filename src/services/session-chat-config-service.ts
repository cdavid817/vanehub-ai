import type { ChatConfig } from "../types/chat";

export interface SessionChatConfigService {
  getSessionChatConfig(sessionId: string): Promise<ChatConfig>;
  saveSessionChatConfig(sessionId: string, config: ChatConfig): Promise<ChatConfig>;
}
