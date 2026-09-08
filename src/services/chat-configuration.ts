import type { Session } from "../types/agent";
import type { ChatConfig, SessionExecutionMode, ReasoningDepth } from "../types/chat";
import type { PolicyTemplateName } from "../types/permissions";

const agentDefaults: Record<string, { providerId: string; modelId: string; reasoning: ReasoningDepth | undefined }> = {
  "claude-code": { providerId: "anthropic", modelId: "claude-opus-4-8", reasoning: "high" },
  "codex-cli": { providerId: "openai", modelId: "gpt-5-5", reasoning: "high" },
  "gemini-cli": { providerId: "google", modelId: "gemini-2-5-pro", reasoning: "high" },
  opencode: { providerId: "opencode", modelId: "opencode-default", reasoning: undefined },
  // The real slug list needs an authenticated `agy models` run; until then this falls back to
  // whatever the CLI itself has configured rather than naming a guess.
  "antigravity-cli": { providerId: "google", modelId: "antigravity-default", reasoning: "high" },
  // The expanded CLIs pick their model inside the CLI: `<vendor>-default` names the CLI's own
  // configured model, not an unverified slug; none exposes a reviewed depth flag. Mirrors `chat_profile.rs`.
  ...Object.fromEntries(([
    ["qwen-code", "qwen", "qwen"], ["kimi-cli", "moonshot", "kimi"], ["qoder-cli", "qoder", "qoder"],
    ["codebuddy-code", "codebuddy-international", "codebuddy"], ["copilot-cli", "github-copilot", "copilot"],
    ["cursor-agent-cli", "cursor", "cursor"], ["iflow-cli", "iflow-custom", "iflow"],
  ] as const).map(([agentId, providerId, slug]) => [agentId, { providerId, modelId: `${slug}-default`, reasoning: undefined }])),
  onepiece: { providerId: "onepiece", modelId: "onepiece-active", reasoning: undefined },
};

/**
 * Provider ids that double as an account-environment choice for one Agent. CodeBuddy's default
 * provider is the international environment; the two alternates are the reviewed ones the
 * native runtime starts the CLI against, and they survive normalisation for that Agent only.
 * Mirrors `chat_configuration.rs::is_reviewed_account_environment`.
 */
const reviewedAccountEnvironments: Record<string, readonly string[]> = {
  "codebuddy-code": ["codebuddy-china", "codebuddy-ioa"],
};

const executionModes: readonly SessionExecutionMode[] = ["inherit", "plan", "execute"];
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
    executionMode: "inherit",
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
  const providerId = input.providerId && reviewedAccountEnvironments[session.agentId]?.includes(input.providerId)
    ? input.providerId
    : defaults.providerId;
  const providerMatches = input.providerId === providerId;
  // Accept any non-empty model ID when the provider matches (supports custom models);
  // fall back to the default model when provider mismatch or model ID is missing.
  const modelId = providerMatches && requestedModelId && requestedModelId.trim().length > 0
    ? requestedModelId
    : defaults.modelId;
  const executionMode = executionModes.includes(input.executionMode) ? input.executionMode : "inherit";
  const reasoningDepth = normalizeReasoningDepth(modelId, input.reasoningDepth, defaults.reasoning);
  return {
    agentId: session.agentId,
    interactionMode: session.interactionMode,
    executionMode,
    providerId,
    modelId,
    reasoningDepth,
    streaming: Boolean(input.streaming),
    thinking: Boolean(input.thinking),
    longContext: session.agentId !== "opencode" && Boolean(input.longContext),
  };
}

export function withEffectiveExecutionPolicy(
  config: ChatConfig,
  agentPolicy: PolicyTemplateName,
): ChatConfig {
  const effectiveExecutionPolicy = config.executionMode === "plan" || agentPolicy === "readonly"
    ? "readonly"
    : agentPolicy === "standard" ? "ask" : "allow";
  return { ...config, agentPolicy, effectiveExecutionPolicy };
}
