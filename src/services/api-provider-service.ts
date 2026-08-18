import type {
  AgentRegistryEntry,
  ApiAgentProviderConfig,
  DiscoverOnePieceProviderModelsInput,
  EmbeddingModelOption,
  EndpointProfileMetadata,
  HybridRoutePreview,
  HybridRoutePreviewInput,
  HybridRoutingRule,
  LocalModelDiscoveryResult,
  OnePieceProviderConfig,
  OnePieceProviderModelDiscoveryResult,
  OnePieceProviderPreset,
  OnePieceProviderProfiles,
  RegisterApiAgentInput,
  SaveCustomOnePieceProviderProfileInput,
  SaveOnePieceProviderConfigInput,
  SaveOnePieceProviderProfileInput,
  UpdateApiAgentInput,
  ValidateOnePieceProviderCredentialInput,
} from "../types/agent";
import type { ProviderCredentialValidationResult } from "../types/provider-credential-validation";

// `deleteApiAgent` is deliberately absent: it cascades across sessions, memories, Loop definitions
// and skill bindings, so it stays declared on `AgentService` and implemented in the composition root.
export interface ApiAgentService {
  registerApiAgent(input: RegisterApiAgentInput): Promise<AgentRegistryEntry>;
  getApiAgentProviderConfig(agentId: string): Promise<ApiAgentProviderConfig | null>;
  updateApiAgent(agentId: string, input: UpdateApiAgentInput): Promise<AgentRegistryEntry>;
}

export interface OnePieceProviderService {
  getOnePieceProviderConfig(): Promise<OnePieceProviderConfig>;
  saveOnePieceProviderConfig(input: SaveOnePieceProviderConfigInput): Promise<OnePieceProviderConfig>;
  resetOnePieceProviderConfig(): Promise<OnePieceProviderConfig>;
  listOnePieceProviderProfiles(): Promise<OnePieceProviderProfiles>;
  listOnePieceProviderPresets(): Promise<OnePieceProviderPreset[]>;
  discoverOnePieceProviderModels(input: DiscoverOnePieceProviderModelsInput): Promise<OnePieceProviderModelDiscoveryResult>;
  validateOnePieceProviderCredential(input: ValidateOnePieceProviderCredentialInput): Promise<ProviderCredentialValidationResult>;
  getEndpointProfileMetadata(profileId: string): Promise<EndpointProfileMetadata | null>;
  activateOnePieceProviderProfile(profileId: string): Promise<OnePieceProviderProfiles>;
  deleteOnePieceProviderProfile(profileId: string): Promise<OnePieceProviderProfiles>;
}

export interface OnePieceProfileService {
  saveOnePieceProviderProfile(input: SaveOnePieceProviderProfileInput): Promise<OnePieceProviderProfiles>;
  saveCustomOnePieceProviderProfile(input: SaveCustomOnePieceProviderProfileInput): Promise<OnePieceProviderProfiles>;
}

export interface HybridRoutingService {
  discoverLocalModelEndpoints(): Promise<LocalModelDiscoveryResult>;
  verifyLocalModelEndpoint(baseUrl: string, timeoutMs: number): Promise<LocalModelDiscoveryResult>;
  listHybridRoutingRules(): Promise<HybridRoutingRule[]>;
  replaceHybridRoutingRules(rules: HybridRoutingRule[]): Promise<HybridRoutingRule[]>;
  previewHybridRoute(input: HybridRoutePreviewInput): Promise<HybridRoutePreview>;
  listEmbeddingModels(profileId: string, transientCredential?: string): Promise<EmbeddingModelOption[]>;
}
