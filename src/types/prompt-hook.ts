import type { ManagedCliAgentId } from "./agent";

export type PromptHookCategory = "bootstrap" | "callback" | "dynamic" | "law" | "navigation" | "routing" | "static";
export type PromptHookStage = "session-init" | "per-turn";
export type PromptHookSource = "builtin" | "user";
export type PromptHookSafetyTier = "readonly" | "limited-edit" | "editable";
export type PromptHookTransparencyTier = "visible-by-default" | "opt-in-view" | "debug-only";
export type PromptHookGovernanceTier = "immutable" | "human-gated" | "auto-evolve";
export type PromptHookTraceStatus = "fired" | "skipped" | "disabled" | "failed";

export interface PromptHookGovernance {
  safetyTier: PromptHookSafetyTier;
  transparencyTier: PromptHookTransparencyTier;
  governanceTier: PromptHookGovernanceTier;
}

export interface PromptHook {
  id: string;
  name: string;
  description: string;
  category: PromptHookCategory;
  stage: PromptHookStage;
  order: number;
  version: number;
  source: PromptHookSource;
  enabled: boolean;
  disableable: boolean;
  cliBindings: ManagedCliAgentId[];
  governance: PromptHookGovernance;
  templateBody?: string;
  createdAt: string;
  updatedAt: string;
}

export interface PromptHookListResult {
  hooks: PromptHook[];
  stats: {
    total: number;
    enabled: number;
    builtin: number;
    user: number;
  };
}

export interface PromptHookMutationInput {
  id: string;
  name: string;
  description: string;
  category: PromptHookCategory;
  stage: PromptHookStage;
  order: number;
  templateBody: string;
  enabled: boolean;
  cliBindings: ManagedCliAgentId[];
  governance: PromptHookGovernance;
}

export interface PromptHookUpdateInput extends PromptHookMutationInput {
  version: number;
}

export interface PromptHookPreviewInput {
  hookId: string;
  agentId: ManagedCliAgentId;
  sampleInput?: string;
}

export interface PromptAssemblyPreviewInput {
  agentId: ManagedCliAgentId;
  sampleInput: string;
}

export interface PromptHookTraceSummary {
  id: string;
  hookId: string;
  category: PromptHookCategory;
  stage: PromptHookStage;
  status: PromptHookTraceStatus;
  version?: number;
  contentHash?: string;
  tokenEstimate?: number;
  reason?: string;
  agentId?: ManagedCliAgentId;
  sessionId?: string;
  createdAt: string;
}

export interface PromptHookPreview {
  hookId?: string;
  agentId: ManagedCliAgentId;
  renderedContent: string;
  trace: PromptHookTraceSummary[];
}

