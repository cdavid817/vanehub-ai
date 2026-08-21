import type { SessionLifecycleState } from "../types/agent";
import type {
  SessionRecoveryAcknowledgement,
  SessionRecoveryReport,
  SessionRecoverySummary,
} from "./agent-service";

/** What `recoverSession` actually released, so the UI can say "nothing was stuck" honestly. */
export interface SessionRuntimeRecovery {
  cancelledMessageIds: string[];
  processStopped: boolean;
  lifecycleState: SessionLifecycleState;
}

export interface SessionRecoveryService {
  getSessionRecoverySummary(sessionId: string): Promise<SessionRecoverySummary>;
  listSessionRecoveryReports(sessionId: string, limit?: number): Promise<SessionRecoveryReport[]>;
  acknowledgeSessionRecovery(
    sessionId: string,
    expectedRecoveryRevision: number,
  ): Promise<SessionRecoveryAcknowledgement>;
  /**
   * Returns a stuck session runtime to a state that accepts messages: cancels a lingering
   * generation and any message left streaming, then resets the lifecycle to idle. It never starts
   * an Agent process — the session answers the user's next message on its own.
   *
   * Distinct from `acknowledgeSessionRecovery`, which decides what to do about reconciled business
   * evidence after a crash. Runtime failure and evidence reconciliation are independent concerns.
   */
  recoverSession(sessionId: string): Promise<SessionRuntimeRecovery>;
}
