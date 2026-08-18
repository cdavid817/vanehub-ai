import type { CodeIndexService } from "./code-index-service";
import { normalizeCodeIndexConfiguration } from "./code-index-contract";
import {
  cloneCodeIndex,
  emptyCodeIndexStatus,
  readWebCodeIndexAudit,
  readWebRetrievalConfiguration,
  readWebRetrievalIndexStatus,
  recordWebCodeIndexAudit,
  requireWebCodeIndex,
  takeNextWebCodeIndexId,
  updateWebCodeIndexPhase,
  webCodeIndexes,
  writeWebCodeIndexAudit,
  writeWebRetrievalConfiguration,
} from "./web-code-index-state";
import type { CodeIndexPhase, CodeIndexWorkspace } from "../types/code-index";
import { codeIndexLanguages } from "../types/code-index";

export const webCodeIndexClient: CodeIndexService = {
  async getRetrievalConfiguration() {
    return { ...readWebRetrievalConfiguration() };
  },

  async saveRetrievalConfiguration(profileId, modelId) {
    writeWebRetrievalConfiguration({
      ...readWebRetrievalConfiguration(),
      sourceProfileId: profileId,
      embeddingModel: modelId,
    });
  },

  async saveCodeIndexAutomaticMode(mode) {
    writeWebRetrievalConfiguration({ ...readWebRetrievalConfiguration(), automaticCodeIndexMode: mode });
  },

  async getRetrievalIndexStatus() {
    return { ...readWebRetrievalIndexStatus() };
  },

  async rebuildRetrievalIndex() {
    const status = readWebRetrievalIndexStatus();
    status.pending += status.indexed + status.failed;
    status.indexed = 0;
    status.failed = 0;
    status.lastFailureCategory = null;
  },

  async listCodeIndexWorkspaces() {
    return [...webCodeIndexes.values()].map(cloneCodeIndex);
  },

  async getCodeIndexWorkspace(workspaceId) {
    return cloneCodeIndex(requireWebCodeIndex(workspaceId));
  },

  async registerCodeIndexWorkspace(root, displayName) {
    const canonicalRoot = root.trim();
    const name = displayName.trim();
    if (!canonicalRoot || !name) throw new Error("Workspace root and display name are required.");
    const existing = [...webCodeIndexes.values()].find((workspace) => workspace.canonicalRoot === canonicalRoot);
    if (existing) return cloneCodeIndex(existing);
    const workspace: CodeIndexWorkspace = {
      workspaceId: `web-code-index-${takeNextWebCodeIndexId()}`,
      canonicalRoot,
      displayName: name,
      origin: "manual",
      enabled: false,
      mode: "local",
      selectedRoots: [""],
      languages: [...codeIndexLanguages],
      exclusionPatterns: [],
      maxFileBytes: 100 * 1024,
      indexVersion: "1",
      generation: 0,
      status: emptyCodeIndexStatus("disabled"),
    };
    webCodeIndexes.set(workspace.workspaceId, workspace);
    return cloneCodeIndex(workspace);
  },

  async saveCodeIndexConfiguration(workspaceId, configuration) {
    const workspace = requireWebCodeIndex(workspaceId);
    const normalized = normalizeCodeIndexConfiguration(configuration);
    Object.assign(workspace, normalized);
    workspace.generation += 1;
    workspace.status = emptyCodeIndexStatus(normalized.enabled ? "scanning" : "disabled");
    return cloneCodeIndex(workspace);
  },

  async refreshCodeIndexWorkspace(workspaceId) {
    const workspace = requireWebCodeIndex(workspaceId);
    if (workspace.mode === "local" && workspace.status.phase === "parsing") {
      Object.assign(workspace.status, {
        totalFiles: 18,
        processedFiles: 18,
        failedFiles: 0,
        totalChunks: 54,
        processedChunks: 0,
        pendingChunks: 54,
        indexedChunks: 0,
        failedChunks: 0,
        redactionCount: 4,
        estimatedEmbeddingRequests: 0,
      });
      updateWebCodeIndexPhase(workspace, "ready");
      return structuredClone(workspace.status);
    }
    const nextPhase: Partial<Record<CodeIndexPhase, CodeIndexPhase>> = {
      scanning: "parsing",
      parsing: "awaiting_embedding_confirmation",
      embedding: "ready",
      degraded: "scanning",
      cancelling: "disabled",
    };
    updateWebCodeIndexPhase(workspace, nextPhase[workspace.status.phase] ?? workspace.status.phase);
    return structuredClone(workspace.status);
  },

  async confirmCodeIndexEmbedding(workspaceId, profileId, model, generation) {
    const workspace = requireWebCodeIndex(workspaceId);
    if (workspace.mode !== "semantic") {
      throw new Error("Local code indexes do not use embedding confirmation.");
    }
    if (!workspace.enabled || generation !== workspace.generation
      || readWebRetrievalConfiguration().sourceProfileId !== profileId
      || readWebRetrievalConfiguration().embeddingModel !== model) {
      throw new Error("Embedding confirmation is stale or does not match the active model.");
    }
    updateWebCodeIndexPhase(workspace, "embedding");
    return { profileId, model, generation };
  },

  async getCodeIndexStatus(workspaceId) {
    return structuredClone(requireWebCodeIndex(workspaceId).status);
  },

  async listCodeIndexAudit(workspaceId, limit = 50) {
    requireWebCodeIndex(workspaceId);
    const boundedLimit = Math.max(0, Math.min(100, Math.trunc(limit)));
    return structuredClone(readWebCodeIndexAudit().filter((entry) => entry.workspaceId === workspaceId).slice(0, boundedLimit));
  },

  async rebuildCodeIndexWorkspace(workspaceId) {
    const workspace = requireWebCodeIndex(workspaceId);
    workspace.generation += 1;
    workspace.status = emptyCodeIndexStatus(workspace.enabled ? "scanning" : "disabled");
    recordWebCodeIndexAudit(workspaceId, "rebuilt");
    return cloneCodeIndex(workspace);
  },

  async disableCodeIndexWorkspace(workspaceId) {
    const workspace = requireWebCodeIndex(workspaceId);
    workspace.enabled = false;
    workspace.generation += 1;
    workspace.status = emptyCodeIndexStatus("disabled");
    return cloneCodeIndex(workspace);
  },

  async deleteCodeIndexWorkspace(workspaceId) {
    requireWebCodeIndex(workspaceId);
    webCodeIndexes.delete(workspaceId);
    writeWebCodeIndexAudit(readWebCodeIndexAudit().filter((entry) => entry.workspaceId !== workspaceId));
  },
};
