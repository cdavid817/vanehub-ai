/**
 * Session deletion with optional, conservative worktree cleanup.
 *
 * Mirrors the native `session-deletion-operations` contract. Every literal here is the same
 * string the Rust coordinator writes to its journal, so a phase or an effect never needs a
 * mapping layer in which it could drift.
 */

export type WorktreeDeletionPolicy = "keep" | "remove-safe";
export type DeletionRuntimeEffect = "native" | "simulated";
export type DeletionCheckCompleteness = "complete" | "incomplete";
export type DeletionWorkspaceKind = "project" | "remote" | "worktree" | "none";

export interface PreviewSessionDeletionInput {
  /** Session ids only. The backend resolves every path; none is ever accepted from here. */
  sessionIds: string[];
}

export interface DeletionPreviewSession {
  sessionId: string;
  title: string;
  archived: boolean;
  active: boolean;
  workspaceKind: DeletionWorkspaceKind;
  worktreeKey: string | null;
  displayPath: string | null;
}

export interface DeletionExternalReference {
  kind: string;
  id: string;
  label: string;
}

export interface DeletionChangeSummary {
  trackedModified: number;
  staged: number;
  conflicted: number;
  untracked: number;
}

export interface DeletionIgnoredSample {
  path: string;
  kind: string;
  size: number;
  modifiedUnix: number;
}

export interface DeletionIgnoredSummary {
  totalEntries: number;
  samples: DeletionIgnoredSample[];
  samplesTruncated: boolean;
  completeness: DeletionCheckCompleteness;
  fingerprint: string;
}

export interface DeletionPreviewWorktree {
  /** Opaque group key. Only a key with a `worktreeId` can ever be chosen for removal. */
  worktreeKey: string;
  worktreeId: string | null;
  displayPath: string;
  branch: string | null;
  sessionIds: string[];
  externalReferences: DeletionExternalReference[];
  allowedPolicies: WorktreeDeletionPolicy[];
  blockers: string[];
  checks: DeletionCheckCompleteness;
  changes: DeletionChangeSummary | null;
  ignored: DeletionIgnoredSummary | null;
  requiresIgnoredAcknowledgement: boolean;
  origin: string;
  provenance: string;
  resourceStatus: string | null;
}

export interface SessionDeletionPreview {
  previewId: string;
  runtimeEffect: DeletionRuntimeEffect;
  createdAt: string;
  expiresAt: string;
  sessions: DeletionPreviewSession[];
  worktrees: DeletionPreviewWorktree[];
}

export interface WorktreeDeletionChoice {
  worktreeKey: string;
  policy: WorktreeDeletionPolicy;
  ignoredFilesAcknowledgement?: { fingerprint: string };
}

export interface ExecuteSessionDeletionInput {
  /** Stable across retransmissions of the same request; never reused for different content. */
  requestId: string;
  previewId: string;
  worktreeChoices: WorktreeDeletionChoice[];
}

export interface SessionDeletionHandle {
  operationId: string;
  runtimeEffect: DeletionRuntimeEffect;
  operationTaskId: string | null;
  /** The request had already been accepted; nothing new was started. */
  existing: boolean;
}

export type DeletionOutcome =
  | "pending"
  | "succeeded"
  | "failed"
  | "partial"
  | "awaiting_decision"
  | "needs_attention";

export type DeletionPhase =
  | "accepted"
  | "quiescing"
  | "revalidating"
  | "removing_worktree"
  | "deleting_sessions"
  | "completed";

export type WorktreeEffect = "not_requested" | "retained" | "remove_started" | "removed" | "removal_unknown";
export type SessionDbEffect = "pending" | "deleted" | "retained";
export type DeletionGroupStatus =
  | "pending"
  | "running"
  | "succeeded"
  | "failed"
  | "awaiting_decision"
  | "finalize_pending"
  | "needs_attention";

export interface DeletionGroupResult {
  groupId: string;
  worktreeKey: string | null;
  worktreeId: string | null;
  policy: WorktreeDeletionPolicy;
  sessionIds: string[];
  status: DeletionGroupStatus;
  phase: DeletionPhase;
  worktreeEffect: WorktreeEffect;
  dbEffect: SessionDbEffect;
  errorCode: string | null;
  retainedPath: string | null;
  attempt: number;
  revision: number;
}

export interface SessionDeletionOperation {
  operationId: string;
  requestId: string;
  outcome: DeletionOutcome;
  phase: DeletionPhase;
  revision: number;
  runtimeEffect: DeletionRuntimeEffect;
  createdAt: string;
  updatedAt: string;
  completedAt: string | null;
  groups: DeletionGroupResult[];
  errorCode: string | null;
  operationTaskId: string | null;
}

export interface RetrySessionDeletionInput {
  operationId: string;
  expectedRevision: number;
  retryRequestId: string;
  /** Required whenever a retried group asks for `remove-safe` again. */
  previewId?: string;
  worktreeChoices: WorktreeDeletionChoice[];
}

export const DELETION_TERMINAL_OUTCOMES: readonly DeletionOutcome[] = [
  "succeeded",
  "failed",
  "partial",
  "awaiting_decision",
  "needs_attention",
];

export function isDeletionOutcomeTerminal(outcome: DeletionOutcome): boolean {
  return DELETION_TERMINAL_OUTCOMES.includes(outcome);
}
