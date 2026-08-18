import type { OnePieceProfileService } from "./api-provider-service";
import {
  applyWebOnePieceActiveProfile,
  readOnePieceProviderProfiles,
  takeNextOnePieceProviderProfileId,
  webEndpointProfileMetadata,
  writeOnePieceProviderProfiles,
} from "./web-api-provider-state";
import { resolveOnePieceProviderPreset } from "../config/onepiece-provider-presets";

export const webOnePieceProfileClient: OnePieceProfileService = {
  async saveOnePieceProviderProfile(input) {
    const name = input.name.trim();
    const modelId = input.modelId.trim();
    const preset = resolveOnePieceProviderPreset(input.providerId, input.endpointType);
    if (!preset) {
      throw new Error("OnePiece provider preset was not found.");
    }
    if (!name || !modelId) {
      throw new Error("Profile name and model are required.");
    }
    const existing = input.id
      ? readOnePieceProviderProfiles().profiles.find((profile) => profile.id === input.id)
      : undefined;
    if (existing?.sourceProviderId && (existing.sourceProviderId !== input.providerId || existing.sourceEndpointType !== input.endpointType)) {
      throw new Error("The provider of an existing OnePiece Profile cannot be changed.");
    }
    const credentialPresent = Boolean(input.apiKey?.trim()) || Boolean(existing?.credentialPresent);
    if (!credentialPresent) {
      throw new Error("API key is required for a new OnePiece provider Profile.");
    }
    const id = existing?.id ?? `onepiece-profile-${takeNextOnePieceProviderProfileId()}`;
    const active = existing?.active ?? readOnePieceProviderProfiles().profiles.length === 0;
    const profile = {
      id,
      name,
      sourceProviderId: input.providerId,
      sourceEndpointType: input.endpointType,
      sourcePresetVersion: preset.catalogVersion,
      provider: preset.provider,
      modelId,
      interfaceFormat: preset.interfaceFormat,
      baseUrl: preset.baseUrl,
      active,
      credentialPresent,
    };
    const current = readOnePieceProviderProfiles();
    writeOnePieceProviderProfiles({
      activeProfileId: current.activeProfileId,
      profiles: existing
        ? current.profiles.map((candidate) => candidate.id === id ? profile : candidate)
        : [...current.profiles, profile],
    });
    webEndpointProfileMetadata.set(id, {
      profileId: id,
      runtimeKind: "cloud",
      endpointSource: "catalog",
      authenticationMode: "required",
      timeoutMs: 30_000,
      privacyClassification: "cloud",
      textGenerationCapability: "supported",
      toolCallingCapability: "unknown",
      imageInputCapability: "unknown",
      structuredOutputCapability: "unknown",
      reasoningFieldCapability: "unknown",
      capabilityProvenance: "configured",
      contextWindowTokens: null,
      reservedOutputTokens: 0,
      contextCapacityProvenance: "unknown",
    });
    if (active) applyWebOnePieceActiveProfile(id);
    return structuredClone(readOnePieceProviderProfiles());
  },

  async saveCustomOnePieceProviderProfile(input) {
    const name = input.name.trim();
    const modelId = input.modelId.trim();
    const baseUrl = input.baseUrl.trim().replace(/\/$/, "");
    let parsed: URL;
    try {
      parsed = new URL(baseUrl);
    } catch {
      throw new Error("The custom endpoint Profile is invalid or unsafe.");
    }
    if (!name || !modelId || !["http:", "https:"].includes(parsed.protocol) || parsed.username || parsed.password) {
      throw new Error("The custom endpoint Profile is invalid or unsafe.");
    }
    const loopback = ["localhost", "127.0.0.1", "[::1]", "::1"].includes(parsed.hostname);
    if ((input.runtimeKind === "local" && !loopback) || input.privacyClassification !== input.runtimeKind) {
      throw new Error("The custom endpoint Profile is invalid or unsafe.");
    }
    const credentialPresent = Boolean(input.apiKey?.trim());
    if ((input.authenticationMode === "required" && !credentialPresent)
      || (input.authenticationMode === "none" && credentialPresent)
      || input.timeoutMs < 100 || input.timeoutMs > 120_000
      || (input.contextWindowTokens == null && input.reservedOutputTokens !== 0)
      || (input.contextWindowTokens != null && (input.contextWindowTokens < 1_024 || input.reservedOutputTokens >= input.contextWindowTokens))) {
      throw new Error("The custom endpoint Profile is invalid or unsafe.");
    }
    const existing = input.id
      ? readOnePieceProviderProfiles().profiles.find((profile) => profile.id === input.id)
      : undefined;
    if (existing?.sourceProviderId) throw new Error("A catalog Profile cannot be converted to a custom endpoint.");
    const id = existing?.id ?? `onepiece-profile-${takeNextOnePieceProviderProfileId()}`;
    const active = existing?.active ?? readOnePieceProviderProfiles().profiles.length === 0;
    const profile = {
      id,
      name,
      sourceProviderId: null,
      sourceEndpointType: null,
      sourcePresetVersion: null,
      provider: input.runtimeKind === "local" ? "Local endpoint" : "Private endpoint",
      modelId,
      interfaceFormat: "openai-compatible" as const,
      baseUrl,
      active,
      credentialPresent: input.authenticationMode === "none" ? false : credentialPresent || Boolean(existing?.credentialPresent),
    };
    const current = readOnePieceProviderProfiles();
    writeOnePieceProviderProfiles({
      activeProfileId: active ? id : current.activeProfileId,
      profiles: existing
        ? current.profiles.map((candidate) => candidate.id === id ? profile : candidate)
        : [...current.profiles, profile],
    });
    webEndpointProfileMetadata.set(id, {
      profileId: id,
      runtimeKind: input.runtimeKind,
      endpointSource: "configured",
      authenticationMode: input.authenticationMode,
      timeoutMs: input.timeoutMs,
      privacyClassification: input.privacyClassification,
      textGenerationCapability: "supported",
      toolCallingCapability: input.toolCallingCapability,
      imageInputCapability: input.imageInputCapability,
      structuredOutputCapability: input.structuredOutputCapability,
      reasoningFieldCapability: input.reasoningFieldCapability,
      capabilityProvenance: "configured",
      contextWindowTokens: input.contextWindowTokens,
      reservedOutputTokens: input.reservedOutputTokens,
      contextCapacityProvenance: input.contextWindowTokens == null ? "unknown" : "configured-estimate",
    });
    if (active) applyWebOnePieceActiveProfile(id);
    return structuredClone(readOnePieceProviderProfiles());
  },
};
