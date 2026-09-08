import { getOnePieceProviderPresets } from "./onepiece-provider-presets";
import type { OnePieceProviderEndpoint, OnePieceProviderPreset } from "../types/agent";
import type { CliConfigAgentId, CliConfigPayload, CliConfigPreset } from "../types/cli-agent-config";

/**
 * Antigravity accepts no third-party endpoint, so it gets one official settings preset rather than
 * a row per provider in the shared endpoint directory.
 */
const antigravityPreset: CliConfigPreset = {
  id: "antigravity-cli-google-official",
  catalogVersion: 1,
  displayName: "Google Antigravity",
  description: "Google Antigravity · Antigravity CLI",
  category: "official",
  agentId: "antigravity-cli",
  deprecated: false,
  providerId: "google-antigravity",
  endpointType: "openai-chat-completions",
  iconKey: "google",
  payload: {
    kind: "antigravity",
    toolPermission: "request-review",
    enableTerminalSandbox: false,
    verbosity: "high",
    model: "",
    advancedSettings: {},
  },
};

const geminiPreset: CliConfigPreset = {
  id: "gemini-cli-google-official",
  catalogVersion: 1,
  displayName: "Google Gemini",
  description: "Google Gemini · Gemini CLI",
  category: "official",
  agentId: "gemini-cli",
  deprecated: false,
  providerId: "google-gemini",
  endpointType: "openai-chat-completions",
  iconKey: "google",
  payload: {
    kind: "gemini-cli",
    baseUrl: "https://generativelanguage.googleapis.com",
    model: "auto",
    authStrategy: "preserve-official",
    advancedEnv: {},
  },
};

/**
 * Qwen Code's own sign-in: no third-party endpoint, no key. Listed once so the OAuth path stays a
 * first-class choice beside the per-provider OpenAI-compatible entries.
 */
const qwenOfficialPreset: CliConfigPreset = {
  id: "qwen-code-qwen-oauth-official",
  catalogVersion: 1,
  displayName: "Qwen OAuth",
  description: "Qwen OAuth · Qwen Code",
  category: "official",
  agentId: "qwen-code",
  deprecated: false,
  providerId: "qwen-oauth",
  endpointType: "openai-chat-completions",
  iconKey: "qwen",
  payload: {
    kind: "qwen-code",
    baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
    model: "coder-model",
    authStrategy: "preserve-official",
    advancedEnv: {},
  },
};

const agentLabels: Record<CliConfigAgentId, string> = {
  "claude-code": "Claude Code",
  "codex-cli": "Codex",
  opencode: "OpenCode",
  "antigravity-cli": "Antigravity CLI",
  "gemini-cli": "Gemini CLI",
  "qwen-code": "Qwen Code",
  "iflow-cli": "iFlow",
};

function basePreset(provider: OnePieceProviderPreset, endpoint: OnePieceProviderEndpoint, agentId: CliConfigAgentId) {
  const agentLabel = agentLabels[agentId];
  return {
    id: `${agentId}-${provider.id}-${endpoint.type}`,
    catalogVersion: provider.catalogVersion,
    displayName: provider.displayName,
    description: `${provider.displayName} · ${agentLabel}`,
    category: provider.category,
    agentId,
    deprecated: false,
    providerId: provider.id,
    endpointType: endpoint.type,
    iconKey: provider.iconKey,
  } as const;
}

function buildPresets(): CliConfigPreset[] {
  return getOnePieceProviderPresets().flatMap((provider) => provider.endpoints.flatMap((endpoint): CliConfigPreset[] => {
    if (endpoint.type === "anthropic-messages") {
      return [{ ...basePreset(provider, endpoint, "claude-code"), payload: {
        kind: "claude-code", baseUrl: endpoint.baseUrl,
        authMode: provider.id === "anthropic" ? "none" : endpoint.authStrategy === "x-api-key" ? "api-key" : "auth-token",
        model: provider.defaultModelId, haikuModel: provider.defaultModelId,
        sonnetModel: provider.defaultModelId, opusModel: provider.defaultModelId, advancedEnv: {},
      }}];
    }
    const codex: CliConfigPreset = { ...basePreset(provider, endpoint, "codex-cli"), payload: {
      kind: "codex-cli", providerId: provider.id, baseUrl: endpoint.baseUrl,
      model: provider.defaultModelId, wireApi: endpoint.type === "openai-responses" ? "responses" : "chat",
      reasoningEffort: "medium", authStrategy: provider.id === "openai" ? "preserve-official" : "bearer-token", advancedToml: {},
    }};
    const opencode: CliConfigPreset = { ...basePreset(provider, endpoint, "opencode"), payload: {
      kind: "opencode", providerId: provider.id, providerName: provider.displayName,
      npm: "@ai-sdk/openai-compatible", baseUrl: endpoint.baseUrl, headers: {},
      models: provider.fallbackModels.map((id) => ({ id, name: id })), defaultModel: provider.defaultModelId,
    }};
    if (endpoint.type !== "openai-chat-completions") return [codex, opencode];
    // Qwen Code and iFlow speak chat completions only, so the responses-API endpoints of the same
    // provider are not offered to them.
    const qwen: CliConfigPreset = { ...basePreset(provider, endpoint, "qwen-code"), payload: {
      kind: "qwen-code", baseUrl: endpoint.baseUrl, model: provider.defaultModelId,
      authStrategy: "api-key", advancedEnv: {},
    }};
    const iflow: CliConfigPreset = { ...basePreset(provider, endpoint, "iflow-cli"), payload: {
      kind: "iflow-cli", baseUrl: endpoint.baseUrl, model: provider.defaultModelId, advancedSettings: {},
    }};
    return [codex, opencode, qwen, iflow];
  })).concat(antigravityPreset, geminiPreset, qwenOfficialPreset);
}

export const cliAgentProviderPresets: readonly CliConfigPreset[] = buildPresets();

export function getCliConfigPresets(agentId: CliConfigAgentId): CliConfigPreset[] {
  return cliAgentProviderPresets.filter((preset) => preset.agentId === agentId)
    .map((preset) => ({ ...preset, payload: structuredClone(preset.payload) }));
}

export function createCustomCliConfigPayload(agentId: CliConfigAgentId): CliConfigPayload {
  if (agentId === "claude-code") return { kind: "claude-code", baseUrl: "https://", authMode: "auth-token", model: "", haikuModel: "", sonnetModel: "", opusModel: "", advancedEnv: {} };
  if (agentId === "codex-cli") return { kind: "codex-cli", providerId: "custom", baseUrl: "https://", model: "", wireApi: "responses", reasoningEffort: "medium", authStrategy: "bearer-token", advancedToml: {} };
  if (agentId === "antigravity-cli") return { kind: "antigravity", toolPermission: "request-review", enableTerminalSandbox: false, verbosity: "high", model: "", advancedSettings: {} };
  if (agentId === "gemini-cli") return { kind: "gemini-cli", baseUrl: "https://generativelanguage.googleapis.com", model: "auto", authStrategy: "api-key", advancedEnv: {} };
  if (agentId === "qwen-code") return { kind: "qwen-code", baseUrl: "https://", model: "", authStrategy: "api-key", advancedEnv: {} };
  if (agentId === "iflow-cli") return { kind: "iflow-cli", baseUrl: "https://", model: "", advancedSettings: {} };
  return { kind: "opencode", providerId: "custom", providerName: "Custom provider", npm: "@ai-sdk/openai-compatible", baseUrl: "https://", headers: {}, models: [], defaultModel: "" };
}
