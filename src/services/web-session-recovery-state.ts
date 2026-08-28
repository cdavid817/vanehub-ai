import type {
  RecoveryDecision,
  SessionRecoveryReport,
  SessionRecoverySummary,
} from "./agent-service";
import type { Session } from "../types/agent";
import { nowIso } from "./web-mock-clock";
import { deleteWebActiveStream, deleteWebSessionMessages } from "./web-chat-state";
import {
  emitWebSessionEvent,
  findWebSession,
  getWebActiveSessionId,
  listWebSessions,
  nextWebSessionSequence,
  prependWebSession,
  replaceWebSessions,
  setWebActiveSessionId,
  updateWebSession,
} from "./web-session-state";

// Owned here and never exported. An exported mutable binding re-imported from two modules gives
// two divergent copies of the mock world, which surfaces as one UI panel showing stale data while
// another shows fresh. Callers reach the reports through the accessors below.
const recoveryReportsBySession = new Map<string, SessionRecoveryReport[]>();

export function mockRecoveryReport(
  session: Session,
  recoveryRevision: number,
  decision: RecoveryDecision,
): SessionRecoveryReport {
  return {
    reportId: `web-recovery-${session.id}-${recoveryRevision}`,
    sessionId: session.id,
    recoveryRevision,
    trigger: decision === "acknowledged" ? "user_acknowledgement" : "startup",
    observedLifecycle: session.lifecycleState,
    observedExecutionRunId: session.activeExecutionRunId,
    decision,
    reasonCodes: decision === "acknowledged"
      ? ["acknowledged_by_user"]
      : ["unfinished_tool_activity"],
    evidenceRefs: [{
      kind: "session",
      sessionId: session.id,
      stateRevision: session.stateRevision,
      historyRevision: session.historyRevision,
    }],
    createdAt: nowIso(),
  };
}

/** Returns the live array, matching what a direct `.get()` on the binding returned. */
export function listWebRecoveryReports(sessionId: string): SessionRecoveryReport[] {
  return recoveryReportsBySession.get(sessionId) ?? [];
}

export function setWebRecoveryReports(sessionId: string, reports: SessionRecoveryReport[]): void {
  recoveryReportsBySession.set(sessionId, reports);
}

export function deleteWebRecoveryReports(sessionId: string): void {
  recoveryReportsBySession.delete(sessionId);
}

export function webRecoverySummary(sessionId: string): SessionRecoverySummary {
  const session = findWebSession(sessionId);
  return {
    session,
    latestReport: recoveryReportsBySession.get(sessionId)?.[0] ?? null,
  };
}

export function seedWebRecoverySessionForTest(
  status: "action_required" | "quarantined" = "action_required",
): Session {
  const timestamp = nowIso();
  const session: Session = {
    id: `web-recovery-session-${nextWebSessionSequence()}`,
    title: "Recovered Web session",
    agentId: "onepiece",
    interactionMode: "api",
    personalizationMode: "standard", lifecycleState: "failed",
    recoveryStatus: "clean",
    recoveryRevision: 0,
    stateRevision: 0,
    historyRevision: 0,
    activeExecutionRunId: null,
    folder: "D:\\example\\recovery-project",
    projectPath: "D:\\example\\recovery-project",
    worktreePath: null,
    worktreeName: null,
    worktreeBranch: null,
    remoteWorkspace: null,
    remoteSshConnectionId: null,
    remoteSshConnectionRevision: null,
    runtimeSessionId: null,
    categoryId: null,
    pinned: false,
    archived: false,
    createdAt: timestamp,
    updatedAt: timestamp,
  };
  prependWebSession(session);
  setWebActiveSessionId(session.id);
  const recoveryRevision = 1;
  const recovered = updateWebSession(session.id, {
    personalizationMode: "standard", lifecycleState: "failed",
    recoveryStatus: status,
    recoveryRevision,
    stateRevision: session.stateRevision + 1,
    activeExecutionRunId: null,
  });
  recoveryReportsBySession.set(recovered.id, [
    mockRecoveryReport(
      recovered,
      recoveryRevision,
      status === "quarantined" ? "quarantined" : "action_required",
    ),
  ]);
  emitWebSessionEvent({
    kind: status === "quarantined" ? "recovery-quarantined" : "recovery-action-required",
    sessionId: recovered.id,
    recoveryRevision,
  });
  return recovered;
}

export function resetWebRecoverySessionsForTest() {
  const recoverySessionIds = new Set(recoveryReportsBySession.keys());
  replaceWebSessions(listWebSessions().filter((session) => !recoverySessionIds.has(session.id)));
  recoverySessionIds.forEach((sessionId) => {
    deleteWebSessionMessages(sessionId);
    deleteWebActiveStream(sessionId);
  });
  recoveryReportsBySession.clear();
  const activeId = getWebActiveSessionId();
  if (activeId && recoverySessionIds.has(activeId)) setWebActiveSessionId(null);
}
