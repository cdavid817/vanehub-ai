import type {
  EvidenceAgentId,
  EvidenceCommandId,
  EvidenceOperationId,
  EvidenceRecordId,
  EvidenceRunId,
  EvidenceSeatId,
  EvidenceSessionId,
  EvidenceSpanId,
  EvidenceToolCallId,
  EvidenceTraceId,
} from "./session-workspace-evidence-ids";
import type { EvidenceFidelity, EvidenceStatus, QueryCoverage } from "./session-workspace-evidence-core";

export type ExecutionRecordKind = "command" | "tool" | "delegation" | "verification" | "legacy";

export interface ExecutionRecordBase {
  id: EvidenceRecordId;
  kind: ExecutionRecordKind;
  sessionId: EvidenceSessionId;
  runId?: EvidenceRunId;
  traceId?: EvidenceTraceId;
  spanId?: EvidenceSpanId;
  operationId?: EvidenceOperationId;
  agentId?: EvidenceAgentId;
  seatId?: EvidenceSeatId;
  startedAt: string;
  endedAt?: string;
  durationMs?: number;
  status: EvidenceStatus;
  fidelity: EvidenceFidelity;
  coverage: QueryCoverage;
}

export type CommandRuntimeKind = "local-shell" | "remote-shell" | "process" | "unknown";

/**
 * What the runtime could actually observe about the output. A PTY hands back one merged stream, so
 * claiming stdout and stderr separately would be an invention; `merged` says so explicitly and
 * `unavailable` distinguishes "we never captured it" from "it was empty".
 */
export type CommandOutputAvailability = "merged" | "separate" | "unavailable" | "redacted";

export interface CommandExecutionRecord extends ExecutionRecordBase {
  kind: "command";
  commandId: EvidenceCommandId;
  runtimeKind: CommandRuntimeKind;
  /** Bounded and already redacted by the producer. Never the raw argument vector. */
  redactedDisplay?: string;
  cwdDisplay?: string;
  exitCode?: number;
  signal?: string;
  outputAvailability: CommandOutputAvailability;
  outputTruncated: boolean;
}

export interface ToolExecutionRecord extends ExecutionRecordBase {
  kind: "tool";
  toolCallId?: EvidenceToolCallId;
  toolName: string;
  /** `message-history` marks a projection of loaded chat messages rather than native evidence. */
  source: "native" | "message-history";
}

export interface DelegationExecutionRecord extends ExecutionRecordBase {
  kind: "delegation";
  parentAgentId?: EvidenceAgentId;
  childAgentId?: EvidenceAgentId;
  attempt?: number;
}

export type VerificationOutcome = "passed" | "failed" | "skipped" | "unknown";

export interface VerificationExecutionRecord extends ExecutionRecordBase {
  kind: "verification";
  verificationName: string;
  outcome: VerificationOutcome;
  passedCount?: number;
  failedCount?: number;
}

/**
 * Historical `message.toolUse` activity. It is never written into the journal as if it were native
 * evidence: its fidelity is `inferred` unless the message carried a verified native id, and its
 * coverage states that only loaded or persisted message activity is available.
 */
export interface LegacyActivityRecord extends ExecutionRecordBase {
  kind: "legacy";
  label: string;
  source: "message-history";
  messageId: string;
}

export type ExecutionRecord =
  | CommandExecutionRecord
  | ToolExecutionRecord
  | DelegationExecutionRecord
  | VerificationExecutionRecord
  | LegacyActivityRecord;

export interface ExecutionRecordDetail {
  record: ExecutionRecord;
  /** Counts only. The drawer queries each owning service rather than embedding its rows here. */
  relatedCounts: {
    logs: number;
    commands: number;
    files: number;
    findings: number;
    usageObservations: number;
  };
  safeAttributes: Record<string, string>;
  errorReasonCode?: string;
}
