import type { Session } from "../../types/agent";
import type {
  DeletionGroupResult,
  DeletionPreviewWorktree,
  SessionDeletionOperation,
  SessionDeletionPreview,
  WorktreeDeletionChoice,
} from "../../types/session-deletion";

/**
 * The deletion dialog's state machine, kept pure so every transition can be tested without a
 * DOM. Destructive consent lives only in `choices`, which is rebuilt empty for every new
 * preview: nothing here is ever read back from storage.
 */
export interface WorktreeChoiceState {
  remove: boolean;
  /** The fingerprint the user acknowledged, or null. Compared against the preview's own. */
  acknowledgedFingerprint: string | null;
}

export interface RetryContext {
  operationId: string;
  expectedRevision: number;
}

export type DeletionDialogState =
  | { status: "closed" }
  | { status: "loading"; sessions: Session[]; retryOf: RetryContext | null }
  | { status: "preview-failed"; sessions: Session[]; error: string; retryOf: RetryContext | null }
  | {
    status: "ready";
    sessions: Session[];
    preview: SessionDeletionPreview;
    choices: Record<string, WorktreeChoiceState>;
    requestId: string;
    error: string | null;
    retryOf: RetryContext | null;
  }
  | {
    status: "executing";
    sessions: Session[];
    preview: SessionDeletionPreview;
    requestId: string;
    operationId: string;
    operation: SessionDeletionOperation | null;
  }
  | {
    status: "settled";
    sessions: Session[];
    preview: SessionDeletionPreview;
    operation: SessionDeletionOperation;
  };

export function emptyChoices(preview: SessionDeletionPreview): Record<string, WorktreeChoiceState> {
  return Object.fromEntries(preview.worktrees.map((worktree) => [
    worktree.worktreeKey,
    { remove: false, acknowledgedFingerprint: null },
  ]));
}

export function toggleRemove(
  choices: Record<string, WorktreeChoiceState>,
  worktree: DeletionPreviewWorktree,
): Record<string, WorktreeChoiceState> {
  if (!worktree.allowedPolicies.includes("remove-safe")) return choices;
  const current = choices[worktree.worktreeKey] ?? { remove: false, acknowledgedFingerprint: null };
  // Unticking removal also drops the acknowledgement: consent is per attempt, not remembered.
  return {
    ...choices,
    [worktree.worktreeKey]: current.remove
      ? { remove: false, acknowledgedFingerprint: null }
      : { remove: true, acknowledgedFingerprint: null },
  };
}

export function setAcknowledgement(
  choices: Record<string, WorktreeChoiceState>,
  worktree: DeletionPreviewWorktree,
  acknowledged: boolean,
): Record<string, WorktreeChoiceState> {
  const current = choices[worktree.worktreeKey] ?? { remove: false, acknowledgedFingerprint: null };
  return {
    ...choices,
    [worktree.worktreeKey]: {
      ...current,
      acknowledgedFingerprint: acknowledged ? worktree.ignored?.fingerprint ?? null : null,
    },
  };
}

export function anyRemovalChosen(choices: Record<string, WorktreeChoiceState>): boolean {
  return Object.values(choices).some((choice) => choice.remove);
}

/** Whether every chosen removal carries the acknowledgement its preview row requires. */
export function canSubmit(preview: SessionDeletionPreview, choices: Record<string, WorktreeChoiceState>): boolean {
  return preview.worktrees.every((worktree) => {
    const choice = choices[worktree.worktreeKey];
    if (!choice?.remove) return true;
    if (!worktree.allowedPolicies.includes("remove-safe")) return false;
    if (!worktree.requiresIgnoredAcknowledgement) return true;
    return Boolean(worktree.ignored) && choice.acknowledgedFingerprint === worktree.ignored?.fingerprint;
  });
}

export function buildChoices(
  preview: SessionDeletionPreview,
  choices: Record<string, WorktreeChoiceState>,
): WorktreeDeletionChoice[] {
  return preview.worktrees.map((worktree) => {
    const choice = choices[worktree.worktreeKey];
    if (!choice?.remove) return { worktreeKey: worktree.worktreeKey, policy: "keep" };
    return {
      worktreeKey: worktree.worktreeKey,
      policy: "remove-safe",
      ...(worktree.requiresIgnoredAcknowledgement && choice.acknowledgedFingerprint
        ? { ignoredFilesAcknowledgement: { fingerprint: choice.acknowledgedFingerprint } }
        : {}),
    };
  });
}

export function confirmLabelKey(removalChosen: boolean): string {
  return removalChosen ? "sessionDeletion.confirmWithWorktree" : "sessionDeletion.confirmSessionOnly";
}

/** A stable request id per preview, so a retransmission of the same confirmation matches. */
export function newRequestId(previewId: string): string {
  const random = typeof crypto !== "undefined" && "randomUUID" in crypto
    ? crypto.randomUUID()
    : `${Date.now()}-${Math.random().toString(16).slice(2)}`;
  return `${previewId}:${random}`;
}

/** Groups still worth another attempt: everything but the ones that finished. */
export function unfinishedGroups(operation: SessionDeletionOperation): DeletionGroupResult[] {
  return operation.groups.filter((group) => group.status !== "succeeded");
}

/** Sessions the operation did not delete, for a retry preview. */
export function remainingSessions(sessions: Session[], operation: SessionDeletionOperation): Session[] {
  const deleted = new Set(
    operation.groups.filter((group) => group.dbEffect === "deleted").flatMap((group) => group.sessionIds),
  );
  return sessions.filter((session) => !deleted.has(session.id));
}

/**
 * Whether the operation can be retried at all from the dialog. Groups parked for attention are
 * not retried here: their directory may be half gone and only a person can decide.
 */
export function retryAllowed(operation: SessionDeletionOperation): boolean {
  const unfinished = unfinishedGroups(operation);
  return unfinished.length > 0 && unfinished.every((group) => group.status !== "needs_attention");
}

/** Whether a retry needs a fresh preview: any unfinished group whose directory is still there. */
export function retryNeedsPreview(operation: SessionDeletionOperation): boolean {
  return unfinishedGroups(operation).some((group) => group.status !== "finalize_pending");
}

export function blockerKey(code: string): string {
  return `sessionDeletion.blocker.${code}`;
}

export function errorKey(code: string | null): string {
  return code ? `sessionDeletion.error.${code}` : "sessionDeletion.error.unknown";
}

export function phaseKey(phase: SessionDeletionOperation["phase"]): string {
  return `sessionDeletion.phase.${phase}`;
}

export function outcomeKey(outcome: SessionDeletionOperation["outcome"]): string {
  return `sessionDeletion.outcome.${outcome}`;
}

export function worktreeEffectKey(effect: DeletionGroupResult["worktreeEffect"]): string {
  return `sessionDeletion.effect.${effect}`;
}

export function dbEffectKey(effect: DeletionGroupResult["dbEffect"]): string {
  return `sessionDeletion.dbEffect.${effect}`;
}

/** Codes the backend may emit; anything else falls back to the generic key at render time. */
export const KNOWN_BLOCKERS = [
  "provenance_unverified", "origin_not_ordinary", "resource_status", "directory_missing", "not_git_worktree",
  "main_or_bare_workspace", "not_registered", "identity_mismatch", "locked", "prunable", "detached_head",
  "branch_not_resolving", "in_progress_operation", "nested_layout", "unsupported_layout", "tracked_changes",
  "staged_changes", "conflicts", "untracked_files", "changes_incomplete", "ignored_incomplete",
  "references_incomplete", "external_references", "gate_held", "no_anchor", "probe_failed", "git_unavailable",
] as const;
