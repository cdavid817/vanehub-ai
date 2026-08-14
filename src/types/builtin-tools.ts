export type BuiltinToolCapability =
  | "filesystem"
  | "command"
  | "browser"
  | "web"
  | "code_execution"
  | "ocr"
  | "artifact"
  | "delegation";

export type BuiltinToolMode = "read" | "write" | "execute" | "publish" | "apply";

export type BuiltinToolReadinessState = "ready" | "degraded" | "unavailable";

export interface BuiltinToolModeReadiness {
  mode: BuiltinToolMode;
  state: BuiltinToolReadinessState;
  reasonCode: string | null;
  simulated: boolean;
}

export interface BuiltinToolCapabilityReadiness {
  capability: BuiltinToolCapability;
  modes: BuiltinToolModeReadiness[];
}

export interface BuiltinToolReadiness {
  agentId: string;
  observedAt: string;
  capabilities: BuiltinToolCapabilityReadiness[];
}

export type BuiltinToolOperationStatus =
  | "queued"
  | "running"
  | "awaiting_human"
  | "cancelling"
  | "succeeded"
  | "failed"
  | "cancelled";

export interface BuiltinToolProgress {
  phase: string;
  completedUnits: number | null;
  totalUnits: number | null;
  messageCode: string | null;
}

export interface BuiltinToolOperation {
  id: string;
  agentId: string;
  sessionId: string;
  capability: BuiltinToolCapability;
  operation: string;
  status: BuiltinToolOperationStatus;
  progress: BuiltinToolProgress | null;
  artifactIds: string[];
  errorCode: string | null;
  simulated: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface ListBuiltinToolOperationsInput {
  sessionId: string;
  capability?: BuiltinToolCapability;
  limit?: number;
}

export type BuiltinToolOperationEvent =
  | { kind: "snapshot"; operation: BuiltinToolOperation }
  | { kind: "removed"; operationId: string };

export type ArtifactIntegrityState = "verified" | "warning" | "failed";

export interface ArtifactSummary {
  id: string;
  displayName: string;
  mediaType: string;
  sizeBytes: number;
  contentHash: string;
  integrity: ArtifactIntegrityState;
  createdAt: string;
  expiresAt: string | null;
  simulated: boolean;
}

export interface ArtifactDetail extends ArtifactSummary {
  producerOperationId: string;
  provenance: string[];
  publishedAt: string | null;
  publicationUrl: string | null;
  limitations: string[];
}

export interface ListArtifactsInput {
  sessionId: string;
  cursor?: string;
  limit?: number;
}

export interface ArtifactPage {
  items: ArtifactSummary[];
  nextCursor: string | null;
}

export interface ReadArtifactInput {
  artifactId: string;
  offset: number;
  length: number;
}

export interface ArtifactChunk {
  artifactId: string;
  offset: number;
  bytesBase64: string;
  nextOffset: number | null;
  contentHash: string;
}

export interface PublishArtifactInput {
  artifactId: string;
  expectedContentHash: string;
  acknowledgement: boolean;
}

export interface DownloadArtifactInput {
  artifactId: string;
  expectedContentHash: string;
}

export interface DownloadArtifactResult {
  path: string;
  contentHash: string;
}

export type DelegationMode = "analyze" | "edit";

export interface StartDelegationInput {
  agentId: string;
  sessionId: string;
  provider: "claude_code" | "codex_cli";
  mode: DelegationMode;
  prompt: string;
  artifactIds: string[];
}

export interface DelegationAttemptSummary {
  id: string;
  delegationId: string;
  provider: "claude_code" | "codex_cli";
  mode: DelegationMode;
  status: BuiltinToolOperationStatus;
  baseCommit: string;
  changeSetArtifactId: string | null;
  createdAt: string;
  completedAt: string | null;
}

export interface DelegationReport {
  attempt: DelegationAttemptSummary;
  outcome: string;
  summary: string;
  hostEvidence: string[];
  providerClaims: string[];
  warnings: string[];
}

export interface ChangeSetReview {
  artifact: ArtifactDetail;
  repositoryIdentity: string;
  baseCommit: string;
  diffHash: string;
  files: string[];
  diffText: string;
  riskClassification: string;
  applyable: boolean;
}

export interface ApplyDelegationChangesInput {
  agentId: string;
  sessionId: string;
  artifactId: string;
  expectedContentHash: string;
  expectedDiffHash: string;
  repositoryIdentity: string;
  baseCommit: string;
  acknowledgement: true;
}

export type DelegationRecoveryState = "safely_completed" | "rolled_back" | "manual_recovery";

export interface DelegationRecoveryResult {
  operationId: string;
  state: DelegationRecoveryState;
  capsuleReference: string | null;
}

export interface BrowserHandoffState {
  operationId: string;
  state: "automating" | "awaiting_human" | "human_control" | "resuming" | "closed";
  ownershipToken: string;
  updatedAt: string;
}
