import type {
  EvidenceAgentId,
  EvidenceRunId,
  EvidenceSeatId,
  EvidenceSessionId,
} from "./session-workspace-evidence-ids";
import type {
  EvidenceCoverageState,
  QueryCoverage,
  WorkspaceEvidenceTarget,
} from "./session-workspace-evidence-core";

export type ReportGroupBy = "run" | "agent" | "seat" | "model" | "tool";

export interface SessionRunReportQuery {
  sessionId: EvidenceSessionId;
  runIds?: EvidenceRunId[];
  seatIds?: EvidenceSeatId[];
  from?: string;
  to?: string;
  groupBy?: ReportGroupBy;
}

export interface SessionRunReportScope {
  sessionId: EvidenceSessionId;
  runIds: EvidenceRunId[];
  seatIds: EvidenceSeatId[];
  from?: string;
  to?: string;
  groupBy: ReportGroupBy;
}

/**
 * Per-section rather than per-report. A report can be useful while one of its sources is still
 * indexing, and collapsing that into a single report-level state would either hide the gap or
 * discard the sections that are complete.
 */
export interface ReportSectionCoverage {
  state: EvidenceCoverageState;
  reasonCodes: string[];
}

export interface ReportCoverage {
  overall: EvidenceCoverageState;
  sections: {
    overview: ReportSectionCoverage;
    usage: ReportSectionCoverage;
    latency: ReportSectionCoverage;
    agents: ReportSectionCoverage;
    tools: ReportSectionCoverage;
    commands: ReportSectionCoverage;
    changes: ReportSectionCoverage;
    verification: ReportSectionCoverage;
    failures: ReportSectionCoverage;
  };
}

export interface ReportOverview {
  runCount: number;
  durationMs?: number;
  succeeded: number;
  failed: number;
  cancelled: number;
  retries: number;
}

/**
 * Reported, reported-derived, and estimated stay separate all the way to the UI. Adding them would
 * turn an estimate into a reported figure, which is exactly what the report must not do. Monetary
 * cost is absent unless a separately versioned pricing observation exists; this change adds none.
 */
export interface SessionUsageReport {
  reportedInputTokens?: number;
  reportedOutputTokens?: number;
  reportedDerivedTokens?: number;
  estimatedCharacters?: number;
  responseCount: number;
  internalPurposeResponseCount: number;
  coverage: ReportSectionCoverage;
  costAvailable: false;
}

export interface LatencyReport {
  p50Ms?: number;
  p95Ms?: number;
  slowestRecordDurationMs?: number;
}

export interface AgentReportRow {
  agentId?: EvidenceAgentId;
  seatId?: EvidenceSeatId;
  runCount: number;
  failedCount: number;
  durationMs?: number;
}

export interface ToolReportRow {
  toolName: string;
  invocations: number;
  failures: number;
  durationMs?: number;
}

export interface CommandReport {
  total: number;
  failed: number;
  running: number;
  durationMs?: number;
}

export interface ChangeReport {
  changedFiles: number;
  unviewedFiles: number;
  unresolvedFindings: number;
}

export interface VerificationReport {
  passed: number;
  failed: number;
  skipped: number;
}

export interface FailureReportRow {
  reasonCode: string;
  count: number;
  target?: WorkspaceEvidenceTarget;
}

export interface FailureReport {
  rows: FailureReportRow[];
}

export interface SessionRunReport {
  scope: SessionRunReportScope;
  generatedAt: string;
  coverage: ReportCoverage;
  overview: ReportOverview;
  usage: SessionUsageReport;
  latency: LatencyReport;
  agents: AgentReportRow[];
  tools: ToolReportRow[];
  commands: CommandReport;
  changes: ChangeReport;
  verification: VerificationReport;
  failures: FailureReport;
  evidenceLinks: WorkspaceEvidenceTarget[];
  sourceCoverage: QueryCoverage;
}
