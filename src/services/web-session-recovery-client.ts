import type { SessionRecoveryAcknowledgement } from "./agent-service";
import type { SessionRecoveryService } from "./session-recovery-service";
import { emitWebSessionEvent, findWebSession, updateWebSession } from "./web-session-state";
import {
  listWebRecoveryReports,
  mockRecoveryReport,
  setWebRecoveryReports,
  webRecoverySummary,
} from "./web-session-recovery-state";

export const webSessionRecoveryClient: SessionRecoveryService = {
  async getSessionRecoverySummary(sessionId: string) {
    return structuredClone(webRecoverySummary(sessionId));
  },

  async listSessionRecoveryReports(sessionId: string, limit = 20) {
    findWebSession(sessionId);
    const boundedLimit = Math.max(1, Math.min(100, Math.trunc(limit)));
    return structuredClone(listWebRecoveryReports(sessionId).slice(0, boundedLimit));
  },

  async acknowledgeSessionRecovery(
    sessionId: string,
    expectedRecoveryRevision: number,
  ): Promise<SessionRecoveryAcknowledgement> {
    const session = findWebSession(sessionId);
    if (session.recoveryStatus === "quarantined") {
      throw new Error(`Recovery acknowledgement is not allowed for quarantined session ${sessionId}.`);
    }
    if (session.recoveryStatus !== "action_required") {
      throw new Error(`Recovery acknowledgement is not allowed for session ${sessionId}.`);
    }
    if (session.recoveryRevision !== expectedRecoveryRevision) {
      throw new Error(
        `Recovery revision conflict for session ${sessionId}; current revision is ${session.recoveryRevision}.`,
      );
    }
    const recoveryRevision = session.recoveryRevision + 1;
    const updated = updateWebSession(sessionId, {
      recoveryStatus: "clean",
      recoveryRevision,
      stateRevision: session.stateRevision + 1,
      activeExecutionRunId: null,
    });
    const report = mockRecoveryReport(updated, recoveryRevision, "acknowledged");
    setWebRecoveryReports(sessionId, [report, ...listWebRecoveryReports(sessionId)]);
    emitWebSessionEvent({
      kind: "recovery-acknowledged",
      sessionId,
      recoveryRevision,
    });
    return structuredClone({ session: updated, report });
  },
};
