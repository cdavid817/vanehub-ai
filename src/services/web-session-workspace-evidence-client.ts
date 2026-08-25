import { matchesExecutionRecordQuery } from "../contracts/execution-record-filter";
import { evidenceCursorSchema } from "../contracts/session-workspace-evidence-ids";
import type {
  CursorPage,
  EvidenceCursor,
  ExecutionEvidenceNotice,
  ExecutionEvidenceSubscription,
  ExecutionRecord,
  ExecutionRecordDetail,
  ExecutionRecordDetailQuery,
  ExecutionRecordQuery,
  SessionRunReport,
  SessionRunReportExportResult,
  SessionRunReportQuery,
  Unsubscribe,
  WorkspaceEvidenceSummary,
  WorkspaceEvidenceSummaryQuery,
} from "../types/session-workspace-evidence";
import { EVIDENCE_PAGE_LIMITS } from "../types/session-workspace-evidence";
import { createEvidenceNoticeDispatcher, onceUnsubscribe } from "./evidence-notice-stream";
import { EvidenceUnavailableError } from "./native-evidence-transport";
import type { SessionWorkspaceEvidenceService } from "./session-workspace-evidence-service";
import {
  simulatedCoverage,
  WEB_EVIDENCE_CLOCK,
  webEvidenceSummary,
  webExecutionRecords,
  webSessionRunReport,
} from "./web-session-workspace-evidence-fixtures";

const CURSOR_PREFIX = "web-evidence-offset-";

function encodeCursor(offset: number): EvidenceCursor {
  return evidenceCursorSchema.parse(`${CURSOR_PREFIX}${offset}`);
}

/**
 * A cursor the mock did not issue is refused rather than coerced to zero. Silently restarting the
 * page is how a keyset list quietly repeats rows, and the desktop runtime answers the same case
 * with `cursor_filter_mismatch`.
 */
function decodeCursor(cursor: EvidenceCursor | undefined): number {
  if (cursor === undefined) return 0;
  if (!cursor.startsWith(CURSOR_PREFIX)) {
    throw new EvidenceUnavailableError("cursor_filter_mismatch");
  }
  const offset = Number.parseInt(cursor.slice(CURSOR_PREFIX.length), 10);
  if (!Number.isInteger(offset) || offset < 0) {
    throw new EvidenceUnavailableError("cursor_filter_mismatch");
  }
  return offset;
}

function boundedLimit(limit: number | undefined): number {
  if (limit === undefined) return EVIDENCE_PAGE_LIMITS.default;
  return Math.min(Math.max(1, Math.trunc(limit)), EVIDENCE_PAGE_LIMITS.maximum);
}

export interface SimulatedEvidenceNotice
  extends Omit<ExecutionEvidenceNotice, "sequence" | "occurredAt"> {
  /** Pin the sequence to script a gap; omit it to advance monotonically. */
  sequence?: number;
}

/**
 * The Web/mock client, plus the one affordance the desktop client has no equivalent for: a way to
 * advance the notice stream deterministically. Declared rather than smuggled in, so no caller has
 * to assert its way to it.
 */
export interface WebSessionWorkspaceEvidenceClient extends SessionWorkspaceEvidenceService {
  emitSimulatedNotice(notice: SimulatedEvidenceNotice): void;
}

/**
 * The Web/mock evidence adapter.
 *
 * Deterministic on purpose: a fixed clock, seeded ids, a monotonic notice sequence, and bounded
 * pages. Every coverage it returns carries a `simulated` reason code, so nothing it produces can
 * be mistaken for an observation of a real process, database, or remote host.
 */
export function createWebSessionWorkspaceEvidenceClient(): WebSessionWorkspaceEvidenceClient {
  const subscribers = new Set<(notice: ExecutionEvidenceNotice) => void>();
  let sequence = 0;

  return {
    async getWorkspaceEvidenceSummary(
      input: WorkspaceEvidenceSummaryQuery,
    ): Promise<WorkspaceEvidenceSummary> {
      return { ...webEvidenceSummary(), sessionId: input.sessionId };
    },

    async listExecutionRecords(input: ExecutionRecordQuery): Promise<CursorPage<ExecutionRecord>> {
      const offset = decodeCursor(input.cursor);
      const limit = boundedLimit(input.limit);
      const matching = webExecutionRecords()
        .map((record) => ({ ...record, sessionId: input.scope.sessionId }))
        .filter((record) => matchesExecutionRecordQuery(record, input));
      const items = matching.slice(offset, offset + limit);
      const nextOffset = offset + items.length;
      const hasMore = nextOffset < matching.length;
      return {
        items,
        nextCursor: hasMore ? encodeCursor(nextOffset) : undefined,
        coverage: simulatedCoverage({ truncated: hasMore }),
      };
    },

    async getExecutionRecord(input: ExecutionRecordDetailQuery): Promise<ExecutionRecordDetail> {
      const record = webExecutionRecords().find((candidate) => candidate.id === input.recordId);
      if (!record) throw new EvidenceUnavailableError("evidence_unavailable");
      return {
        record: { ...record, sessionId: input.sessionId },
        relatedCounts: { logs: 3, commands: 1, files: 2, findings: 1, usageObservations: 1 },
        safeAttributes: { runtime: "web-mock", simulated: "true" },
        errorReasonCode: record.status === "failed" ? "command_failed" : undefined,
      };
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
      const forward = (notice: ExecutionEvidenceNotice) => {
        dispatcher.accept(notice);
      };
      subscribers.add(forward);
      return onceUnsubscribe(() => {
        subscribers.delete(forward);
      });
    },

    async getSessionRunReport(input: SessionRunReportQuery): Promise<SessionRunReport> {
      const report = webSessionRunReport();
      return {
        ...report,
        scope: {
          ...report.scope,
          sessionId: input.sessionId,
          runIds: input.runIds ?? report.scope.runIds,
          seatIds: input.seatIds ?? report.scope.seatIds,
          from: input.from,
          to: input.to,
          groupBy: input.groupBy ?? report.scope.groupBy,
        },
      };
    },

    /**
     * Nowhere to write, so nothing is claimed.
     *
     * `simulated` rather than `exported` with a made-up path: a browser demo that reported a file
     * on disk would send somebody looking for one.
     */
    async exportSessionRunReport(): Promise<SessionRunReportExportResult> {
      return { status: "simulated", path: null };
    },

    emitSimulatedNotice(notice) {
      // A caller may pin the sequence to script a gap; otherwise it advances monotonically.
      sequence = notice.sequence ?? sequence + 1;
      subscribers.forEach((subscriber) => subscriber({
        ...notice,
        sequence,
        occurredAt: WEB_EVIDENCE_CLOCK,
      }));
    },
  };
}

export const webSessionWorkspaceEvidenceClient: WebSessionWorkspaceEvidenceClient =
  createWebSessionWorkspaceEvidenceClient();

/**
 * The same instance narrowed to the service contract, so composing it into the application service
 * cannot leak `emitSimulatedNotice` into a surface React could reach.
 */
export const webSessionWorkspaceEvidenceService: SessionWorkspaceEvidenceService =
  webSessionWorkspaceEvidenceClient;
