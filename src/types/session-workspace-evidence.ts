/**
 * The session-workspace evidence DTOs.
 *
 * Split into focused modules so no production file approaches the project line rule, and
 * re-exported here so callers depend on one contract surface rather than on where a type happens
 * to live today.
 */
export type {
  EvidenceAgentId,
  EvidenceBranded,
  EvidenceCommandId,
  EvidenceCursor,
  EvidenceOperationId,
  EvidenceRecordId,
  EvidenceRunId,
  EvidenceSeatId,
  EvidenceSessionId,
  EvidenceSpanId,
  EvidenceToolCallId,
  EvidenceTraceId,
} from "./session-workspace-evidence-ids";

export {
  EVIDENCE_PAGE_LIMITS,
} from "./session-workspace-evidence-core";

export type {
  CursorPage,
  EvidenceCoverageState,
  EvidenceFidelity,
  EvidenceStatus,
  EvidenceSubscriptionBootstrap,
  ExecutionEvidenceNotice,
  ExecutionEvidenceNoticeKind,
  ExecutionEvidenceSubscription,
  ExecutionRecordDetailQuery,
  ExecutionRecordFilters,
  ExecutionRecordQuery,
  QueryCoverage,
  Unsubscribe,
  WorkspaceEvidenceFocus,
  WorkspaceEvidenceScope,
  WorkspaceEvidenceSummary,
  WorkspaceEvidenceSummaryQuery,
  WorkspaceEvidenceTabId,
  WorkspaceEvidenceTarget,
} from "./session-workspace-evidence-core";

export type {
  CommandExecutionRecord,
  CommandOutputAvailability,
  CommandRuntimeKind,
  DelegationExecutionRecord,
  ExecutionRecord,
  ExecutionRecordBase,
  ExecutionRecordDetail,
  ExecutionRecordKind,
  LegacyActivityRecord,
  ToolExecutionRecord,
  VerificationExecutionRecord,
  VerificationOutcome,
} from "./session-workspace-evidence-records";

export type {
  AgentReportRow,
  ChangeReport,
  CommandReport,
  FailureReport,
  FailureReportRow,
  LatencyReport,
  ReportCoverage,
  ReportGroupBy,
  ReportSectionCoverage,
  ReportOverview,
  SessionRunReport,
  SessionRunReportQuery,
  SessionRunReportScope,
  SessionUsageReport,
  ToolReportRow,
  VerificationReport,
} from "./session-workspace-evidence-report";
