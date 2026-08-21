import { invoke } from "@tauri-apps/api/core";
import type {
  SessionRecoveryAcknowledgement,
  SessionRecoveryReport,
  SessionRecoverySummary,
} from "./agent-service";
import type { SessionRecoveryService, SessionRuntimeRecovery } from "./session-recovery-service";

/**
 * Mirrors `web-session-recovery-client.ts` so the two runtimes' recovery surfaces stay
 * side-by-side and a new operation is obviously missing from one of them.
 */
export const tauriSessionRecoveryClient: SessionRecoveryService = {
  getSessionRecoverySummary(sessionId: string) {
    return invoke<SessionRecoverySummary>("get_session_recovery_summary", { sessionId });
  },

  listSessionRecoveryReports(sessionId: string, limit?: number) {
    return invoke<SessionRecoveryReport[]>("list_session_recovery_reports", {
      sessionId,
      limit: limit ?? null,
    });
  },

  acknowledgeSessionRecovery(sessionId: string, expectedRecoveryRevision: number) {
    return invoke<SessionRecoveryAcknowledgement>("acknowledge_session_recovery", {
      sessionId,
      expectedRecoveryRevision,
    });
  },

  recoverSession(sessionId: string) {
    return invoke<SessionRuntimeRecovery>("recover_session", { sessionId });
  },
};
