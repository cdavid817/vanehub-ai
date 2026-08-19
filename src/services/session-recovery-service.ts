import type {
  SessionRecoveryAcknowledgement,
  SessionRecoveryReport,
  SessionRecoverySummary,
} from "./agent-service";

export interface SessionRecoveryService {
  getSessionRecoverySummary(sessionId: string): Promise<SessionRecoverySummary>;
  listSessionRecoveryReports(sessionId: string, limit?: number): Promise<SessionRecoveryReport[]>;
  acknowledgeSessionRecovery(
    sessionId: string,
    expectedRecoveryRevision: number,
  ): Promise<SessionRecoveryAcknowledgement>;
}
