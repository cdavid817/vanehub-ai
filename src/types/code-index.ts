export const codeIndexLanguages = [
  "javascript",
  "typescript",
  "python",
  "rust",
  "go",
  "java",
  "c",
  "cpp",
] as const;

export type CodeIndexLanguage = (typeof codeIndexLanguages)[number];

export const codeIndexModes = ["local", "semantic"] as const;

export type CodeIndexMode = (typeof codeIndexModes)[number];

export type CodeIndexWorkspaceOrigin = "manual" | "automatic";
export type CodeIndexLocalState = "disabled" | "scanning" | "parsing" | "ready" | "degraded" | "unavailable";
export type CodeIndexSemanticState = "not_applicable" | "disabled" | "pending" | "unconfigured" | "awaiting_confirmation" | "embedding" | "ready" | "degraded";

export interface CodeIndexChannelStatus {
  local: CodeIndexLocalState;
  semantic: CodeIndexSemanticState;
}

export const codeIndexAutomaticModes = ["disabled", "local", "semantic"] as const;

export type CodeIndexAutomaticMode = (typeof codeIndexAutomaticModes)[number];

export const codeIndexPhases = [
  "disabled",
  "scanning",
  "parsing",
  "awaiting_embedding_confirmation",
  "embedding",
  "ready",
  "degraded",
  "cancelling",
  "unavailable",
] as const;

export type CodeIndexPhase = (typeof codeIndexPhases)[number];

export interface CodeIndexStatus {
  phase: CodeIndexPhase;
  totalFiles: number;
  processedFiles: number;
  failedFiles: number;
  totalChunks: number;
  processedChunks: number;
  pendingChunks: number;
  indexedChunks: number;
  failedChunks: number;
  redactionCount: number;
  estimatedEmbeddingRequests: number;
  lastFailureCategory: string | null;
  updatedAt: string;
}

export interface CodeIndexConfigurationInput {
  enabled: boolean;
  mode: CodeIndexMode;
  selectedRoots: string[];
  languages: CodeIndexLanguage[];
  exclusionPatterns: string[];
  maxFileBytes: number;
}

export interface CodeIndexWorkspace extends CodeIndexConfigurationInput {
  workspaceId: string;
  canonicalRoot: string;
  displayName: string;
  origin: CodeIndexWorkspaceOrigin;
  indexVersion: string;
  generation: number;
  status: CodeIndexStatus;
}

export interface CodeEmbeddingConfirmation {
  profileId: string;
  model: string;
  generation: number;
}

export type CodeIndexAuditEvent =
  | "admitted"
  | "skipped"
  | "indexed"
  | "failed"
  | "deleted"
  | "rebuilt";

export type CodeIndexAuditReason =
  | "outside_selected_roots"
  | "sensitive_file"
  | "user_excluded"
  | "language_disabled"
  | "size_limit"
  | "binary"
  | "unreadable"
  | "parse"
  | "auth"
  | "invalid_request"
  | "rate_limit"
  | "network"
  | "stale_generation";

export interface CodeIndexAuditEntry {
  auditId: number;
  workspaceId: string;
  relativePath: string | null;
  event: CodeIndexAuditEvent;
  reason: CodeIndexAuditReason | null;
  itemCount: number;
  createdAt: string;
}
