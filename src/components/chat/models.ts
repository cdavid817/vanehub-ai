import type { ModelInfo, ReasoningDepth, SessionExecutionMode } from "../../types/chat";

export const REASONING_DEPTHS: ReasoningDepth[] = ["low", "medium", "high", "max"];

export const EXECUTION_MODES: Array<{
  id: SessionExecutionMode;
  label: string;
  description: string;
}> = [
  { id: "inherit", label: "Inherit", description: "Use the Agent policy without widening it" },
  { id: "plan", label: "Plan", description: "Analyze in read-only mode" },
  { id: "execute", label: "Execute", description: "Execute within the Agent policy ceiling" },
];

export const PROVIDER_LABELS: Record<string, string> = {
  anthropic: "Anthropic",
  openai: "OpenAI",
  google: "Google",
  opencode: "OpenCode",
  onepiece: "OnePiece",
};

/**
 * Resolve a model ID to a human-readable label.
 *
 * Precedence:
 * 1. Look up in PROVIDER_MODELS catalog (exact match)
 * 2. Normalize the raw ID: split on dots/hyphens, capitalize each segment,
 *    rejoin with spaces (e.g. "deepseek-v4-pro" → "Deepseek V4 Pro")
 * 3. Fall back to the raw ID itself
 */
export function resolveModelLabel(providerId?: string | null, modelId?: string | null): string {
  if (!modelId) return "";
  if (providerId) {
    const known = PROVIDER_MODELS[providerId]?.find((m) => m.id === modelId);
    if (known) return known.label;
  }
  // Normalize unknown model IDs into a readable label
  const normalized = modelId
    .split(/[.\-_]/)
    .filter(Boolean)
    .map((segment) => segment.charAt(0).toUpperCase() + segment.slice(1).toLowerCase())
    .join(" ");
  return normalized || modelId;
}

export const PROVIDER_MODELS: Record<string, ModelInfo[]> = {
  anthropic: [
    {
      id: "claude-opus-4-8",
      label: "Opus 4.8",
      providerId: "anthropic",
      description: "Highest-capability Claude model",
      supportsReasoning: true,
      maxReasoningDepth: "max",
      supportsLongContext: true,
    },
    {
      id: "claude-sonnet-5",
      label: "Sonnet 5",
      providerId: "anthropic",
      description: "Balanced coding and reasoning model",
      supportsReasoning: true,
      maxReasoningDepth: "max",
      supportsLongContext: true,
    },
    {
      id: "claude-sonnet-4-6",
      label: "Sonnet 4.6",
      providerId: "anthropic",
      description: "Stable general coding model",
      supportsReasoning: true,
      maxReasoningDepth: "high",
      supportsLongContext: true,
    },
    {
      id: "claude-haiku-4-5",
      label: "Haiku 4.5",
      providerId: "anthropic",
      description: "Fast lightweight model",
      supportsReasoning: false,
      maxReasoningDepth: "low",
      supportsLongContext: false,
    },
  ],
  openai: [
    {
      id: "gpt-5-5",
      label: "GPT-5.5",
      providerId: "openai",
      description: "Primary OpenAI reasoning model",
      supportsReasoning: true,
      maxReasoningDepth: "max",
      supportsLongContext: true,
    },
    {
      id: "gpt-5-4",
      label: "GPT-5.4",
      providerId: "openai",
      description: "General high-performance model",
      supportsReasoning: true,
      maxReasoningDepth: "high",
      supportsLongContext: true,
    },
    {
      id: "gpt-5-2-codex",
      label: "GPT-5.2 Codex",
      providerId: "openai",
      description: "Optimized for coding tasks",
      supportsReasoning: true,
      maxReasoningDepth: "high",
      supportsLongContext: true,
    },
    {
      id: "gpt-5-1-codex-max",
      label: "GPT-5.1 Codex Max",
      providerId: "openai",
      description: "Long-running coding task model",
      supportsReasoning: true,
      maxReasoningDepth: "max",
      supportsLongContext: true,
    },
  ],
  google: [
    {
      id: "gemini-2-5-pro",
      label: "Gemini 2.5 Pro",
      providerId: "google",
      description: "High-capability Google model",
      supportsReasoning: true,
      maxReasoningDepth: "high",
      supportsLongContext: true,
    },
    {
      id: "gemini-2-5-flash",
      label: "Gemini 2.5 Flash",
      providerId: "google",
      description: "Fast response model",
      supportsReasoning: true,
      maxReasoningDepth: "medium",
      supportsLongContext: true,
    },
  ],
  opencode: [
    {
      id: "opencode-default",
      label: "OpenCode Default",
      providerId: "opencode",
      description: "OpenCode default model",
      supportsReasoning: false,
      maxReasoningDepth: "low",
      supportsLongContext: false,
    },
  ],
  onepiece: [
    {
      id: "onepiece-active",
      label: "OnePiece",
      providerId: "onepiece",
      supportsReasoning: false,
      maxReasoningDepth: "low",
      supportsLongContext: true,
    },
  ],
};
