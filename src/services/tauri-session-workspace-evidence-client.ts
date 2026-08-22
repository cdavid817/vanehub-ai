import type {
  CursorPage,
  ExecutionEvidenceNotice,
  ExecutionEvidenceSubscription,
  ExecutionRecord,
  ExecutionRecordDetail,
  ExecutionRecordDetailQuery,
  ExecutionRecordQuery,
  SessionRunReport,
  SessionRunReportQuery,
  Unsubscribe,
  WorkspaceEvidenceSummary,
  WorkspaceEvidenceSummaryQuery,
} from "../types/session-workspace-evidence";
import { EVIDENCE_PAGE_LIMITS } from "../types/session-workspace-evidence";
import {
  parseExecutionEvidenceNotice,
  parseExecutionRecordDetail,
  parseExecutionRecordPage,
  parseSessionRunReport,
  parseWorkspaceEvidenceSummary,
} from "../contracts/session-workspace-evidence";
import { createEvidenceNoticeDispatcher, onceUnsubscribe } from "./evidence-notice-stream";
import {
  unavailableEvidenceTransport,
  type NativeEvidenceTransport,
} from "./native-evidence-transport";
import type { SessionWorkspaceEvidenceService } from "./session-workspace-evidence-service";

function boundedLimit(limit: number | undefined): number {
  if (limit === undefined) return EVIDENCE_PAGE_LIMITS.default;
  return Math.min(Math.max(1, Math.trunc(limit)), EVIDENCE_PAGE_LIMITS.maximum);
}

/**
 * The desktop evidence client, built around an injected transport.
 *
 * Everything the client is responsible for — request shaping, page bounds, schema validation,
 * sequence de-duplication, gap detection — is exercised in Group 2 against a fixture transport.
 * The transport this is bound to in the application refuses with a typed reason code until the
 * commands exist (3.15 for evidence, 10.8 for the report), so no method here can reach an
 * unregistered command.
 */
export function createTauriSessionWorkspaceEvidenceClient(
  transport: NativeEvidenceTransport,
): SessionWorkspaceEvidenceService {
  return {
    async getWorkspaceEvidenceSummary(
      input: WorkspaceEvidenceSummaryQuery,
    ): Promise<WorkspaceEvidenceSummary> {
      return parseWorkspaceEvidenceSummary(
        await transport.invokeEvidence("get_workspace_evidence_summary", {
          sessionId: input.sessionId,
          seatId: input.seatId ?? null,
        }),
      );
    },

    async listExecutionRecords(input: ExecutionRecordQuery): Promise<CursorPage<ExecutionRecord>> {
      return parseExecutionRecordPage(
        await transport.invokeEvidence("list_execution_records", {
          scope: input.scope,
          filters: input.filters ?? null,
          cursor: input.cursor ?? null,
          limit: boundedLimit(input.limit),
        }),
      );
    },

    async getExecutionRecord(input: ExecutionRecordDetailQuery): Promise<ExecutionRecordDetail> {
      return parseExecutionRecordDetail(
        await transport.invokeEvidence("get_execution_record", {
          sessionId: input.sessionId,
          recordId: input.recordId,
        }),
      );
    },

    async subscribeExecutionEvidence(
      input: ExecutionEvidenceSubscription,
      listener: (event: ExecutionEvidenceNotice) => void,
    ): Promise<Unsubscribe> {
      const dispatcher = createEvidenceNoticeDispatcher({
        sessionId: input.sessionId,
        fromSequence: input.fromSequence,
        listener,
      });
      const unsubscribe = await transport.subscribeEvidenceNotices((payload) => {
        // A malformed notice is dropped rather than thrown: an event handler has no caller to
        // reject to, and one bad frame must not tear down a live subscription.
        const parsed = safeParseNotice(payload);
        if (parsed) dispatcher.accept(parsed);
      });
      return onceUnsubscribe(unsubscribe);
    },

    async getSessionRunReport(input: SessionRunReportQuery): Promise<SessionRunReport> {
      return parseSessionRunReport(
        await transport.invokeEvidence("get_session_run_report", {
          sessionId: input.sessionId,
          runIds: input.runIds ?? null,
          seatIds: input.seatIds ?? null,
          from: input.from ?? null,
          to: input.to ?? null,
          groupBy: input.groupBy ?? null,
        }),
      );
    },
  };
}

function safeParseNotice(payload: unknown): ExecutionEvidenceNotice | null {
  try {
    return parseExecutionEvidenceNotice(payload);
  } catch {
    return null;
  }
}

/**
 * The binding the application uses today. It stays typed-unavailable until the native commands are
 * registered; swapping the transport is the whole of that activation.
 */
export const tauriSessionWorkspaceEvidenceClient: SessionWorkspaceEvidenceService =
  createTauriSessionWorkspaceEvidenceClient(unavailableEvidenceTransport);
