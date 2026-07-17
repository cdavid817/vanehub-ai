import type { Session } from "../types/agent";
import type { ChatConfig, PermissionMode, ReasoningDepth } from "../types/chat";

const agentDefaults: Record<string, { providerId: string; modelId: string; reasoning: ReasoningDepth | undefined }> = {
  "claude-code": { providerId: "anthropic", modelId: "claude-opus-4-8", reasoning: "high" },
  "codex-cli": { providerId: "openai", modelId: "gpt-5-5", reasoning: "high" },
  "gemini-cli": { providerId: "google", modelId: "gemini-2-5-pro", reasoning: "high" },
  opencode: { providerId: "opencode", modelId: "opencode-default", reasoning: undefined },
};

const supportedModels: Record<string, readonly string[]> = {
  "claude-code": ["claude-opus-4-8", "claude-sonnet-5", "claude-sonnet-4-6", "claude-haiku-4-5"],
  "codex-cli": ["gpt-5-5", "gpt-5-4", "gpt-5-2-codex", "gpt-5-1-codex-max"],
  "gemini-cli": ["gemini-2-5-pro", "gemini-2-5-flash"],
  opencode: ["opencode-default"],
};

const permissionModes: readonly PermissionMode[] = ["default", "plan", "agent", "auto"];
const reasoningDepths: readonly ReasoningDepth[] = ["low", "medium", "high", "max"];

const maxReasoningByModel: Record<string, ReasoningDepth | null> = {
  "claude-opus-4-8": "max",
  "claude-sonnet-5": "max",
  "claude-sonnet-4-6": "high",
  "claude-haiku-4-5": null,
  "gpt-5-5": "max",
  "gpt-5-4": "high",
  "gpt-5-2-codex": "high",
  "gpt-5-1-codex-max": "max",
  "gemini-2-5-pro": "high",
  "gemini-2-5-flash": "medium",
  "opencode-default": null,
};

function normalizeReasoningDepth(modelId: string, input: ReasoningDepth | undefined, fallback: ReasoningDepth | undefined) {
  const maximum = maxReasoningByModel[modelId];
  if (!maximum) return undefined;
  const candidate = input && reasoningDepths.includes(input) ? input : fallback;
  if (!candidate) return undefined;
  const maximumIndex = reasoningDepths.indexOf(maximum);
  return reasoningDepths[Math.min(reasoningDepths.indexOf(candidate), maximumIndex)];
}

function defaultsForAgent(agentId: string) {
  return agentDefaults[agentId] ?? agentDefaults["claude-code"];
}

export function defaultChatConfigForSession(session: Session): ChatConfig {
  const defaults = defaultsForAgent(session.agentId);
  return {
    agentId: session.agentId,
    interactionMode: session.interactionMode,
    permissionMode: "default",
    providerId: defaults.providerId,
    modelId: defaults.modelId,
    reasoningDepth: defaults.reasoning,
    streaming: true,
    thinking: true,
    longContext: session.agentId !== "opencode",
  };
}

export function normalizeChatConfigForSession(session: Session, input: ChatConfig): ChatConfig {
  const defaults = defaultsForAgent(session.agentId);
  const requestedModelId = input.modelId;
  const modelId = input.providerId === defaults.providerId && requestedModelId && supportedModels[session.agentId]?.includes(requestedModelId)
    ? requestedModelId
    : defaults.modelId;
  const permissionMode = permissionModes.includes(input.permissionMode) ? input.permissionMode : "default";
  const reasoningDepth = normalizeReasoningDepth(modelId, input.reasoningDepth, defaults.reasoning);
  return {
    agentId: session.agentId,
    interactionMode: session.interactionMode,
    permissionMode,
    providerId: defaults.providerId,
    modelId,
    reasoningDepth,
    streaming: Boolean(input.streaming),
    thinking: Boolean(input.thinking),
    longContext: session.agentId !== "opencode" && Boolean(input.longContext),
  };
}
