import type {
  AgentPersonalizationCapability,
  EffectivePreview,
  EffectivePreviewInput,
  PersonalizationHealth,
  PersonalizationPolicy,
  PersonalizationPolicyPatch,
  PersonalizationPolicyRef,
} from "../types/personalization";
import type {
  CreateMemoryInput,
  MaintenanceResult,
  MemoryCandidate,
  MemoryDetail,
  MemoryPage,
  MemoryQuery,
  ResetPreview,
  ResetScope,
  ReviewCandidateInput,
  ReviewOutcome,
  UpdateMemoryInput,
} from "../types/personalization-memory";

/**
 * `add-unified-personalization-governance`: the one boundary personalization screens speak through.
 *
 * Separate from {@link AgentMemoryService}, which is the pre-governance host-level pool: that one
 * reads every memory at once and resets without a scope, and both are exactly what governance
 * replaces. It is removed once its callers move over.
 */
export interface PersonalizationService {
  getPersonalizationHealth(): Promise<PersonalizationHealth>;
  listPersonalizationPolicies(): Promise<PersonalizationPolicy[]>;
  /** Null means the layer has never been written -- distinct from a layer written to all-inherit. */
  getPersonalizationPolicy(scope: PersonalizationPolicyRef): Promise<PersonalizationPolicy | null>;
  patchPersonalizationPolicy(patch: PersonalizationPolicyPatch): Promise<PersonalizationPolicy>;
  previewEffectivePersonalization(input: EffectivePreviewInput): Promise<EffectivePreview>;
  listPersonalizationAgentCapabilities(): Promise<AgentPersonalizationCapability[]>;
  queryPersonalizationMemories(query: MemoryQuery): Promise<MemoryPage>;
  getPersonalizationMemory(memoryId: string): Promise<MemoryDetail | null>;
  createPersonalizationMemory(input: CreateMemoryInput): Promise<MemoryDetail>;
  updatePersonalizationMemory(input: UpdateMemoryInput): Promise<MemoryDetail>;
  deletePersonalizationMemory(memoryId: string, expectedRevision?: number): Promise<MaintenanceResult>;
  listPersonalizationCandidates(limit?: number): Promise<MemoryCandidate[]>;
  reviewPersonalizationCandidate(input: ReviewCandidateInput): Promise<ReviewOutcome>;
  /** Issues the token `executePersonalizationReset` requires, scoped to what it just counted. */
  previewPersonalizationReset(scope: ResetScope): Promise<ResetPreview>;
  executePersonalizationReset(
    scope: ResetScope,
    confirmationToken: string,
    typedPhrase: string,
  ): Promise<MaintenanceResult>;
  reconcilePersonalizationMemories(): Promise<MaintenanceResult>;
}
