import type { ModelInfo, PermissionMode, ReasoningDepth } from "../../types/chat";

export const REASONING_DEPTHS: ReasoningDepth[] = ["low", "medium", "high", "max"];

export const PERMISSION_MODES: Array<{
  id: PermissionMode;
  label: string;
  description: string;
}> = [
  { id: "default", label: "Default", description: "按 Agent 默认权限执行" },
  { id: "plan", label: "Plan", description: "先规划，再等待确认" },
  { id: "agent", label: "Agent", description: "允许 Agent 自主推进任务" },
  { id: "auto", label: "Auto", description: "自动选择合适执行方式" },
];

export const PROVIDER_LABELS: Record<string, string> = {
  anthropic: "Anthropic",
  openai: "OpenAI",
  google: "Google",
  opencode: "OpenCode",
};

export const PROVIDER_MODELS: Record<string, ModelInfo[]> = {
  anthropic: [
    {
      id: "claude-opus-4-8",
      label: "Opus 4.8",
      providerId: "anthropic",
      description: "最高能力 Claude 模型",
      supportsReasoning: true,
      maxReasoningDepth: "max",
      supportsLongContext: true,
    },
    {
      id: "claude-sonnet-5",
      label: "Sonnet 5",
      providerId: "anthropic",
      description: "均衡的编码与推理模型",
      supportsReasoning: true,
      maxReasoningDepth: "max",
      supportsLongContext: true,
    },
    {
      id: "claude-sonnet-4-6",
      label: "Sonnet 4.6",
      providerId: "anthropic",
      description: "稳定的通用编码模型",
      supportsReasoning: true,
      maxReasoningDepth: "high",
      supportsLongContext: true,
    },
    {
      id: "claude-haiku-4-5",
      label: "Haiku 4.5",
      providerId: "anthropic",
      description: "快速轻量模型",
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
      description: "OpenAI 主力推理模型",
      supportsReasoning: true,
      maxReasoningDepth: "max",
      supportsLongContext: true,
    },
    {
      id: "gpt-5-4",
      label: "GPT-5.4",
      providerId: "openai",
      description: "通用高性能模型",
      supportsReasoning: true,
      maxReasoningDepth: "high",
      supportsLongContext: true,
    },
    {
      id: "gpt-5-2-codex",
      label: "GPT-5.2 Codex",
      providerId: "openai",
      description: "编码任务优化",
      supportsReasoning: true,
      maxReasoningDepth: "high",
      supportsLongContext: true,
    },
    {
      id: "gpt-5-1-codex-max",
      label: "GPT-5.1 Codex Max",
      providerId: "openai",
      description: "长任务编码模型",
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
      description: "Google 高能力模型",
      supportsReasoning: true,
      maxReasoningDepth: "high",
      supportsLongContext: true,
    },
    {
      id: "gemini-2-5-flash",
      label: "Gemini 2.5 Flash",
      providerId: "google",
      description: "快速响应模型",
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
      description: "OpenCode 默认模型",
      supportsReasoning: false,
      maxReasoningDepth: "low",
      supportsLongContext: false,
    },
  ],
};
