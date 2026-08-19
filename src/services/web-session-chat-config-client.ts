import type { SessionChatConfigService } from "./session-chat-config-service";
import {
  defaultChatConfigForSession,
  normalizeChatConfigForSession,
  withEffectiveExecutionPolicy,
} from "./chat-configuration";
import { getWebDefaultPolicyTemplate, webPrincipalTemplates } from "./web-permissions-mock-state";
import { emitWebSessionEvent, findWebSession } from "./web-session-state";
import { readWebChatConfigs, writeWebChatConfigs } from "./web-chat-config-state";

export const webSessionChatConfigClient: SessionChatConfigService = {
  async getSessionChatConfig(sessionId) {
    const session = findWebSession(sessionId);
    const stored = readWebChatConfigs()[sessionId];
    const normalized = stored
      ? normalizeChatConfigForSession(session, stored)
      : defaultChatConfigForSession(session);
    const policy = webPrincipalTemplates.get(session.agentId) ?? getWebDefaultPolicyTemplate();
    return withEffectiveExecutionPolicy(normalized, policy);
  },

  async saveSessionChatConfig(sessionId, config) {
    const session = findWebSession(sessionId);
    const normalized = normalizeChatConfigForSession(session, config);
    writeWebChatConfigs({ ...readWebChatConfigs(), [sessionId]: normalized });
    emitWebSessionEvent({ kind: "configuration-changed", sessionId });
    const policy = webPrincipalTemplates.get(session.agentId) ?? getWebDefaultPolicyTemplate();
    return withEffectiveExecutionPolicy(normalized, policy);
  },
};
