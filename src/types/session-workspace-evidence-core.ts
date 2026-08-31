import type {
  EvidenceCommandId,
  EvidenceCursor,
  EvidenceOperationId,
  EvidenceRecordId,
  EvidenceRunId,
  EvidenceSeatId,
  EvidenceSessionId,
  EvidenceSpanId,
  EvidenceTraceId,
} from "./session-workspace-evidence-ids";

/** How directly the producer observed what it reported. Never upgraded by a consumer. */
export type EvidenceFidelity = "native" | "proxied" | "inferred" | "opaque";

export type EvidenceStatus =
  | "queued"
  | "running"
  | "succeeded"
  | "failed"
  | "cancelled"
  | "incomplete";

/**
 * Whether the answer describes the whole retained corpus. `complete` is a claim, not a default:
 * an empty page with `indexing` coverage means something different to the reader than an empty
 * page with `complete` coverage, and conflating them is the defect this field exists to prevent.
 */
export type EvidenceCoverageState = "complete" | "indexing" | "partial" | "unavailable";

export interface QueryCoverage {
  state: EvidenceCoverageState;
  reasonCodes: string[];
  oldestAvailableAt?: string;
  newestAvailableAt?: string;
  indexedThroughAt?: string;
  droppedCount?: number;
  truncated: boolean;
}

/**
 * What a subscriber needs before it applies live notices: the sequence the store has committed
 * through. Without it a listener registered before the first page cannot tell a notice it already
 * has from one describing work it has not seen — the two look identical.
 */
export interface EvidenceSubscriptionBootstrap {
  sessionId: EvidenceSessionId;
  watermarkSequence: number;
  coverage: QueryCoverage;
}

export interface CursorPage<Item> {
  items: Item[];
  nextCursor?: EvidenceCursor;
  coverage: QueryCoverage;
}

/** The only cross-panel selection contract. Serializable so it can survive a tab switch. */
export interface WorkspaceEvidenceScope {
  sessionId: EvidenceSessionId;
  seatId?: EvidenceSeatId;
  runId?: EvidenceRunId;
  traceId?: EvidenceTraceId;
  spanId?: EvidenceSpanId;
  operationId?: EvidenceOperationId;
  commandId?: EvidenceCommandId;
  relativePath?: string;
  hunkFingerprint?: string;
  occurredAt?: string;
}

export type WorkspaceEvidenceTabId =
  | "terminal-history"
  | "shell"
  | "logs"
  | "traces"
  | "changes"
  | "files"
  | "report";

export type WorkspaceEvidenceFocus = "row" | "detail" | "filter" | "timestamp";

export interface WorkspaceEvidenceTarget {
  tab: WorkspaceEvidenceTabId;
  scope: WorkspaceEvidenceScope;
  focus?: WorkspaceEvidenceFocus;
}

export interface WorkspaceEvidenceSummaryQuery {
  sessionId: EvidenceSessionId;
  seatId?: EvidenceSeatId;
}

export interface WorkspaceEvidenceSummary {
  sessionId: EvidenceSessionId;
  generatedAt: string;
  coverage: QueryCoverage;
  runState: { status: EvidenceStatus; runId?: EvidenceRunId; startedAt?: string };
  changes: { changedFiles: number; unviewedFiles: number };
  executionRecords: { running: number; failed: number };
  shells: { live: number };
  logs: { newErrors: number };
  traces: { running: number; failed: number };
  verification: { passed: number; failed: number };
  usage: { reportedTokens?: number; coverage: EvidenceCoverageState };
}

export type ExecutionEvidenceNoticeKind =
  | "record-appended"
  | "record-updated"
  | "summary-changed"
  | "coverage-gap";

/**
 * Identifiers and counts only. A notice never carries a command line, a log message, file content,
 * or a prompt, because it crosses the Tauri event channel where redaction cannot be re-applied.
 */
export interface ExecutionEvidenceNotice {
  kind: ExecutionEvidenceNoticeKind;
  sequence: number;
  sessionId: EvidenceSessionId;
  occurredAt: string;
  recordId?: EvidenceRecordId;
  runId?: EvidenceRunId;
  traceId?: EvidenceTraceId;
  spanId?: EvidenceSpanId;
  operationId?: EvidenceOperationId;
  commandId?: EvidenceCommandId;
  seatId?: EvidenceSeatId;
  droppedCount?: number;
}

export interface ExecutionEvidenceSubscription {
  sessionId: EvidenceSessionId;
  seatId?: EvidenceSeatId;
  /** Resume point. Frames at or below this sequence are de-duplicated by the subscriber. */
  fromSequence?: number;
}

export type Unsubscribe = () => void;

export interface ExecutionRecordFilters {
  kinds?: string[];
  statuses?: EvidenceStatus[];
  fidelities?: EvidenceFidelity[];
  search?: string;
}

export interface ExecutionRecordQuery {
  scope: WorkspaceEvidenceScope;
  filters?: ExecutionRecordFilters;
  cursor?: EvidenceCursor;
  limit?: number;
}

export interface ExecutionRecordDetailQuery {
  sessionId: EvidenceSessionId;
  recordId: EvidenceRecordId;
}

/** Bounds live here rather than in a React literal so a panel cannot quietly raise them. */
export const EVIDENCE_PAGE_LIMITS = {
  default: 100,
  maximum: 500,
} as const;
