import type { NativeEvidenceTransport } from "../../services/native-evidence-transport";
import { EvidenceUnavailableError } from "../../services/native-evidence-transport";
import type { Unsubscribe } from "../../types/session-workspace-evidence";

/**
 * Serialized payloads in exactly the shape the native commands are specified to return.
 *
 * These are the wire contract. They are written as plain JSON rather than built from the DTO types
 * on purpose: constructing them through the same types the parser produces would make the test
 * tautological, and the thing worth checking is that camelCase field names, optional-field
 * omission, and discriminants survive the boundary.
 */
export const nativeEvidenceFixtures = {
  summary: {
    sessionId: "session-1",
    generatedAt: "2026-08-22T10:00:00.000Z",
    coverage: {
      state: "partial",
      reasonCodes: ["log_index_repairing"],
      oldestAvailableAt: "2026-08-01T00:00:00.000Z",
      newestAvailableAt: "2026-08-22T10:00:00.000Z",
      indexedThroughAt: "2026-08-22T09:44:12.000Z",
      droppedCount: 2,
      truncated: false,
    },
    runState: { status: "running", runId: "run-124", startedAt: "2026-08-22T09:53:28.000Z" },
    changes: { changedFiles: 8, unviewedFiles: 4 },
    executionRecords: { running: 2, failed: 1 },
    shells: { live: 2 },
    logs: { newErrors: 3 },
    traces: { running: 1, failed: 1 },
    verification: { passed: 138, failed: 2 },
    usage: { reportedTokens: 112000, coverage: "partial" },
  },

  recordPage: {
    items: [
      {
        id: "record-1",
        kind: "command",
        sessionId: "session-1",
        runId: "run-124",
        traceId: "trace-9",
        spanId: "span-4",
        operationId: "operation-2",
        agentId: "agent-1",
        seatId: "seat-builder",
        startedAt: "2026-08-22T10:42:10.000Z",
        endedAt: "2026-08-22T10:42:22.400Z",
        durationMs: 12400,
        status: "failed",
        fidelity: "native",
        coverage: { state: "complete", reasonCodes: [], truncated: false },
        commandId: "command-1",
        runtimeKind: "local-shell",
        redactedDisplay: "npm test",
        cwdDisplay: "…/vanehub-ai",
        exitCode: 1,
        outputAvailability: "merged",
        outputTruncated: true,
      },
      {
        id: "record-2",
        kind: "tool",
        sessionId: "session-1",
        runId: "run-124",
        startedAt: "2026-08-22T10:41:51.000Z",
        durationMs: 31,
        status: "succeeded",
        fidelity: "proxied",
        coverage: { state: "complete", reasonCodes: [], truncated: false },
        toolCallId: "toolcall-7",
        toolName: "read_file",
        source: "native",
      },
      {
        // Completion-only: the runtime saw this finish but never saw it begin, so `startedAt` is
        // absent rather than back-derived from `endedAt` or `durationMs`.
        id: "record-4",
        kind: "command",
        sessionId: "session-1",
        runId: "run-124",
        endedAt: "2026-08-22T10:39:02.000Z",
        status: "incomplete",
        fidelity: "proxied",
        coverage: {
          state: "partial",
          reasonCodes: ["evidence_start_not_observed"],
          truncated: false,
        },
        commandId: "command-2",
        runtimeKind: "process",
        outputAvailability: "unavailable",
        outputTruncated: false,
      },
      {
        id: "record-3",
        kind: "legacy",
        sessionId: "session-1",
        startedAt: "2026-08-22T10:41:20.000Z",
        status: "incomplete",
        fidelity: "inferred",
        coverage: { state: "partial", reasonCodes: ["coverage_partial"], truncated: false },
        label: "shell toolUse",
        source: "message-history",
        messageId: "message-88",
      },
    ],
    nextCursor: "v1.eyJzZXEiOjEyfQ",
    coverage: { state: "complete", reasonCodes: [], truncated: true },
  },

  recordDetail: {
    record: {
      id: "record-1",
      kind: "command",
      sessionId: "session-1",
      startedAt: "2026-08-22T10:42:10.000Z",
      status: "failed",
      fidelity: "native",
      coverage: { state: "complete", reasonCodes: [], truncated: false },
      commandId: "command-1",
      runtimeKind: "remote-shell",
      outputAvailability: "unavailable",
      outputTruncated: false,
    },
    relatedCounts: { logs: 12, commands: 1, files: 3, findings: 2, usageObservations: 1 },
    safeAttributes: { "vanehub.runtime": "remote", "vanehub.exit": "1" },
    errorReasonCode: "command_failed",
  },

  subscriptionBootstrap: {
    sessionId: "session-1",
    // Zero: the fixture store has committed nothing, so a subscriber's own `fromSequence` decides
    // the resume point. A non-zero watermark here would mask a client that ignores the caller's.
    watermarkSequence: 0,
    coverage: { state: "partial", reasonCodes: ["evidence_capture_not_initialized"], truncated: false },
  },

  report: {
    scope: { sessionId: "session-1", runIds: ["run-124"], seatIds: ["seat-builder"], groupBy: "run" },
    generatedAt: "2026-08-22T10:45:00.000Z",
    coverage: {
      overall: "partial",
      sections: {
        overview: { state: "complete", reasonCodes: [] },
        usage: { state: "partial", reasonCodes: ["coverage_partial"] },
        latency: { state: "complete", reasonCodes: [] },
        agents: { state: "complete", reasonCodes: [] },
        tools: { state: "complete", reasonCodes: [] },
        commands: { state: "complete", reasonCodes: [] },
        changes: { state: "complete", reasonCodes: [] },
        verification: { state: "complete", reasonCodes: [] },
        failures: { state: "complete", reasonCodes: [] },
      },
    },
    overview: { runCount: 1, durationMs: 71000, succeeded: 0, failed: 1, cancelled: 0, retries: 1 },
    usage: {
      reportedInputTokens: 90000,
      reportedOutputTokens: 22000,
      responseCount: 3,
      internalPurposeResponseCount: 1,
      coverage: { state: "partial", reasonCodes: ["coverage_partial"] },
      costAvailable: false,
    },
    latency: { p50Ms: 31, p95Ms: 12400, slowestRecordDurationMs: 12400 },
    agents: [{ agentId: "agent-1", seatId: "seat-builder", runCount: 1, failedCount: 1 }],
    tools: [{ toolName: "read_file", invocations: 1, failures: 0, durationMs: 31 }],
    commands: { total: 1, failed: 1, running: 0, durationMs: 12400 },
    changes: { changedFiles: 8, unviewedFiles: 4, unresolvedFindings: 2 },
    verification: { passed: 138, failed: 2, skipped: 0 },
    failures: { rows: [{ reasonCode: "command_failed", count: 1 }] },
    evidenceLinks: [{ tab: "terminal-history", scope: { sessionId: "session-1", runId: "run-124" } }],
    sourceCoverage: { state: "partial", reasonCodes: ["coverage_partial"], truncated: false },
  },
  reportExport: { status: "exported", path: "D:/exports/vanehub-report-session-1.json" },
} as const;

export interface FixtureEvidenceTransport extends NativeEvidenceTransport {
  /** Payloads the client requested, so a test can assert page bounds and scope propagation. */
  requests: { command: string; payload: unknown }[];
  /** Push a raw notice payload as the native event channel would deliver it. */
  publish: (payload: unknown) => void;
}

/**
 * A transport that answers from the recorded payloads instead of `invoke()`. It is how Group 2
 * proves the client's serialization and subscription behaviour before any command is registered.
 */
export function createFixtureEvidenceTransport(
  overrides: Partial<Record<string, unknown>> = {},
): FixtureEvidenceTransport {
  const requests: { command: string; payload: unknown }[] = [];
  const handlers = new Set<(payload: unknown) => void>();
  const responses: Record<string, unknown> = {
    get_workspace_evidence_summary: nativeEvidenceFixtures.summary,
    list_execution_records: nativeEvidenceFixtures.recordPage,
    get_execution_record: nativeEvidenceFixtures.recordDetail,
    get_evidence_subscription_bootstrap: nativeEvidenceFixtures.subscriptionBootstrap,
    get_session_run_report: nativeEvidenceFixtures.report,
    export_session_run_report: nativeEvidenceFixtures.reportExport,
    ...overrides,
  };

  return {
    requests,
    publish(payload) {
      handlers.forEach((handler) => handler(payload));
    },
    invokeEvidence(command, payload) {
      requests.push({ command, payload });
      const response = responses[command];
      if (response === undefined) {
        return Promise.reject(new EvidenceUnavailableError("evidence_unavailable"));
      }
      return Promise.resolve(response);
    },
    subscribeEvidenceNotices(handler): Promise<Unsubscribe> {
      handlers.add(handler);
      return Promise.resolve(() => {
        handlers.delete(handler);
      });
    },
  };
}
