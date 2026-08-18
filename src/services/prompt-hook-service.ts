import type {
  PromptAssemblyPreviewInput,
  PromptHook,
  PromptHookDraft,
  PromptHookListResult,
  PromptHookMutationInput,
  PromptHookPreview,
  PromptHookPreviewInput,
  PromptHookTraceSummary,
  PromptHookUpdateInput,
  PromptHookVariableDefinition,
  PromptHookVersion,
  PromptHookVersionHistory,
  PublishPromptHookInput,
  RollbackPromptHookInput,
  SavePromptHookDraftInput,
} from "../types/prompt-hook";

export interface PromptHookService {
  listPromptHooks(): Promise<PromptHookListResult>;
  createPromptHook(input: PromptHookMutationInput): Promise<PromptHook>;
  updatePromptHook(hookId: string, input: PromptHookUpdateInput): Promise<PromptHook>;
  deletePromptHook(hookId: string): Promise<void>;
  setPromptHookEnabled(hookId: string, enabled: boolean): Promise<PromptHook>;
  setPromptHookCliBindings(hookId: string, agentIds: string[]): Promise<PromptHook>;
  previewPromptHook(input: PromptHookPreviewInput): Promise<PromptHookPreview>;
  previewPromptAssembly(input: PromptAssemblyPreviewInput): Promise<PromptHookPreview>;
  listPromptHookTraces(limit?: number): Promise<PromptHookTraceSummary[]>;
  listPromptHookVariables(): Promise<PromptHookVariableDefinition[]>;
  savePromptHookDraft(input: SavePromptHookDraftInput): Promise<PromptHookDraft>;
  publishPromptHook(input: PublishPromptHookInput): Promise<PromptHookVersion>;
  getPromptHookVersionHistory(hookId: string): Promise<PromptHookVersionHistory>;
  rollbackPromptHook(input: RollbackPromptHookInput): Promise<PromptHookVersion>;
}
