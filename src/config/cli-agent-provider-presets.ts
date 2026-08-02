import type {
  CliConfigAgentId,
  CliConfigPayload,
  CliConfigPreset,
} from "../types/cli-agent-config";

const CATALOG_VERSION = 1;

function claude(
  id: string,
  displayName: string,
  baseUrl: string,
  model: string,
  category: CliConfigPreset["category"] = "common",
): CliConfigPreset {
  return {
    id: `claude-code-${id}`,
    catalogVersion: CATALOG_VERSION,
    displayName,
    description: `${displayName} · Claude Code`,
    category,
    agentId: "claude-code",
    deprecated: false,
    payload: {
      kind: "claude-code",
      baseUrl,
      authMode: category === "official" ? "none" : "auth-token",
      model,
      haikuModel: model,
      sonnetModel: model,
      opusModel: model,
      advancedEnv: {},
    },
  };
}

function codex(
  id: string,
  displayName: string,
  baseUrl: string,
  model: string,
  wireApi: "responses" | "chat" = "chat",
  category: CliConfigPreset["category"] = "common",
): CliConfigPreset {
  return {
    id: `codex-cli-${id}`,
    catalogVersion: CATALOG_VERSION,
    displayName,
    description: `${displayName} · Codex`,
    category,
    agentId: "codex-cli",
    deprecated: false,
    payload: {
      kind: "codex-cli",
      providerId: id,
      baseUrl,
      model,
      wireApi,
      reasoningEffort: "medium",
      authStrategy: category === "official" ? "preserve-official" : "bearer-token",
      advancedToml: {},
    },
  };
}

function opencode(
  id: string,
  displayName: string,
  baseUrl: string,
  model: string,
  npm = "@ai-sdk/openai-compatible",
  category: CliConfigPreset["category"] = "common",
): CliConfigPreset {
  return {
    id: `opencode-${id}`,
    catalogVersion: CATALOG_VERSION,
    displayName,
    description: `${displayName} · OpenCode`,
    category,
    agentId: "opencode",
    deprecated: false,
    payload: {
      kind: "opencode",
      providerId: id,
      providerName: displayName,
      npm,
      baseUrl,
      headers: {},
      models: [{ id: model, name: model }],
      defaultModel: model,
    },
  };
}

export const cliAgentProviderPresets: readonly CliConfigPreset[] = [
  claude("anthropic", "Anthropic", "https://api.anthropic.com", "claude-sonnet-4-6", "official"),
  claude("openrouter", "OpenRouter", "https://openrouter.ai/api", "anthropic/claude-sonnet-4.6"),
  claude("deepseek", "DeepSeek", "https://api.deepseek.com/anthropic", "deepseek-chat"),
  claude("zhipu-glm", "Zhipu GLM", "https://open.bigmodel.cn/api/anthropic", "glm-4.7"),
  claude("kimi", "Kimi / Moonshot", "https://api.moonshot.cn/anthropic", "kimi-k2.5"),
  claude("siliconflow", "SiliconFlow", "https://api.siliconflow.cn", "deepseek-ai/DeepSeek-V3.2"),
  claude("bailian", "Alibaba Bailian", "https://dashscope.aliyuncs.com/apps/anthropic", "qwen3.5-plus"),
  claude("volcengine-ark", "Volcengine Ark", "https://ark.cn-beijing.volces.com/api/coding", "ark-code-latest"),

  codex("openai", "OpenAI", "https://api.openai.com/v1", "gpt-5.4", "responses", "official"),
  codex("openrouter", "OpenRouter", "https://openrouter.ai/api/v1", "openai/gpt-5.4", "responses"),
  codex("deepseek", "DeepSeek", "https://api.deepseek.com", "deepseek-chat"),
  codex("zhipu-glm", "Zhipu GLM", "https://open.bigmodel.cn/api/coding/paas/v4", "glm-4.7"),
  codex("kimi", "Kimi / Moonshot", "https://api.moonshot.cn/v1", "kimi-k2.5"),
  codex("siliconflow", "SiliconFlow", "https://api.siliconflow.cn/v1", "deepseek-ai/DeepSeek-V3.2"),
  codex("bailian", "Alibaba Bailian", "https://dashscope.aliyuncs.com/compatible-mode/v1", "qwen3.5-plus", "responses"),
  codex("volcengine-ark", "Volcengine Ark", "https://ark.cn-beijing.volces.com/api/v3", "ark-code-latest"),

  opencode("anthropic", "Anthropic", "https://api.anthropic.com/v1", "claude-sonnet-4-6", "@ai-sdk/anthropic", "official"),
  opencode("openai", "OpenAI", "https://api.openai.com/v1", "gpt-5.4", "@ai-sdk/openai", "official"),
  opencode("openrouter", "OpenRouter", "https://openrouter.ai/api/v1", "anthropic/claude-sonnet-4.6", "@ai-sdk/anthropic"),
  opencode("deepseek", "DeepSeek", "https://api.deepseek.com/v1", "deepseek-chat"),
  opencode("zhipu-glm", "Zhipu GLM", "https://open.bigmodel.cn/api/coding/paas/v4", "glm-4.7"),
  opencode("kimi", "Kimi / Moonshot", "https://api.moonshot.cn/v1", "kimi-k2.5"),
  opencode("siliconflow", "SiliconFlow", "https://api.siliconflow.cn/v1", "deepseek-ai/DeepSeek-V3.2"),
  opencode("bailian", "Alibaba Bailian", "https://dashscope.aliyuncs.com/compatible-mode/v1", "qwen3.5-plus"),
  opencode("volcengine-ark", "Volcengine Ark", "https://ark.cn-beijing.volces.com/api/v3", "ark-code-latest"),
] as const;

export function getCliConfigPresets(agentId: CliConfigAgentId): CliConfigPreset[] {
  return cliAgentProviderPresets
    .filter((preset) => preset.agentId === agentId)
    .map((preset) => ({ ...preset, payload: structuredClone(preset.payload) }));
}

export function createCustomCliConfigPayload(agentId: CliConfigAgentId): CliConfigPayload {
  if (agentId === "claude-code") {
    return {
      kind: "claude-code",
      baseUrl: "https://",
      authMode: "auth-token",
      model: "",
      haikuModel: "",
      sonnetModel: "",
      opusModel: "",
      advancedEnv: {},
    };
  }
  if (agentId === "codex-cli") {
    return {
      kind: "codex-cli",
      providerId: "custom",
      baseUrl: "https://",
      model: "",
      wireApi: "responses",
      reasoningEffort: "medium",
      authStrategy: "bearer-token",
      advancedToml: {},
    };
  }
  return {
    kind: "opencode",
    providerId: "custom",
    providerName: "Custom provider",
    npm: "@ai-sdk/openai-compatible",
    baseUrl: "https://",
    headers: {},
    models: [],
    defaultModel: "",
  };
}
