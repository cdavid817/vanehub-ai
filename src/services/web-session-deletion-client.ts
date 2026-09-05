import type { Session } from "../types/agent";
import type {
  DeletionPreviewWorktree,
  ExecuteSessionDeletionInput,
  PreviewSessionDeletionInput,
  RetrySessionDeletionInput,
  SessionDeletionHandle,
  SessionDeletionOperation,
  SessionDeletionPreview,
  WorktreeDeletionChoice,
} from "../types/session-deletion";
import type { SessionDeletionService } from "./session-deletion-service";
import { nowIso } from "./web-mock-clock";
import { listWebLoopRuns } from "./web-loop-state";
import { runWebDeletionOperation } from "./web-session-deletion-runner";
import { claimWebSessions, releaseWebSessionClaims, WEB_SESSION_CLAIMED } from "./web-session-deletion-state";
import { findWebSession, getWebActiveSessionId, listWebSessions } from "./web-session-state";
import {
  buildWebDeletionGroups,
  buildWebDeletionPreviewRow,
  hashWebDeletionRequest,
  resolveWebDeletionChoices,
} from "./web-session-deletion-simulation";

/**
 * The Web/mock adapter. Every preview, handle and result is marked `simulated`: nothing here
 * touches a filesystem, so nothing here may claim a directory was removed. Scenario toggles live
 * in `web-session-deletion-simulation` so tests can stage failures, refusals and partial batches.
 */
const PREVIEW_TTL_MS = 10 * 60 * 1000;
const MAX_BATCH = 100;

interface StoredPreview {
  preview: SessionDeletionPreview;
  expiresAt: number;
}

const previews = new Map<string, StoredPreview>();
const operations = new Map<string, SessionDeletionOperation>();
const requests = new Map<string, { operationId: string; hash: string }>();
const retryRequests = new Map<string, string>();
let sequence = 1;

function nextId(prefix: string) {
  const id = `${prefix}-${sequence}`;
  sequence += 1;
  return id;
}

function validationError(code: string): Error {
  return new Error(`validation error: ${code}`);
}

function normalizeSelection(sessionIds: string[]): string[] {
  const ordered: string[] = [];
  for (const raw of sessionIds) {
    const id = raw.trim();
    if (!id || ordered.includes(id)) continue;
    if (id.startsWith("system-activity-v1-")) throw validationError("system_activity_session_refused");
    ordered.push(id);
  }
  if (ordered.length === 0) throw validationError("deletion_empty_selection");
  if (ordered.length > MAX_BATCH) throw validationError("deletion_batch_too_large");
  return ordered;
}

function touch(operation: SessionDeletionOperation) {
  operation.revision += 1;
  operation.updatedAt = nowIso();
}

function schedule(operationId: string) {
  // Deferred rather than inline so a handle is observably "accepted, not finished" — the same
  // shape the native runtime has, where the work runs on a blocking pool after the reply.
  void Promise.resolve().then(() => {
    const operation = operations.get(operationId);
    if (operation) runWebDeletionOperation(operation);
  });
}

function handle(operation: SessionDeletionOperation, existing: boolean): SessionDeletionHandle {
  return { operationId: operation.operationId, runtimeEffect: "simulated", operationTaskId: null, existing };
}

function snapshot(operation: SessionDeletionOperation): SessionDeletionOperation {
  return structuredClone(operation);
}

export const webSessionDeletionClient: SessionDeletionService = {
  async previewSessionDeletion(input: PreviewSessionDeletionInput) {
    const sessionIds = normalizeSelection(input.sessionIds);
    const sessions: Session[] = sessionIds.map((id) => findWebSession(id));
    const activeId = getWebActiveSessionId();
    const byPath = new Map<string, string[]>();
    const rows: SessionDeletionPreview["sessions"] = sessions.map((session) => {
      const isWorktree = Boolean(session.worktreePath) && !session.remoteWorkspace;
      if (isWorktree && session.worktreePath) {
        byPath.set(session.worktreePath, [...(byPath.get(session.worktreePath) ?? []), session.id]);
      }
      return {
        sessionId: session.id,
        title: session.title,
        archived: session.archived,
        active: activeId === session.id,
        workspaceKind: session.remoteWorkspace ? "remote" : isWorktree ? "worktree" : session.projectPath || session.folder ? "project" : "none",
        worktreeKey: isWorktree && session.worktreePath ? `wt:${session.worktreePath}` : null,
        displayPath: session.remoteWorkspace?.uri ?? session.worktreePath ?? session.folder ?? session.projectPath,
      };
    });
    const worktrees: DeletionPreviewWorktree[] = [...byPath.entries()].map(([path, ids]) =>
      buildWebDeletionPreviewRow(path, ids, listWebSessions(), listWebLoopRuns()));
    const preview: SessionDeletionPreview = {
      previewId: nextId("web-deletion-preview"),
      runtimeEffect: "simulated",
      createdAt: nowIso(),
      expiresAt: new Date(Date.now() + PREVIEW_TTL_MS).toISOString(),
      sessions: rows,
      worktrees,
    };
    previews.set(preview.previewId, { preview, expiresAt: Date.now() + PREVIEW_TTL_MS });
    return structuredClone(preview);
  },

  async executeSessionDeletion(input: ExecuteSessionDeletionInput) {
    const requestId = input.requestId.trim();
    if (!requestId) throw validationError("deletion_request_id_required");
    const stored = previews.get(input.previewId);
    if (!stored || stored.expiresAt < Date.now()) throw validationError("deletion_preview_expired");
    const resolved = resolveWebDeletionChoices(stored.preview.worktrees, input.worktreeChoices);
    const sessionIds = stored.preview.sessions.map((session) => session.sessionId);
    const hash = hashWebDeletionRequest(sessionIds, resolved);
    const known = requests.get(requestId);
    if (known) {
      if (known.hash !== hash) throw validationError("deletion_request_id_conflict");
      const existing = operations.get(known.operationId);
      if (existing) return handle(existing, true);
    }
    const operationId = nextId("web-session-deletion");
    const groups = buildWebDeletionGroups(stored.preview, resolved, () => nextId("web-deletion-group"));
    for (const group of groups) {
      const conflict = claimWebSessions(group.sessionIds, operationId);
      if (conflict) {
        for (const claimed of groups) releaseWebSessionClaims(claimed.sessionIds, operationId);
        throw validationError(WEB_SESSION_CLAIMED);
      }
    }
    const operation: SessionDeletionOperation = {
      operationId,
      requestId,
      outcome: "pending",
      phase: "accepted",
      revision: 1,
      runtimeEffect: "simulated",
      createdAt: nowIso(),
      updatedAt: nowIso(),
      completedAt: null,
      groups,
      errorCode: null,
      operationTaskId: null,
    };
    operations.set(operationId, operation);
    requests.set(requestId, { operationId, hash });
    previews.delete(input.previewId);
    schedule(operationId);
    return handle(operation, false);
  },

  async getSessionDeletionOperation(operationId: string) {
    const operation = operations.get(operationId);
    if (!operation) throw validationError("deletion_operation_not_found");
    return snapshot(operation);
  },

  async listPendingSessionDeletions() {
    return [...operations.values()].filter((operation) => operation.outcome === "pending").map(snapshot);
  },

  async retrySessionDeletion(input: RetrySessionDeletionInput) {
    const operation = operations.get(input.operationId);
    if (!operation) throw validationError("deletion_operation_not_found");
    if (operation.revision !== input.expectedRevision) throw validationError("deletion_revision_conflict");
    const retryId = input.retryRequestId.trim();
    if (!retryId) throw validationError("deletion_retry_request_id_required");
    if (retryRequests.get(operation.operationId) === retryId) return handle(operation, true);
    const stored = input.previewId ? previews.get(input.previewId) : undefined;
    if (input.previewId && (!stored || stored.expiresAt < Date.now())) throw validationError("deletion_preview_expired");
    const resolved = stored ? resolveWebDeletionChoices(stored.preview.worktrees, input.worktreeChoices) : [];
    let reopened = 0;
    for (const group of operation.groups) {
      if (group.status === "succeeded") continue;
      if (group.status === "needs_attention") throw validationError("deletion_retry_not_allowed");
      if (group.status === "finalize_pending") {
        group.status = "pending";
        group.errorCode = null;
        reopened += 1;
        continue;
      }
      const choice = resolved.find((entry) => entry.worktreeKey === group.worktreeKey);
      const policy = choice?.policy ?? "keep";
      if (policy === "remove-safe" && !stored) throw validationError("deletion_retry_requires_preview");
      if (!stored && group.policy === "remove-safe" && group.worktreeEffect !== "removed") {
        throw validationError("deletion_retry_requires_preview");
      }
      const conflict = claimWebSessions(group.sessionIds, operation.operationId);
      if (conflict) throw validationError(WEB_SESSION_CLAIMED);
      group.policy = policy;
      group.status = "pending";
      group.phase = "accepted";
      group.worktreeEffect = "not_requested";
      group.dbEffect = "pending";
      group.errorCode = null;
      reopened += 1;
    }
    if (reopened === 0) throw validationError("deletion_retry_not_allowed");
    if (input.previewId) previews.delete(input.previewId);
    retryRequests.set(operation.operationId, retryId);
    operation.outcome = "pending";
    operation.phase = "accepted";
    operation.errorCode = null;
    touch(operation);
    schedule(operation.operationId);
    return handle(operation, false);
  },
};

/** Test support: forgets every simulated preview, operation and claim. */
export function resetWebSessionDeletions(): void {
  previews.clear();
  operations.clear();
  requests.clear();
  retryRequests.clear();
}

export type { WorktreeDeletionChoice };
