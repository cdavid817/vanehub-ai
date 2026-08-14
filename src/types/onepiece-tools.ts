export type OnePieceToolCapability =
  | "artifact"
  | "browser"
  | "web"
  | "code_execution"
  | "ocr"
  | "claude_code_delegation"
  | "codex_cli_delegation";

export type OnePieceToolReadinessStatus = "ready" | "unavailable" | "desktop_required" | "simulated";

export type OnePieceToolReadinessReason =
  | "disabled"
  | "missing_dependency"
  | "version_mismatch"
  | "unhealthy_dependency"
  | "isolation_unavailable"
  | "backend_unavailable"
  | "policy_unavailable"
  | "desktop_runtime_required";

export interface OnePieceToolReadiness {
  contractVersion: 1;
  capability: OnePieceToolCapability;
  status: OnePieceToolReadinessStatus;
  reasonCode: OnePieceToolReadinessReason | null;
  supportedModes: string[];
  checkedAt: string;
}

export type OnePieceToolOperationStatus =
  | "queued"
  | "awaiting_approval"
  | "running"
  | "awaiting_human"
  | "succeeded"
  | "failed"
  | "cancelled";

export interface OnePieceToolOperation {
  contractVersion: 1;
  id: string;
  sessionId: string;
  generationId: string;
  toolName: string;
  status: OnePieceToolOperationStatus;
  progressSequence: number;
  progressMessage: string | null;
  resultArtifactIds: string[];
  errorCode: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface OnePieceToolApproval {
  contractVersion: 1;
  requestId: string;
  operationId: string;
  action: string;
  canonicalResource: string;
  inputHash: string;
  onceOnly: boolean;
  expiresAt: string | null;
}

export interface ArtifactSummary {
  contractVersion: 1;
  id: string;
  contentHash: string;
  mediaType: string;
  sizeBytes: number;
  displayName: string;
  sourceOperationId: string | null;
  sourceArtifactIds: string[];
  createdAt: string;
  expiresAt: string | null;
}

export interface ArtifactDetail extends ArtifactSummary {
  integrityStatus: "verified" | "missing" | "mismatch";
  previewKind: "text" | "image" | "pdf" | "binary" | "unavailable";
  preview: string | null;
  previewTruncated: boolean;
  publicationRef: string | null;
}

export type DelegationTarget = "claude_code" | "codex_cli";
export type DelegationMode = "analyze" | "edit";
export type DelegationStatus = "queued" | "running" | "succeeded" | "failed" | "cancelled";

export interface DelegationAttempt {
  contractVersion: 1;
  id: string;
  delegationId: string;
  attemptNumber: number;
  target: DelegationTarget;
  mode: DelegationMode;
  status: DelegationStatus;
  safeSummary: string | null;
  reportArtifactId: string | null;
  changeSetArtifactId: string | null;
  errorCode: string | null;
  startedAt: string | null;
  completedAt: string | null;
}

export interface DelegationView {
  contractVersion: 1;
  id: string;
  sessionId: string;
  taskHash: string;
  status: DelegationStatus;
  attempts: DelegationAttempt[];
  createdAt: string;
  updatedAt: string;
}

export interface ChangeSetFile {
  path: string;
  changeKind: "add" | "modify" | "delete" | "rename";
  oldHash: string | null;
  newHash: string | null;
  binary: boolean;
  mode: string | null;
}

export interface ChangeSetView {
  contractVersion: 1;
  artifactId: string;
  contentHash: string;
  repositoryIdentity: string;
  baseCommit: string;
  attemptId: string;
  files: ChangeSetFile[];
  warnings: string[];
  integrityStatus: "verified" | "missing" | "mismatch";
}

export type ChangeSetApplyStatus =
  | "awaiting_approval"
  | "preflighting"
  | "applying"
  | "verifying"
  | "succeeded"
  | "rolled_back"
  | "manual_recovery_required"
  | "failed";

export interface ChangeSetApplyAttempt {
  contractVersion: 1;
  id: string;
  changeSetArtifactId: string;
  targetRepositoryIdentity: string;
  expectedBaseCommit: string;
  status: ChangeSetApplyStatus;
  errorCode: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface ChangeSetRecoveryState {
  contractVersion: 1;
  applyAttemptId: string;
  status: "not_required" | "rolled_back" | "manual_recovery_required";
  recoveryReference: string | null;
  safeInstructions: string[];
  updatedAt: string;
}

export interface BrowserHumanHandoff {
  contractVersion: 1;
  operationId: string;
  status: "inactive" | "awaiting_user" | "user_controlling" | "resumed" | "expired";
  expiresAt: string | null;
}
