import { mockAgents } from "./mock-agent-data";
import type {
  ApiAgentProviderConfig,
  EmbeddingModelOption,
  EndpointProfileMetadata,
  HybridRoutingRule,
  OnePieceProviderConfig,
  OnePieceProviderProfiles,
} from "../types/agent";

/** Mock API agents' `modelId`/`interfaceFormat`/`baseUrl` (`add-agent-lifecycle-management`) —
 * kept out of `AgentRegistryEntry` itself, mirroring the real backend, where those fields live
 * behind a separate read path (`getApiAgentProviderConfig`) rather than on the CLI/API-agnostic
 * registry view. */
export const webApiAgentProviderConfigs = new Map<string, ApiAgentProviderConfig>();
export const webEndpointProfileMetadata = new Map<string, EndpointProfileMetadata>();
export const webEmbeddingModelOptions: EmbeddingModelOption[] = [
  { id: "text-embedding-3-small", displayName: "text-embedding-3-small" },
  { id: "text-embedding-3-large", displayName: "text-embedding-3-large" },
];

// The four rebindable bindings below stay module-private. Reads and writes go through the accessors
// so no other module can re-import a binding and end up mutating a second copy of the mock world.
let webOnePieceProviderConfig: OnePieceProviderConfig = {
  provider: "VaneHub",
  modelId: null,
  interfaceFormat: null,
  baseUrl: null,
  autoApproveTools: false,
  credentialPresent: false,
};
let webOnePieceProviderProfiles: OnePieceProviderProfiles = { profiles: [], activeProfileId: null };
let nextOnePieceProviderProfileId = 1;
let webHybridRoutingRules: HybridRoutingRule[] = [];

export function readOnePieceProviderConfig(): OnePieceProviderConfig {
  return webOnePieceProviderConfig;
}

export function writeOnePieceProviderConfig(value: OnePieceProviderConfig) {
  webOnePieceProviderConfig = value;
}

export function readOnePieceProviderProfiles(): OnePieceProviderProfiles {
  return webOnePieceProviderProfiles;
}

export function writeOnePieceProviderProfiles(value: OnePieceProviderProfiles) {
  webOnePieceProviderProfiles = value;
}

export function takeNextOnePieceProviderProfileId() {
  return nextOnePieceProviderProfileId++;
}

export function readHybridRoutingRules(): HybridRoutingRule[] {
  return webHybridRoutingRules;
}

export function writeHybridRoutingRules(value: HybridRoutingRule[]) {
  webHybridRoutingRules = value;
}

/** The composition root's `deleteApiAgent` cascade needs to drop a provider config without owning
 * the map, so the behavior is exported rather than the container. */
export function deleteWebApiAgentProviderConfig(agentId: string) {
  webApiAgentProviderConfigs.delete(agentId);
}

export function applyWebOnePieceActiveProfile(profileId: string | null) {
  const active = profileId == null
    ? null
    : webOnePieceProviderProfiles.profiles.find((profile) => profile.id === profileId) ?? null;
  webOnePieceProviderProfiles = {
    activeProfileId: active?.id ?? null,
    profiles: webOnePieceProviderProfiles.profiles.map((profile) => ({
      ...profile,
      active: profile.id === active?.id,
    })),
  };
  webOnePieceProviderConfig = active ? {
    provider: active.provider,
    modelId: active.modelId,
    interfaceFormat: active.interfaceFormat,
    baseUrl: active.baseUrl,
    autoApproveTools: webOnePieceProviderConfig.autoApproveTools,
    credentialPresent: active.credentialPresent,
  } : {
    provider: "VaneHub",
    modelId: null,
    interfaceFormat: null,
    baseUrl: null,
    autoApproveTools: false,
    credentialPresent: false,
  };
  if (active) {
    webApiAgentProviderConfigs.set("onepiece", {
      modelId: active.modelId,
      interfaceFormat: active.interfaceFormat,
      baseUrl: active.baseUrl,
      autoApproveTools: webOnePieceProviderConfig.autoApproveTools,
    });
  } else {
    webApiAgentProviderConfigs.delete("onepiece");
  }
  const agent = mockAgents.find((candidate) => candidate.id === "onepiece");
  if (agent) {
    agent.provider = active?.provider ?? "VaneHub";
    agent.availabilityState = active?.credentialPresent ? "available" : active ? "needs-auth" : "unavailable";
    agent.unavailableReason = active?.credentialPresent
      ? undefined
      : active
        ? "OnePiece requires an API key."
        : "OnePiece requires provider configuration.";
  }
}
