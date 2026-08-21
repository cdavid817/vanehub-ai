import { mockAgents } from "./mock-agent-data";
import {
  createWebAgentMemory,
  listWebAgentMemories,
  simulateWebMemoryIndexInjection,
} from "./web-agent-memory-state";
import { emitWebChatEvent, getWebSessionMessages, publishWebChatEvent } from "./web-chat-state";
import {
  scheduleWebSendMessageTimeout,
  type WebSendMessageContext,
  type WebSendMessageTimeoutId,
} from "./web-send-message-context";
import { normalizeWebPath } from "./web-skill-location";
import { listWebSkillApiAgentBindings, listWebSkills } from "./web-skill-state";
import type { Skill } from "../types/skill";

const mockCompactionTriggerCharacters = 2_000;

export function scheduleWebSendMessageResponse(
  context: WebSendMessageContext,
  timeoutIds: WebSendMessageTimeoutId[],
): void {
  const { assistantMessage, config, input, tokens } = context;
  scheduleWebSendMessageTimeout(timeoutIds, () => {
    emitWebChatEvent({ type: "started", sessionId: input.sessionId, messageId: assistantMessage.id });
  }, 80);
  scheduleMemoryAndContextEvents(context, timeoutIds);
  scheduleSkillEvent(context, timeoutIds);
  tokens.forEach((contentDelta, index) => {
    scheduleWebSendMessageTimeout(timeoutIds, () => {
      publishWebChatEvent({ type: "token", sessionId: input.sessionId, messageId: assistantMessage.id, contentDelta });
    }, 240 + index * 90);
  });
  if (config.thinking) {
    scheduleWebSendMessageTimeout(timeoutIds, () => {
      publishWebChatEvent({
        type: "thinking",
        sessionId: input.sessionId,
        messageId: assistantMessage.id,
        contentDelta: "Mock thinking: checking session context and selected config.",
      });
    }, 180);
  }
  scheduleWebSendMessageTimeout(timeoutIds, () => {
    publishWebChatEvent({
      type: "tool_use",
      sessionId: input.sessionId,
      messageId: assistantMessage.id,
      toolUse: {
        id: `web-tool-${assistantMessage.id}`,
        name: "read_file",
        input: { path: "README.md" },
        output: "Loaded deterministic Web preview content.",
        status: "completed",
      },
    });
  }, 210);
  scheduleRichBlocks(context, timeoutIds);
  scheduleWebSendMessageTimeout(timeoutIds, () => {
    publishWebChatEvent({ type: "completed", sessionId: input.sessionId, messageId: assistantMessage.id });
  }, 320 + tokens.length * 90);
}

function scheduleMemoryAndContextEvents(
  context: WebSendMessageContext,
  timeoutIds: WebSendMessageTimeoutId[],
): void {
  const {
    assistantMessage,
    automaticCompactionEnabled,
    input,
    memoryEnabled,
    session,
    toolAssistedExtractionEnabled,
    userMessage,
  } = context;
  const historyCharacterCount = getWebSessionMessages(input.sessionId).reduce(
    (total, message) => total + message.content.length,
    0,
  );
  if (automaticCompactionEnabled && historyCharacterCount > mockCompactionTriggerCharacters) {
    const afterCharacters = Math.min(
      historyCharacterCount,
      Math.max(userMessage.content.length, Math.ceil(historyCharacterCount * 0.4)),
    );
    const savedCharacters = Math.max(0, historyCharacterCount - afterCharacters);
    scheduleWebSendMessageTimeout(timeoutIds, () => {
      publishWebChatEvent({
        type: "rich_block",
        sessionId: input.sessionId,
        messageId: assistantMessage.id,
        block: {
          id: `web-compaction-${assistantMessage.id}`,
          kind: "card",
          v: 1,
          title: "Conversation compacted",
          bodyMarkdown: "Earlier context was compacted. This evidence contains measurements only and excludes conversation content.",
          tone: "info",
          fields: [
            { label: "Before characters", value: String(historyCharacterCount) },
            { label: "After characters", value: String(afterCharacters) },
            { label: "Characters saved", value: String(savedCharacters) },
            { label: "Before tokens", value: "Unavailable" },
            { label: "After tokens", value: "Unavailable" },
            { label: "Tokens saved", value: "Unavailable" },
            { label: "Measurement quality", value: "characters-only → characters-only" },
            { label: "Trigger source", value: "character-fallback" },
            { label: "Compaction path", value: "compatibility" },
            { label: "Policy version", value: "onepiece-context-production-v1" },
          ],
          meta: {
            evidenceKind: "context-compaction",
            beforeCharacters: historyCharacterCount,
            afterCharacters,
            savedCharacters,
            beforeTokens: null,
            afterTokens: null,
            savedTokens: null,
            beforeQuality: "characters-only",
            afterQuality: "characters-only",
            triggerSource: "character-fallback",
            compactionPath: "compatibility",
            policyVersion: "onepiece-context-production-v1",
          },
        },
      });
    }, 150);
    if (
      memoryEnabled &&
      toolAssistedExtractionEnabled &&
      mockAgents.find((candidate) => candidate.id === session.agentId)?.launch.kind === "api"
    ) {
      scheduleMemoryExtraction(context, timeoutIds, "long conversation", 150);
    }
  }
  if (memoryEnabled && mockAgents.find((candidate) => candidate.id === session.agentId)?.launch.kind === "cli") {
    scheduleMemoryExtraction(context, timeoutIds, "CLI session", 150);
  }
  if (memoryEnabled && listWebAgentMemories().length > 0) {
    const injected = simulateWebMemoryIndexInjection();
    scheduleWebSendMessageTimeout(timeoutIds, () => {
      publishWebChatEvent({
        type: "rich_block",
        sessionId: input.sessionId,
        messageId: assistantMessage.id,
        block: {
          id: `web-memory-applied-${assistantMessage.id}`,
          kind: "card",
          v: 1,
          title: "Memory applied",
          bodyMarkdown: `This response was influenced by memories saved in earlier sessions. Index carried ${injected.indexed} of them; ${injected.selected.length} read in full.`,
          tone: "info",
        },
      });
    }, 150);
  }
}

function scheduleMemoryExtraction(
  context: WebSendMessageContext,
  timeoutIds: WebSendMessageTimeoutId[],
  source: "long conversation" | "CLI session",
  delay: number,
): void {
  const { assistantMessage, input, session, userMessage } = context;
  scheduleWebSendMessageTimeout(timeoutIds, () => {
    const memory = createWebAgentMemory(
      session.agentId,
      session.folder,
      `Extracted from a ${source}: "${userMessage.content.slice(0, 60)}"`,
      "automatic",
    );
    publishWebChatEvent({
      type: "rich_block",
      sessionId: input.sessionId,
      messageId: assistantMessage.id,
      block: {
        id: `web-memory-extracted-${assistantMessage.id}`,
        kind: "card",
        v: 1,
        title: "Memory extracted",
        bodyMarkdown: `Saved for future sessions: "${memory.content}"`,
        tone: "info",
      },
    });
  }, delay);
}

function scheduleSkillEvent(context: WebSendMessageContext, timeoutIds: WebSendMessageTimeoutId[]): void {
  const { assistantMessage, input, session } = context;
  const workspace = session.folder ? normalizeWebPath(session.folder, "Workspace path") : null;
  const names = listWebSkillApiAgentBindings()
    .filter((binding) => binding.agentId === session.agentId)
    .map((binding) => listWebSkills().find((skill) =>
      skill.id === binding.skillId && skill.scope === binding.scope && skill.workspacePath === binding.workspacePath))
    .filter((skill): skill is Skill => skill != null && skill.enabled &&
      (skill.scope === "global" || (skill.scope === "workspace" && skill.workspacePath === workspace)))
    .map((skill) => skill.metadata.name);
  if (names.length === 0) return;
  scheduleWebSendMessageTimeout(timeoutIds, () => {
    publishWebChatEvent({
      type: "rich_block",
      sessionId: input.sessionId,
      messageId: assistantMessage.id,
      block: {
        id: `web-skills-${assistantMessage.id}`,
        kind: "card",
        v: 1,
        title: "Skill instructions applied",
        bodyMarkdown: `This response was influenced by: ${names.join(", ")}.`,
        tone: "info",
      },
    });
  }, 150);
}

function scheduleRichBlocks(context: WebSendMessageContext, timeoutIds: WebSendMessageTimeoutId[]): void {
  const { assistantMessage, input, session } = context;
  scheduleWebSendMessageTimeout(timeoutIds, () => {
    publishWebChatEvent({
      type: "rich_block",
      sessionId: input.sessionId,
      messageId: assistantMessage.id,
      block: {
        id: `web-rich-card-${assistantMessage.id}`,
        kind: "card",
        v: 1,
        title: "Web preview summary",
        bodyMarkdown: "Mock Rich Block rendering is active for this session.",
        tone: "info",
        fields: [
          { label: "Agent", value: session.agentId },
          { label: "Mode", value: session.interactionMode },
        ],
      },
    });
  }, 260);
  scheduleWebSendMessageTimeout(timeoutIds, () => {
    publishWebChatEvent({
      type: "rich_block",
      sessionId: input.sessionId,
      messageId: assistantMessage.id,
      block: {
        id: `web-rich-checklist-${assistantMessage.id}`,
        kind: "checklist",
        v: 1,
        title: "Mock validation",
        items: [
          { id: "contract", text: "Stream event normalized", checked: true },
          { id: "render", text: "Rich Block attached to message", checked: true },
        ],
      },
    });
  }, 300);
}
