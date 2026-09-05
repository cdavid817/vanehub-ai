import type { DeletionGroupResult, SessionDeletionOperation } from "../types/session-deletion";
import { nowIso } from "./web-mock-clock";
import { listWebLoopRuns } from "./web-loop-state";
import { deleteWebSessionRecord, releaseWebSessionClaims } from "./web-session-deletion-state";
import { listWebSessions } from "./web-session-state";
import {
  buildWebDeletionPreviewRow,
  simulatedRemovalOutcome,
  webDeletionScenario,
} from "./web-session-deletion-simulation";

/**
 * Drives one simulated operation the way the native coordinator drives a real one: quiesce,
 * revalidate, "remove", then delete the rows — recording each effect separately so a partial or
 * uncertain result is reported as exactly that.
 */
export function terminalOutcome(groups: DeletionGroupResult[]): SessionDeletionOperation["outcome"] {
  if (groups.some((group) => group.status === "pending" || group.status === "running")) return "pending";
  if (groups.some((group) => group.status === "finalize_pending" || group.status === "needs_attention")) return "needs_attention";
  const succeeded = groups.filter((group) => group.status === "succeeded").length;
  if (succeeded === groups.length) return "succeeded";
  if (succeeded > 0) return "partial";
  if (groups.some((group) => group.status === "awaiting_decision")) return "awaiting_decision";
  return "failed";
}

function settle(group: DeletionGroupResult, status: DeletionGroupResult["status"], errorCode: string | null, operationId: string) {
  group.status = status;
  group.errorCode = errorCode;
  group.revision += 1;
  if (status !== "finalize_pending" && status !== "needs_attention") releaseWebSessionClaims(group.sessionIds, operationId);
}

function runGroup(operation: SessionDeletionOperation, group: DeletionGroupResult) {
  const scenario = webDeletionScenario();
  group.status = "running";
  group.attempt += 1;
  group.phase = "quiescing";
  const stuck = group.sessionIds.find((id) => scenario.quiesceTimeoutSessions.has(id));
  if (stuck) {
    group.worktreeEffect = group.policy === "remove-safe" ? "retained" : "not_requested";
    group.dbEffect = "retained";
    settle(group, "failed", "deletion_quiesce_timeout", operation.operationId);
    return;
  }
  if (group.policy === "remove-safe" && group.worktreeEffect !== "removed") {
    group.phase = "revalidating";
    const path = group.retainedPath ?? "";
    const row = buildWebDeletionPreviewRow(path, group.sessionIds, listWebSessions(), listWebLoopRuns());
    if (!row.allowedPolicies.includes("remove-safe")) {
      group.worktreeEffect = "retained";
      group.dbEffect = "retained";
      settle(group, "awaiting_decision", row.blockers[0] ?? "worktree_removal_refused", operation.operationId);
      return;
    }
    group.phase = "removing_worktree";
    group.worktreeEffect = "remove_started";
    const outcome = simulatedRemovalOutcome(path);
    if (outcome === "refused") {
      group.worktreeEffect = "retained";
      group.dbEffect = "retained";
      settle(group, "awaiting_decision", "worktree_removal_refused", operation.operationId);
      return;
    }
    if (outcome === "unknown") {
      group.worktreeEffect = "removal_unknown";
      settle(group, "needs_attention", "worktree_removal_unknown", operation.operationId);
      return;
    }
    group.worktreeEffect = "removed";
  } else if (group.policy !== "remove-safe" && group.worktreeId) {
    group.worktreeEffect = "retained";
  }
  group.phase = "deleting_sessions";
  if (scenario.finalizeFailureSessions.some((id) => group.sessionIds.includes(id))) {
    if (group.worktreeEffect === "removed") {
      group.dbEffect = "pending";
      settle(group, "finalize_pending", "session_finalize_failed", operation.operationId);
    } else {
      group.dbEffect = "retained";
      settle(group, "failed", "session_finalize_failed", operation.operationId);
    }
    return;
  }
  for (const sessionId of group.sessionIds) {
    if (listWebSessions().some((session) => session.id === sessionId)) deleteWebSessionRecord(sessionId);
  }
  group.dbEffect = "deleted";
  group.phase = "completed";
  settle(group, "succeeded", null, operation.operationId);
}

export function runWebDeletionOperation(operation: SessionDeletionOperation) {
  if (operation.outcome !== "pending") return;
  operation.phase = "quiescing";
  for (const group of operation.groups) {
    if (group.status === "pending" || group.status === "running") runGroup(operation, group);
  }
  operation.outcome = terminalOutcome(operation.groups);
  operation.phase = "completed";
  operation.errorCode = operation.groups.find((group) => group.errorCode)?.errorCode ?? null;
  operation.completedAt = nowIso();
  operation.revision += 1;
  operation.updatedAt = nowIso();
}
