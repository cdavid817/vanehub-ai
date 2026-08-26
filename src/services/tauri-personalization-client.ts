import { invoke } from "@tauri-apps/api/core";
import type {
  AgentPersonalizationCapability,
  EffectivePreview,
  EffectivePreviewInput,
  PersonalizationHealth,
  PersonalizationPolicy,
  PersonalizationPolicyPatch,
  PersonalizationPolicyRef,
  WorkspaceScope,
  WorkspaceScopeInput,
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
import type { PersonalizationService } from "./personalization-service";

/**
 * Mirrors `web-personalization-client.ts` so the two runtimes' personalization surfaces stay
 * side-by-side and an operation missing from one is obvious.
 *
 * Optional arguments are sent as explicit `null` rather than omitted: an absent key and a null one
 * both deserialize to `None` today, but only the null says the caller meant "no value" rather than
 * "this build predates the field".
 */
export const tauriPersonalizationClient: PersonalizationService = {
  getPersonalizationHealth() {
    return invoke<PersonalizationHealth>("get_personalization_health");
  },

  listPersonalizationPolicies() {
    return invoke<PersonalizationPolicy[]>("list_personalization_policies");
  },

  getPersonalizationPolicy(scope: PersonalizationPolicyRef) {
    return invoke<PersonalizationPolicy | null>("get_personalization_policy", {
      scopeKind: scope.scopeKind,
      agentId: scope.agentId ?? null,
      workspaceKey: scope.workspaceKey ?? null,
    });
  },

  patchPersonalizationPolicy(patch: PersonalizationPolicyPatch) {
    return invoke<PersonalizationPolicy>("patch_personalization_policy", { input: patch });
  },

  previewEffectivePersonalization(input: EffectivePreviewInput) {
    return invoke<EffectivePreview>("preview_effective_personalization", { input });
  },

  listPersonalizationAgentCapabilities() {
    return invoke<AgentPersonalizationCapability[]>("list_personalization_agent_capabilities");
  },

  resolvePersonalizationWorkspace(input: WorkspaceScopeInput) {
    return invoke<WorkspaceScope | null>("resolve_personalization_workspace", { input });
  },

  queryPersonalizationMemories(query: MemoryQuery) {
    return invoke<MemoryPage>("query_personalization_memories", { input: query });
  },

  getPersonalizationMemory(memoryId: string) {
    return invoke<MemoryDetail | null>("get_personalization_memory", { memoryId });
  },

  createPersonalizationMemory(input: CreateMemoryInput) {
    return invoke<MemoryDetail>("create_personalization_memory", { input });
  },

  updatePersonalizationMemory(input: UpdateMemoryInput) {
    return invoke<MemoryDetail>("update_personalization_memory", { input });
  },

  deletePersonalizationMemory(memoryId: string, expectedRevision?: number) {
    return invoke<MaintenanceResult>("delete_personalization_memory", {
      memoryId,
      expectedRevision: expectedRevision ?? null,
    });
  },

  listPersonalizationCandidates(limit?: number) {
    return invoke<MemoryCandidate[]>("list_personalization_candidates", { limit: limit ?? null });
  },

  reviewPersonalizationCandidate(input: ReviewCandidateInput) {
    return invoke<ReviewOutcome>("review_personalization_candidate", { input });
  },

  previewPersonalizationReset(scope: ResetScope) {
    return invoke<ResetPreview>("preview_personalization_reset", { input: scope });
  },

  executePersonalizationReset(scope: ResetScope, confirmationToken: string, typedPhrase: string) {
    return invoke<MaintenanceResult>("execute_personalization_reset", {
      input: scope,
      confirmationToken,
      typedPhrase,
    });
  },

  reconcilePersonalizationMemories() {
    return invoke<MaintenanceResult>("reconcile_personalization_memories");
  },
};
