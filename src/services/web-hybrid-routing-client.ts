import type { HybridRoutingService } from "./api-provider-service";
import {
  readHybridRoutingRules,
  readOnePieceProviderProfiles,
  webEmbeddingModelOptions,
  webEndpointProfileMetadata,
  writeHybridRoutingRules,
} from "./web-api-provider-state";
import type { LocalModelDiscoveryResult } from "../types/agent";

export const webHybridRoutingClient: HybridRoutingService = {
  async discoverLocalModelEndpoints(): Promise<LocalModelDiscoveryResult> {
    return {
      operationId: "web-local-discovery-simulated",
      candidates: [{
        service: "ollama",
        baseUrl: "http://127.0.0.1:11434",
        models: ["web-simulated-local-model"],
        metadataProvenance: "verified",
        latencyBucket: "simulated",
      }],
    };
  },

  async verifyLocalModelEndpoint(baseUrl, timeoutMs) {
    if (!baseUrl.trim() || timeoutMs < 100 || timeoutMs > 120_000) throw new Error("Invalid endpoint verification request.");
    return {
      operationId: "web-local-verification-simulated",
      candidates: [{
        service: "openai-compatible",
        baseUrl: baseUrl.trim().replace(/\/$/, ""),
        models: ["web-simulated-local-model"],
        metadataProvenance: "verified" as const,
        latencyBucket: "simulated",
      }],
    };
  },

  async listHybridRoutingRules() {
    return structuredClone(readHybridRoutingRules());
  },

  async replaceHybridRoutingRules(rules) {
    if (rules.some((rule, index) => rule.orderIndex !== index
      || !readOnePieceProviderProfiles().profiles.some((profile) => profile.id === rule.preferredProfileId))) {
      throw new Error("Hybrid Routing rules are invalid or reference missing Profiles.");
    }
    writeHybridRoutingRules(structuredClone(rules));
    return structuredClone(readHybridRoutingRules());
  },

  async previewHybridRoute(input) {
    const rule = input.hybridEnabled
      ? readHybridRoutingRules().find((candidate) => candidate.enabled && candidate.taskClass === input.taskClass)
      : undefined;
    const ids = rule ? [rule.preferredProfileId, rule.fallbackProfileId] : [input.activeProfileId];
    const selected = ids
      .filter((id): id is string => Boolean(id))
      .map((id) => ({ profile: readOnePieceProviderProfiles().profiles.find((candidate) => candidate.id === id), metadata: webEndpointProfileMetadata.get(id) }))
      .find(({ profile, metadata }) => profile && metadata
        && (input.dataPolicy !== "local-only" || metadata.runtimeKind === "local")
        && (!input.requiresTools || metadata.toolCallingCapability === "supported")
        && (!input.requiresImageInput || metadata.imageInputCapability === "supported")
        && (!input.requiresStructuredOutput || metadata.structuredOutputCapability === "supported")
        && (!input.requestsReasoningField || metadata.reasoningFieldCapability === "supported"));
    if (!selected?.profile) {
      return { profileId: null, ruleId: rule?.id ?? null, reason: input.dataPolicy === "local-only" ? "waiting-local-only" : "no-usable-profile", waitingForUserChoice: input.dataPolicy === "local-only" };
    }
    return { profileId: selected.profile.id, ruleId: rule?.id ?? null, reason: rule ? selected.profile.id === rule.preferredProfileId ? "rule-preferred" : "rule-fallback-unavailable" : input.hybridEnabled ? "no-matching-rule" : "hybrid-disabled", waitingForUserChoice: false };
  },

  async listEmbeddingModels(profileId, transientCredential) {
    const profile = readOnePieceProviderProfiles().profiles.find((candidate) => candidate.id === profileId);
    if (!profile) throw new Error("OnePiece provider profile was not found.");
    if (profile.interfaceFormat !== "openai-compatible") {
      throw new Error("Only openai-compatible OnePiece profiles support embedding model discovery.");
    }
    if (!transientCredential?.trim() && !profile.credentialPresent) {
      throw new Error("API key is required to list embedding models for this OnePiece provider.");
    }
    return webEmbeddingModelOptions.map((option) => ({ ...option }));
  },
};
