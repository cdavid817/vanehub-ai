import type {
  AgentPersonalizationCapability,
  EffectivePreview,
  EffectivePreviewInput,
  PersonalizationHealth,
  PersonalizationPolicyPatch,
  PersonalizationPolicyRef,
  WorkspaceScopeInput,
} from "../types/personalization";
import type {
  CreateMemoryInput,
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
import {
  listWebCandidates,
  listWebMemories,
  listWebPolicies,
  nextWebMemoryId,
  putWebMemory,
  readWebCandidate,
  readWebMemory,
  readWebPolicy,
  removeWebCandidate,
  removeWebMemory,
  readWebLastReconciledAt,
  webPendingCandidateCount,
  writeWebLastReconciledAt,
  writeWebPolicy,
} from "./web-personalization-state";
import {
  MOCK_AGENT_CAPABILITIES,
  applyPatch,
  conflict,
  cursorIndex,
  maintenance,
  matchesQuery,
  notFound,
  previewFor,
  renderCursor,
  resolveWorkspaceScope,
  requireScopeKey,
  resetMatches,
  resetToken,
  summarize,
  validation,
} from "./web-personalization-rules";

/**
 * Mirrors `tauri-personalization-client.ts`, and rejects with the same message strings the native
 * command layer sends -- that is the whole contract, because `CommandError` serializes to its
 * message and nothing else crosses the wire.
 *
 * Conflicts, refused resets and unknown enum values are all enforced here rather than waved
 * through. A screen developed only against a permissive mock has never run its conflict branch,
 * and that branch is exactly the one that fires first on a real desktop.
 */
export const webPersonalizationClient: PersonalizationService = {
  async getPersonalizationHealth(): Promise<PersonalizationHealth> {
    return {
      state: "ready",
      memoryAvailable: true,
      pendingCandidates: webPendingCandidateCount(),
      lastReconciledAt: readWebLastReconciledAt(),
      repairRequired: false,
    };
  },

  async listPersonalizationPolicies() {
    return listWebPolicies();
  },

  async getPersonalizationPolicy(scope: PersonalizationPolicyRef) {
    requireScopeKey(scope);
    return readWebPolicy(scope);
  },

  async patchPersonalizationPolicy(patch: PersonalizationPolicyPatch) {
    requireScopeKey(patch);
    const current = readWebPolicy(patch);
    const storedRevision = current?.revision ?? 0;
    if (patch.expectedRevision !== undefined && patch.expectedRevision !== storedRevision) {
      throw conflict(patch.expectedRevision, storedRevision);
    }
    return writeWebPolicy(patch, (existing) => applyPatch(existing, patch));
  },

  async previewEffectivePersonalization(input: EffectivePreviewInput): Promise<EffectivePreview> {
    if (input.sessionMode === "project-only" && !input.workspaceKey) {
      throw validation("This operation needs a workspace to be scoped to.");
    }
    return previewFor(input, readWebPolicy({ scopeKind: "global" }), listWebMemories());
  },

  async listPersonalizationAgentCapabilities(): Promise<AgentPersonalizationCapability[]> {
    return MOCK_AGENT_CAPABILITIES;
  },

  async resolvePersonalizationWorkspace(input: WorkspaceScopeInput) {
    return resolveWorkspaceScope(input);
  },

  async queryPersonalizationMemories(query: MemoryQuery): Promise<MemoryPage> {
    if (query.scopeKind === "workspace" && !query.workspaceKey) {
      throw validation("unsupported scope filter: workspace");
    }
    const matched = listWebMemories().filter((memory) => matchesQuery(memory, query));
    const limit = Math.min(Math.max(query.limit ?? 50, 1), 200);
    const start = query.cursor ? cursorIndex(matched, query.cursor) : 0;
    const page = matched.slice(start, start + limit);
    const next = matched[start + limit];
    return {
      items: page.map(summarize),
      nextCursor: next ? renderCursor(next) : null,
      totalMatched: matched.length,
    };
  },

  async getPersonalizationMemory(memoryId: string) {
    return readWebMemory(memoryId);
  },

  async createPersonalizationMemory(input: CreateMemoryInput): Promise<MemoryDetail> {
    if (!input.content.trim()) throw validation("memory content must not be empty");
    if (input.scopeKind === "workspace" && !input.workspaceKey) {
      throw validation("This operation needs a workspace to be scoped to.");
    }
    const now = new Date().toISOString();
    const created: MemoryDetail = {
      id: nextWebMemoryId(),
      name: input.name,
      description: input.description,
      memoryType: input.memoryType,
      content: input.content,
      scopeKind: input.scopeKind,
      workspaceKey: input.workspaceKey ?? null,
      audienceAgentIds: input.audienceAgentIds ?? null,
      status: "active",
      source: "explicit_user",
      sensitivity: "normal",
      revision: 1,
      sourceAgentId: null,
      // A memory the user wrote in Settings was not recorded in any conversation, so there is no
      // session to offer. The mock says so rather than inventing one, which is what keeps the
      // panel's link absent here and present for an extracted memory.
      sourceSessionId: null,
      createdAt: now,
      updatedAt: now,
    };
    putWebMemory(created);
    return created;
  },

  async updatePersonalizationMemory(input: UpdateMemoryInput): Promise<MemoryDetail> {
    const current = readWebMemory(input.id);
    if (!current) throw notFound();
    if (input.expectedRevision !== current.revision) {
      throw conflict(input.expectedRevision, current.revision);
    }
    if (input.content !== undefined && !input.content.trim()) {
      throw validation("memory content must not be empty");
    }
    const updated: MemoryDetail = {
      ...current,
      name: input.name ?? current.name,
      description: input.description ?? current.description,
      memoryType: input.memoryType ?? current.memoryType,
      content: input.content ?? current.content,
      status: input.status ?? current.status,
      sensitivity: input.sensitivity ?? current.sensitivity,
      revision: current.revision + 1,
      updatedAt: new Date().toISOString(),
    };
    putWebMemory(updated);
    return updated;
  },

  async deletePersonalizationMemory(memoryId: string, expectedRevision?: number) {
    const current = readWebMemory(memoryId);
    if (!current) throw notFound();
    if (expectedRevision !== undefined && expectedRevision !== current.revision) {
      throw conflict(expectedRevision, current.revision);
    }
    removeWebMemory(memoryId);
    return maintenance({ matched: 1, deletedFiles: 1, removedProjectionRows: 1, revokedRetrievalEntries: 1 });
  },

  async listPersonalizationCandidates(limit?: number) {
    return listWebCandidates().slice(0, Math.min(Math.max(limit ?? 50, 1), 200));
  },

  async reviewPersonalizationCandidate(input: ReviewCandidateInput): Promise<ReviewOutcome> {
    const candidate = readWebCandidate(input.candidateId);
    if (!candidate) throw notFound();
    if (input.action === "merge-into" && !input.mergeTargetId) {
      throw validation("merge needs a target");
    }
    if (input.action === "reject") {
      removeWebCandidate(candidate.id);
      return { candidateId: candidate.id, status: "rejected", resultingMemoryId: null, retainedContent: false };
    }
    const targetId = input.mergeTargetId ?? candidate.targetId;
    if (targetId) {
      const target = readWebMemory(targetId);
      if (!target) throw notFound();
      const expected = input.mergeExpectedRevision ?? candidate.expectedTargetRevision;
      if (expected !== null && expected !== undefined && expected !== target.revision) {
        throw conflict(expected, target.revision);
      }
      putWebMemory({
        ...target,
        content: input.content ?? candidate.content ?? target.content,
        status: input.action === "mark-sensitive-and-archive" ? "archived" : target.status,
        sensitivity: input.action === "mark-sensitive-and-archive" ? "sensitive" : target.sensitivity,
        revision: target.revision + 1,
        updatedAt: new Date().toISOString(),
      });
      removeWebCandidate(candidate.id);
      return { candidateId: candidate.id, status: "approved", resultingMemoryId: target.id, retainedContent: true };
    }
    const created = await this.createPersonalizationMemory({
      name: input.name ?? candidate.name ?? "unnamed",
      description: input.description ?? candidate.description ?? "",
      memoryType: input.memoryType ?? "user",
      content: input.content ?? candidate.content ?? "",
      scopeKind: "global",
    });
    removeWebCandidate(candidate.id);
    return { candidateId: candidate.id, status: "approved", resultingMemoryId: created.id, retainedContent: true };
  },

  async previewPersonalizationReset(scope: ResetScope): Promise<ResetPreview> {
    const matched = resetMatches(scope);
    return {
      confirmationToken: resetToken(scope),
      matched: matched.length,
      global: matched.filter((memory) => memory.scopeKind === "global").length,
      workspace: matched.filter((memory) => memory.scopeKind === "workspace").length,
      candidates: webPendingCandidateCount(),
      malformed: 0,
    };
  },

  async executePersonalizationReset(scope: ResetScope, confirmationToken: string, typedPhrase: string) {
    if (typedPhrase !== "DELETE") throw validation("personalization-reset-refused: phrase-mismatch");
    if (confirmationToken !== resetToken(scope)) {
      throw validation("personalization-reset-refused: token-scope-mismatch");
    }
    const matched = resetMatches(scope);
    matched.forEach((memory) => removeWebMemory(memory.id));
    listWebCandidates().forEach((candidate) => removeWebCandidate(candidate.id));
    return maintenance({
      matched: matched.length,
      deletedFiles: matched.length,
      removedProjectionRows: matched.length,
      revokedRetrievalEntries: matched.length,
    });
  },

  async reconcilePersonalizationMemories() {
    // Stamped here for the same reason the desktop stamps it: a maintenance screen that
    // could not say when a rebuild last ran would leave the user re-running it blindly.
    writeWebLastReconciledAt(new Date().toISOString());
    return maintenance({ matched: listWebMemories().length });
  },
};
