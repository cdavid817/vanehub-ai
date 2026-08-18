import { i18n } from "../i18n";
import { mockAgents } from "./mock-agent-data";
import type { OnePieceProviderService } from "./api-provider-service";
import {
  applyWebOnePieceActiveProfile,
  readHybridRoutingRules,
  readOnePieceProviderConfig,
  readOnePieceProviderProfiles,
  webApiAgentProviderConfigs,
  webEndpointProfileMetadata,
  writeHybridRoutingRules,
  writeOnePieceProviderConfig,
  writeOnePieceProviderProfiles,
} from "./web-api-provider-state";
import { getOnePieceProviderPresets, resolveOnePieceProviderPreset } from "../config/onepiece-provider-presets";

export const webOnePieceProviderClient: OnePieceProviderService = {
  async getOnePieceProviderConfig() {
    return { ...readOnePieceProviderConfig() };
  },

  async saveOnePieceProviderConfig(input) {
    const provider = input.provider.trim();
    const modelId = input.modelId.trim();
    const baseUrl = input.baseUrl?.trim() || null;
    if (!provider || !modelId) throw new Error("Provider and model are required.");
    if (input.interfaceFormat === "openai-compatible" && !baseUrl) {
      throw new Error(i18n.t("agents.registerApiAgent.errors.baseUrlRequired"));
    }
    const hasReplacement = Boolean(input.apiKey?.trim());
    if (input.apiKey != null && !hasReplacement) throw new Error("API key cannot be empty.");
    if (!hasReplacement && !readOnePieceProviderConfig().credentialPresent) {
      throw new Error("API key is required for the first OnePiece configuration.");
    }
    writeOnePieceProviderConfig({
      provider,
      modelId,
      interfaceFormat: input.interfaceFormat,
      baseUrl: input.interfaceFormat === "anthropic" ? null : baseUrl,
      autoApproveTools: readOnePieceProviderConfig().autoApproveTools,
      credentialPresent: hasReplacement || readOnePieceProviderConfig().credentialPresent,
    });
    webApiAgentProviderConfigs.set("onepiece", {
      modelId,
      interfaceFormat: input.interfaceFormat,
      baseUrl: readOnePieceProviderConfig().baseUrl,
      autoApproveTools: readOnePieceProviderConfig().autoApproveTools,
    });
    const agent = mockAgents.find((candidate) => candidate.id === "onepiece");
    if (agent) {
      agent.provider = provider;
      agent.availabilityState = "available";
      agent.unavailableReason = undefined;
    }
    return { ...readOnePieceProviderConfig() };
  },

  async resetOnePieceProviderConfig() {
    writeOnePieceProviderProfiles({ profiles: [], activeProfileId: null });
    webEndpointProfileMetadata.clear();
    writeHybridRoutingRules([]);
    writeOnePieceProviderConfig({
      provider: "VaneHub",
      modelId: null,
      interfaceFormat: null,
      baseUrl: null,
      autoApproveTools: false,
      credentialPresent: false,
    });
    webApiAgentProviderConfigs.delete("onepiece");
    const agent = mockAgents.find((candidate) => candidate.id === "onepiece");
    if (agent) {
      agent.provider = "VaneHub";
      agent.availabilityState = "unavailable";
      agent.unavailableReason = "OnePiece requires provider configuration.";
    }
    return { ...readOnePieceProviderConfig() };
  },

  async listOnePieceProviderProfiles() {
    return structuredClone(readOnePieceProviderProfiles());
  },

  async listOnePieceProviderPresets() {
    return getOnePieceProviderPresets();
  },

  async discoverOnePieceProviderModels(input) {
    const preset = resolveOnePieceProviderPreset(input.providerId, input.endpointType);
    if (!preset) throw new Error("OnePiece provider preset was not found.");
    const profile = input.profileId
      ? readOnePieceProviderProfiles().profiles.find((candidate) => candidate.id === input.profileId)
      : undefined;
    if (input.profileId && !profile) throw new Error("OnePiece provider profile was not found.");
    if (profile && (profile.sourceProviderId !== input.providerId || profile.sourceEndpointType !== input.endpointType)) {
      throw new Error("The OnePiece profile does not belong to the selected provider.");
    }
    if (!input.apiKey?.trim() && !profile?.credentialPresent) {
      throw new Error("API key is required to fetch models for this OnePiece provider.");
    }
    const ids = [profile?.modelId, ...preset.fallbackModels]
      .filter((value): value is string => Boolean(value));
    return {
      providerId: input.providerId,
      endpointType: input.endpointType,
      models: [...new Set(ids)].map((id) => ({
        id,
        displayName: id,
        source: id === profile?.modelId ? "profile" as const : "catalog" as const,
      })),
      source: "catalog" as const,
      warning: null,
    };
  },

  async validateOnePieceProviderCredential(input) {
    const preset = resolveOnePieceProviderPreset(input.providerId, input.endpointType);
    if (!preset) throw new Error("OnePiece provider preset was not found.");
    const profile = input.profileId
      ? readOnePieceProviderProfiles().profiles.find((candidate) => candidate.id === input.profileId)
      : undefined;
    if (input.profileId && !profile) throw new Error("OnePiece provider profile was not found.");
    if (profile && (profile.sourceProviderId !== input.providerId || profile.sourceEndpointType !== input.endpointType)) {
      throw new Error("The OnePiece profile does not belong to the selected provider.");
    }
    if (!input.modelId.trim()) throw new Error("A model is required to verify the API key.");
    const credential = input.apiKey?.trim();
    if (!credential && !profile?.credentialPresent) throw new Error("API key is required to verify this provider.");
    const status = credential === "web-invalid"
      ? "invalid-credential" as const
      : credential === "web-rate-limited"
        ? "rate-limited" as const
        : credential === "web-unavailable"
          ? "provider-unavailable" as const
          : "valid" as const;
    return { status, latencyMs: 12, httpStatus: status === "valid" ? 200 : status === "invalid-credential" ? 401 : status === "rate-limited" ? 429 : null };
  },

  async getEndpointProfileMetadata(profileId) {
    return structuredClone(webEndpointProfileMetadata.get(profileId) ?? null);
  },

  async activateOnePieceProviderProfile(profileId) {
    const profile = readOnePieceProviderProfiles().profiles.find((candidate) => candidate.id === profileId);
    if (!profile) throw new Error("OnePiece provider profile was not found.");
    const authenticationMode = webEndpointProfileMetadata.get(profileId)?.authenticationMode ?? "required";
    if (authenticationMode === "required" && !profile.credentialPresent) {
      throw new Error("The selected OnePiece provider Profile has no API key.");
    }
    applyWebOnePieceActiveProfile(profileId);
    return structuredClone(readOnePieceProviderProfiles());
  },

  async deleteOnePieceProviderProfile(profileId) {
    const profiles = readOnePieceProviderProfiles();
    const profile = profiles.profiles.find((candidate) => candidate.id === profileId);
    if (!profile) throw new Error("OnePiece provider profile was not found.");
    writeOnePieceProviderProfiles({
      activeProfileId: profile.active ? null : profiles.activeProfileId,
      profiles: profiles.profiles.filter((candidate) => candidate.id !== profileId),
    });
    webEndpointProfileMetadata.delete(profileId);
    writeHybridRoutingRules(readHybridRoutingRules().filter((rule) =>
      rule.preferredProfileId !== profileId && rule.fallbackProfileId !== profileId));
    if (profile.active) applyWebOnePieceActiveProfile(null);
    return structuredClone(readOnePieceProviderProfiles());
  },
};
