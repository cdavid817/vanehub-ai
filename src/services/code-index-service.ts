import type { RetrievalConfiguration, RetrievalIndexStatus } from "../types/agent";
import type {
  CodeEmbeddingConfirmation,
  CodeIndexAuditEntry,
  CodeIndexAutomaticMode,
  CodeIndexConfigurationInput,
  CodeIndexStatus,
  CodeIndexWorkspace,
} from "../types/code-index";

export interface CodeIndexService {
  // Configuration, index status and rebuild are all global: retrieval applies to every agent,
  // so status aggregates across every agent and `scope_folder`, and rebuild requeues all of them.
  getRetrievalConfiguration(): Promise<RetrievalConfiguration>;
  saveRetrievalConfiguration(profileId: string, modelId: string): Promise<void>;
  saveCodeIndexAutomaticMode(mode: CodeIndexAutomaticMode): Promise<void>;
  getRetrievalIndexStatus(): Promise<RetrievalIndexStatus>;
  rebuildRetrievalIndex(): Promise<void>;
  listCodeIndexWorkspaces(): Promise<CodeIndexWorkspace[]>;
  getCodeIndexWorkspace(workspaceId: string): Promise<CodeIndexWorkspace>;
  registerCodeIndexWorkspace(root: string, displayName: string): Promise<CodeIndexWorkspace>;
  saveCodeIndexConfiguration(
    workspaceId: string,
    configuration: CodeIndexConfigurationInput,
  ): Promise<CodeIndexWorkspace>;
  refreshCodeIndexWorkspace(workspaceId: string): Promise<CodeIndexStatus>;
  confirmCodeIndexEmbedding(
    workspaceId: string,
    profileId: string,
    model: string,
    generation: number,
  ): Promise<CodeEmbeddingConfirmation>;
  getCodeIndexStatus(workspaceId: string): Promise<CodeIndexStatus>;
  listCodeIndexAudit(workspaceId: string, limit?: number): Promise<CodeIndexAuditEntry[]>;
  rebuildCodeIndexWorkspace(workspaceId: string): Promise<CodeIndexWorkspace>;
  disableCodeIndexWorkspace(workspaceId: string): Promise<CodeIndexWorkspace>;
  deleteCodeIndexWorkspace(workspaceId: string): Promise<void>;
}
