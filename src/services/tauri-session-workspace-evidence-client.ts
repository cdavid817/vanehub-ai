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
  parseEvidenceSubscriptionBootstrap,
  parseExecutionEvidenceNotice,
  parseExecutionRecordDetail,
  parseExecutionRecordPage,
  parseSessionRunReport,
  parseWorkspaceEvidenceSummary,
} from "../contracts/session-workspace-evidence";
import {
  createEvidenceNoticeDispatcher,
  onceUnsubscribe,
  type EvidenceNoticeDispatcher,
} from "./evidence-notice-stream";
import type { NativeEvidenceTransport } from "./native-evidence-transport";
import { createNativeEvidenceTransport } from "./tauri-native-evidence-transport";
import type { SessionWorkspaceEvidenceService } from "./session-workspace-evidence-service";

function boundedLimit(limit: number | undefined): number {
  if (limit === undefined) return EVIDENCE_PAGE_LIMITS.default;
  return Math.min(Math.max(1, Math.trunc(limit)), EVIDENCE_PAGE_LIMITS.maximum);
}

/**
 * The desktop evidence client, built around an injected transport.
 *
 * Everything the client is responsible for — request shaping, page bounds, schema validation,
 * sequence de-duplication, gap detection — is exercised against a fixture transport, and the same
 * cases then run against the registered native commands. Activating a capability is a change of
 * transport, not of client: no method here knows whether it is talking to SQLite or a fixture.
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

    /**
     * Listener first, bootstrap second, buffer in between.
     *
     * Calling the bootstrap first and subscribing after would lose every notice emitted in that
     * window, and nothing downstream could detect the loss: the sequences would be contiguous
     * from the subscriber's point of view. So the listener goes up first, everything that arrives
     * before the watermark is known is held, and the buffer is then replayed through the same
     * dispatcher that de-duplicates and detects gaps for live frames.
     */
    async subscribeExecutionEvidence(
      input: ExecutionEvidenceSubscription,
      listener: (event: ExecutionEvidenceNotice) => void,
    ): Promise<Unsubscribe> {
      let dispatcher: EvidenceNoticeDispatcher | null = null;
      const buffered: ExecutionEvidenceNotice[] = [];
      const unsubscribe = await transport.subscribeEvidenceNotices((payload) => {
        // A malformed notice is dropped rather than thrown: an event handler has no caller to
        // reject to, and one bad frame must not tear down a live subscription.
        const parsed = safeParseNotice(payload);
        if (!parsed) return;
        if (dispatcher) dispatcher.accept(parsed);
        else buffered.push(parsed);
      });

      const watermark = await resumeSequence(transport, input);
      dispatcher = createEvidenceNoticeDispatcher({
        sessionId: input.sessionId,
        fromSequence: watermark,
        listener,
      });
      // Ordered by sequence rather than arrival: the dispatcher's gap detection reads a jump in
      // the sequence, and replaying two frames out of order would report a gap that never existed.
      for (const notice of buffered.sort((a, b) => a.sequence - b.sequence)) {
        dispatcher.accept(notice);
      }
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
 * The sequence to resume from. A caller that states one wins: it knows what it already rendered,
 * whereas the watermark only says what the store holds. When it does not, the watermark is the
 * resume point — and if the bootstrap itself fails, 0 is used, which replays rather than skips.
 * An over-delivery is de-duplicated downstream; a skipped notice is gone.
 */
async function resumeSequence(
  transport: NativeEvidenceTransport,
  input: ExecutionEvidenceSubscription,
): Promise<number> {
  if (input.fromSequence !== undefined) return input.fromSequence;
  try {
    const bootstrap = parseEvidenceSubscriptionBootstrap(
      await transport.invokeEvidence("get_evidence_subscription_bootstrap", {
        sessionId: input.sessionId,
      }),
    );
    return bootstrap.watermarkSequence;
  } catch {
    return 0;
  }
}

/**
 * The binding the application uses. Summary, list, detail, and the subscription bootstrap reach
 * registered native commands; the session-run report stays typed-unavailable until 10.8.
 */
export const tauriSessionWorkspaceEvidenceClient: SessionWorkspaceEvidenceService =
  createTauriSessionWorkspaceEvidenceClient(createNativeEvidenceTransport());
