import { i18n } from "../i18n";
import { mockAgents } from "./mock-agent-data";
import type { ApiAgentService } from "./api-provider-service";
import { slugify } from "./web-mock-identifiers";
import { webApiAgentProviderConfigs } from "./web-api-provider-state";
import type { AgentRegistryEntry } from "../types/agent";

export const webApiAgentClient: ApiAgentService = {
  async registerApiAgent(input) {
    const displayName = input.displayName.trim();
    const provider = input.provider.trim();
    const apiKey = input.apiKey.trim();
    const modelId = input.modelId.trim();
    const runtimeKind = input.runtimeKind ?? "cloud";
    const authenticationMode = input.authenticationMode ?? "required";
    const privacyClassification = input.privacyClassification ?? "cloud";
    const timeoutMs = input.timeoutMs ?? 30_000;
    if (!displayName || !provider || !modelId
      || (authenticationMode === "required" && !apiKey)
      || (authenticationMode === "none" && Boolean(apiKey))) {
      throw new Error(i18n.t("agents.registerApiAgent.errors.incomplete"));
    }
    const baseUrl = input.baseUrl?.trim() || null;
    if (input.interfaceFormat === "openai-compatible" && !baseUrl) {
      throw new Error(i18n.t("agents.registerApiAgent.errors.baseUrlRequired"));
    }
    if (input.interfaceFormat !== "openai-compatible" && authenticationMode !== "required") {
      throw new Error("Only OpenAI-compatible endpoints can use optional authentication.");
    }
    let loopback = false;
    if (baseUrl) {
      try {
        loopback = ["localhost", "127.0.0.1", "[::1]", "::1"].includes(new URL(baseUrl).hostname);
      } catch {
        throw new Error("The API endpoint configuration is invalid or unsafe.");
      }
    }
    if (runtimeKind !== privacyClassification || (runtimeKind === "local" && !loopback)
      || (runtimeKind === "cloud" && authenticationMode !== "required")
      || timeoutMs < 100 || timeoutMs > 120_000) {
      throw new Error("The API endpoint configuration is invalid or unsafe.");
    }
    const baseId = slugify(displayName) || "api-agent";
    let candidateId = baseId;
    let suffix = 2;
    while (mockAgents.some((agent) => agent.id === candidateId)) {
      candidateId = `${baseId}-${suffix}`;
      suffix += 1;
    }
    const entry: AgentRegistryEntry = {
      id: candidateId,
      displayName,
      provider,
      launch: { kind: "api" },
      supportedInteractionModes: ["api"],
      availabilityState: "available",
      capabilityTags: ["api"],
      agentOrigin: "user",
    };
    mockAgents.push(entry);
    webApiAgentProviderConfigs.set(candidateId, {
      modelId,
      interfaceFormat: input.interfaceFormat,
      baseUrl,
      autoApproveTools: false,
    });
    return entry;
  },

  async getApiAgentProviderConfig(agentId) {
    return webApiAgentProviderConfigs.get(agentId) ?? null;
  },

  async updateApiAgent(agentId, input) {
    const agent = mockAgents.find((candidate) => candidate.id === agentId);
    const current = webApiAgentProviderConfigs.get(agentId);
    if (!agent || !current) {
      throw new Error(i18n.t("agents.updateApiAgent.errors.notFound"));
    }
    const displayName = input.displayName.trim();
    const modelId = input.modelId.trim();
    if (!displayName || !modelId) {
      throw new Error(i18n.t("agents.registerApiAgent.errors.incomplete"));
    }
    const baseUrl = input.baseUrl?.trim() || null;
    if (current.interfaceFormat === "openai-compatible" && !baseUrl) {
      throw new Error(i18n.t("agents.registerApiAgent.errors.baseUrlRequired"));
    }
    agent.displayName = displayName;
    webApiAgentProviderConfigs.set(agentId, { ...current, modelId, baseUrl });
    return agent;
  },
};
