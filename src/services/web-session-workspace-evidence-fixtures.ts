import {
  evidenceCommandIdSchema,
  evidenceRecordIdSchema,
  evidenceRunIdSchema,
  evidenceSeatIdSchema,
  evidenceSessionIdSchema,
  evidenceSpanIdSchema,
  evidenceTraceIdSchema,
} from "../contracts/session-workspace-evidence-ids";
import type {
  ExecutionRecord,
  QueryCoverage,
  SessionRunReport,
  WorkspaceEvidenceSummary,
} from "../types/session-workspace-evidence";

/**
 * A fixed instant. The Web adapter must produce the same bytes on every run, so nothing here reads
 * the wall clock — a fixture that moved with real time would make the conformance suite's
 * assertions depend on when they ran.
 */
export const WEB_EVIDENCE_CLOCK = "2026-01-01T00:00:00.000Z";

/**
 * Every coverage the Web adapter returns carries `simulated`, so a reader cannot mistake fixture
 * output for an observation. The mock claims no SQLite, process, filesystem, Git, SSH, or OTLP
 * effect, and this is where it says so in a form the UI can render.
 */
export function simulatedCoverage(overrides: Partial<QueryCoverage> = {}): QueryCoverage {
  return {
    state: "complete",
    reasonCodes: ["simulated"],
    oldestAvailableAt: WEB_EVIDENCE_CLOCK,
    newestAvailableAt: WEB_EVIDENCE_CLOCK,
    truncated: false,
    ...overrides,
  };
}

const sessionId = evidenceSessionIdSchema.parse("session-1");
const runId = evidenceRunIdSchema.parse("web-run-1");
const traceId = evidenceTraceIdSchema.parse("web-trace-1");

function at(offsetSeconds: number): string {
  return new Date(Date.parse(WEB_EVIDENCE_CLOCK) + offsetSeconds * 1000).toISOString();
}

/**
 * Seeded records covering every kind and a representative spread of status and fidelity, so the
 * conformance suite exercises the discriminated union rather than one happy shape.
 */
export function webExecutionRecords(): ExecutionRecord[] {
  return [
    {
      id: evidenceRecordIdSchema.parse("web-record-1"),
      kind: "command",
      sessionId,
      runId,
      traceId,
      spanId: evidenceSpanIdSchema.parse("web-span-1"),
      seatId: evidenceSeatIdSchema.parse("web-seat-1"),
      startedAt: at(0),
      endedAt: at(12),
      durationMs: 12_400,
      status: "failed",
      fidelity: "native",
      coverage: simulatedCoverage(),
      commandId: evidenceCommandIdSchema.parse("web-command-1"),
      runtimeKind: "local-shell",
      redactedDisplay: "npm test",
      cwdDisplay: "…/vanehub-ai",
      exitCode: 1,
      outputAvailability: "merged",
      outputTruncated: true,
    },
    {
      id: evidenceRecordIdSchema.parse("web-record-2"),
      kind: "tool",
      sessionId,
      runId,
      traceId,
      startedAt: at(20),
      endedAt: at(20),
      durationMs: 31,
      status: "succeeded",
      fidelity: "proxied",
      coverage: simulatedCoverage(),
      toolName: "read_file",
      source: "native",
    },
    {
      id: evidenceRecordIdSchema.parse("web-record-3"),
      kind: "delegation",
      sessionId,
      runId,
      startedAt: at(30),
      status: "running",
      fidelity: "native",
      coverage: simulatedCoverage(),
      attempt: 2,
    },
    {
      id: evidenceRecordIdSchema.parse("web-record-4"),
      kind: "verification",
      sessionId,
      runId,
      startedAt: at(40),
      endedAt: at(51),
      durationMs: 11_000,
      status: "succeeded",
      fidelity: "native",
      coverage: simulatedCoverage(),
      verificationName: "npm run test",
      outcome: "failed",
      passedCount: 138,
      failedCount: 2,
    },
    {
      // Completion-only. The mock keeps the same rule the native runtime does: no start was
      // observed, so `startedAt` is omitted rather than derived from `endedAt`.
      id: evidenceRecordIdSchema.parse("web-record-6"),
      kind: "command",
      sessionId,
      runId,
      endedAt: at(55),
      status: "incomplete",
      fidelity: "proxied",
      coverage: simulatedCoverage({
        state: "partial",
        reasonCodes: ["simulated", "evidence_start_not_observed"],
      }),
      commandId: evidenceCommandIdSchema.parse("web-command-2"),
      runtimeKind: "process",
      outputAvailability: "unavailable",
      outputTruncated: false,
    },
    {
      id: evidenceRecordIdSchema.parse("web-record-5"),
      kind: "legacy",
      sessionId,
      startedAt: at(60),
      status: "incomplete",
      fidelity: "inferred",
      // Legacy activity is a projection of loaded messages, never a complete corpus.
      coverage: simulatedCoverage({ state: "partial", reasonCodes: ["simulated", "coverage_partial"] }),
      label: "shell toolUse",
      source: "message-history",
      messageId: "web-message-9",
    },
  ];
}

export function webEvidenceSummary(): WorkspaceEvidenceSummary {
  return {
    sessionId,
    generatedAt: WEB_EVIDENCE_CLOCK,
    coverage: simulatedCoverage(),
    runState: { status: "running", runId, startedAt: WEB_EVIDENCE_CLOCK },
    changes: { changedFiles: 8, unviewedFiles: 4 },
    executionRecords: { running: 1, failed: 1 },
    shells: { live: 2 },
    logs: { newErrors: 3 },
    traces: { running: 1, failed: 0 },
    verification: { passed: 138, failed: 2 },
    usage: { reportedTokens: 112_000, coverage: "partial" },
  };
}

export function webSessionRunReport(): SessionRunReport {
  const section = { state: "complete" as const, reasonCodes: ["simulated"] };
  return {
    scope: { sessionId, runIds: [runId], seatIds: [], groupBy: "run" },
    generatedAt: WEB_EVIDENCE_CLOCK,
    coverage: {
      overall: "partial",
      sections: {
        overview: section,
        // The mock has no usage accounting behind it, so the section says partial rather than
        // presenting fixture totals as reported figures.
        usage: { state: "partial", reasonCodes: ["simulated", "coverage_partial"] },
        latency: section,
        agents: section,
        tools: section,
        commands: section,
        changes: section,
        verification: section,
        failures: section,
      },
    },
    overview: { runCount: 1, durationMs: 71_000, succeeded: 0, failed: 1, cancelled: 0, retries: 1 },
    usage: {
      responseCount: 3,
      internalPurposeResponseCount: 1,
      coverage: { state: "partial", reasonCodes: ["simulated", "coverage_partial"] },
      costAvailable: false,
    },
    latency: { p50Ms: 31, p95Ms: 12_400, slowestRecordDurationMs: 12_400 },
    agents: [],
    tools: [{ toolName: "read_file", invocations: 1, failures: 0, durationMs: 31 }],
    commands: { total: 1, failed: 1, running: 0, durationMs: 12_400 },
    changes: { changedFiles: 8, unviewedFiles: 4, unresolvedFindings: 2 },
    verification: { passed: 138, failed: 2, skipped: 0 },
    failures: { rows: [{ reasonCode: "command_failed", count: 1 }] },
    evidenceLinks: [{ tab: "terminal-history", scope: { sessionId, runId } }],
    sourceCoverage: simulatedCoverage({ state: "partial", reasonCodes: ["simulated", "coverage_partial"] }),
  };
}
