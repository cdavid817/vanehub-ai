import type {
  CursorPage,
  ExecutionEvidenceNotice,
  ExecutionEvidenceSubscription,
  ExecutionRecord,
  ExecutionRecordDetail,
  ExecutionRecordDetailQuery,
  ExecutionRecordQuery,
  SessionRunReport,
  SessionRunReportExportQuery,
  SessionRunReportExportResult,
  SessionRunReportQuery,
  Unsubscribe,
  WorkspaceEvidenceSummary,
  WorkspaceEvidenceSummaryQuery,
} from "../types/session-workspace-evidence";

/**
 * The evidence reads the workspace panels depend on.
 *
 * A focused interface rather than more methods on `AgentService`: the evidence surface grows over
 * several task groups, and composing it keeps the application service from becoming the place
 * every capability lands by default.
 *
 * Every method may fail with an `EvidenceUnavailableError`. That is not an exceptional path — it
 * is how a runtime says a capability is not wired yet, or that the query index cannot answer
 * safely, without the caller having to interpret a framework error string.
 */
export interface SessionWorkspaceEvidenceService {
  getWorkspaceEvidenceSummary(input: WorkspaceEvidenceSummaryQuery): Promise<WorkspaceEvidenceSummary>;

  listExecutionRecords(input: ExecutionRecordQuery): Promise<CursorPage<ExecutionRecord>>;

  getExecutionRecord(input: ExecutionRecordDetailQuery): Promise<ExecutionRecordDetail>;

  /**
   * Notices are identifier-only and monotonic. The returned unsubscribe is idempotent, because a
   * React effect cleanup can run more than once and must not tear down a later subscription.
   */
  subscribeExecutionEvidence(
    input: ExecutionEvidenceSubscription,
    listener: (event: ExecutionEvidenceNotice) => void,
  ): Promise<Unsubscribe>;

  getSessionRunReport(input: SessionRunReportQuery): Promise<SessionRunReport>;

  /**
   * Writes the bounded report as JSON through the runtime's own destination flow.
   *
   * No `Blob` and no download: a frontend that wrote the file itself would be a file write with a
   * path the backend never saw, which is the one thing the export rules exist to prevent. The Web
   * runtime has nowhere to write and says `simulated` rather than inventing a path.
   */
  exportSessionRunReport(input: SessionRunReportExportQuery): Promise<SessionRunReportExportResult>;
}
