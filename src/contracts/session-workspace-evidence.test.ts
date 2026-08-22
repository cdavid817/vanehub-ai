import { describe, expect, it } from "vitest";
import {
  parseExecutionEvidenceNotice,
  parseExecutionRecord,
  parseExecutionRecordPage,
  parseSessionRunReport,
  parseWorkspaceEvidenceSummary,
} from "./session-workspace-evidence";
import { evidenceCursorSchema, evidenceSessionIdSchema } from "./session-workspace-evidence-ids";

const coverage = { state: "complete", reasonCodes: [], truncated: false };

function commandRecord(overrides: Record<string, unknown> = {}) {
  return {
    id: "record-1",
    kind: "command",
    sessionId: "session-1",
    startedAt: "2026-08-22T10:00:00.000Z",
    status: "succeeded",
    fidelity: "native",
    coverage,
    commandId: "command-1",
    runtimeKind: "local-shell",
    outputAvailability: "merged",
    outputTruncated: false,
    ...overrides,
  };
}

describe("evidence transport schemas", () => {
  it("produces branded ids only by parsing", () => {
    const sessionId = evidenceSessionIdSchema.parse("session-1");
    expect(sessionId).toBe("session-1");
    expect(() => evidenceSessionIdSchema.parse("")).toThrow();
    expect(() => evidenceSessionIdSchema.parse(42)).toThrow();
  });

  // Decoding a cursor here would let the frontend construct one, which is how offset arithmetic
  // came back last time. It is validated as a non-empty string and nothing more.
  it("treats a cursor as an opaque non-empty string", () => {
    expect(evidenceCursorSchema.parse("v1.eyJzIjoxfQ")).toBe("v1.eyJzIjoxfQ");
    expect(() => evidenceCursorSchema.parse("")).toThrow();
  });

  it("parses each execution record kind", () => {
    expect(parseExecutionRecord(commandRecord()).kind).toBe("command");
    expect(parseExecutionRecord({
      id: "record-2",
      kind: "tool",
      sessionId: "session-1",
      startedAt: "2026-08-22T10:00:00.000Z",
      status: "succeeded",
      fidelity: "proxied",
      coverage,
      toolName: "read_file",
      source: "native",
    }).kind).toBe("tool");
    expect(parseExecutionRecord({
      id: "record-3",
      kind: "legacy",
      sessionId: "session-1",
      startedAt: "2026-08-22T10:00:00.000Z",
      status: "incomplete",
      fidelity: "inferred",
      coverage,
      label: "shell toolUse",
      source: "message-history",
      messageId: "message-9",
    }).fidelity).toBe("inferred");
  });

  // A completion-only record: the runtime saw it finish but never saw it begin.
  it("accepts a terminal record with no start rather than requiring one", () => {
    const withoutStart: Record<string, unknown> = commandRecord({
      status: "incomplete",
      endedAt: "2026-08-22T10:39:02.000Z",
      coverage: {
        state: "partial",
        reasonCodes: ["evidence_start_not_observed"],
        truncated: false,
      },
    });
    delete withoutStart.startedAt;

    const parsed = parseExecutionRecord(withoutStart);

    expect(parsed.startedAt).toBeUndefined();
    // The terminal status is preserved: not observing the start says nothing about the outcome.
    expect(parsed.status).toBe("incomplete");
    expect(parsed.endedAt).toBe("2026-08-22T10:39:02.000Z");
    expect(parsed.coverage.reasonCodes).toContain("evidence_start_not_observed");
  });

  // Nothing may reconstruct a start from a value that is not one.
  it("does not invent a start from endedAt, durationMs, or occurrence time", () => {
    const withoutStart: Record<string, unknown> = commandRecord({
      status: "failed",
      endedAt: "2026-08-22T10:39:02.000Z",
      durationMs: 900,
    });
    delete withoutStart.startedAt;

    const parsed = parseExecutionRecord(withoutStart);

    expect(parsed).not.toHaveProperty("startedAt");
    expect(parsed.durationMs).toBe(900);
  });

  it("still round-trips a record that did observe its start", () => {
    const parsed = parseExecutionRecord(commandRecord({ startedAt: "2026-08-22T10:00:00.000Z" }));
    expect(parsed.startedAt).toBe("2026-08-22T10:00:00.000Z");
  });

  // Rendering an unknown kind as the nearest familiar one would attribute fields that may mean
  // something else, so the row is rejected and the caller reports reduced coverage instead.
  it("fails closed on an unknown record kind", () => {
    expect(() => parseExecutionRecord(commandRecord({ kind: "teleport" }))).toThrow();
  });

  it("rejects a record missing a field its kind requires", () => {
    const withoutCommandId: Record<string, unknown> = commandRecord();
    delete withoutCommandId.commandId;
    expect(() => parseExecutionRecord(withoutCommandId)).toThrow();
  });

  // A backend that adds a field must not break a frontend that has not learned about it yet.
  it("accepts additive unknown fields and drops them", () => {
    const parsed = parseExecutionRecord(commandRecord({ futureField: "later", nested: { a: 1 } }));
    expect(parsed).not.toHaveProperty("futureField");
    expect(parsed.kind).toBe("command");
  });

  it("parses a cursor page and keeps its coverage", () => {
    const page = parseExecutionRecordPage({
      items: [commandRecord()],
      nextCursor: "v1.next",
      coverage: { state: "partial", reasonCodes: ["coverage_partial"], truncated: true, droppedCount: 3 },
    });
    expect(page.items).toHaveLength(1);
    expect(page.nextCursor).toBe("v1.next");
    expect(page.coverage.state).toBe("partial");
    expect(page.coverage.droppedCount).toBe(3);
  });

  it("accepts a page with no continuation token", () => {
    const page = parseExecutionRecordPage({ items: [], coverage });
    expect(page.nextCursor).toBeUndefined();
  });

  it("parses an identifier-only evidence notice", () => {
    const notice = parseExecutionEvidenceNotice({
      kind: "record-appended",
      sequence: 7,
      sessionId: "session-1",
      occurredAt: "2026-08-22T10:00:00.000Z",
      recordId: "record-1",
    });
    expect(notice.sequence).toBe(7);
    expect(notice.recordId).toBe("record-1");
  });

  it("rejects an unknown notice kind", () => {
    expect(() => parseExecutionEvidenceNotice({
      kind: "record-teleported",
      sequence: 1,
      sessionId: "session-1",
      occurredAt: "2026-08-22T10:00:00.000Z",
    })).toThrow();
  });

  it("parses a workspace evidence summary", () => {
    const summary = parseWorkspaceEvidenceSummary({
      sessionId: "session-1",
      generatedAt: "2026-08-22T10:00:00.000Z",
      coverage,
      runState: { status: "running" },
      changes: { changedFiles: 8, unviewedFiles: 4 },
      executionRecords: { running: 1, failed: 2 },
      shells: { live: 2 },
      logs: { newErrors: 3 },
      traces: { running: 1, failed: 0 },
      verification: { passed: 138, failed: 2 },
      usage: { reportedTokens: 112000, coverage: "partial" },
    });
    expect(summary.usage.coverage).toBe("partial");
    expect(summary.shells.live).toBe(2);
  });

  // The change introduces no pricing catalogue, so a cost figure has nothing versioned behind it.
  it("refuses a report that claims monetary cost is available", () => {
    const report = {
      scope: { sessionId: "session-1", runIds: [], seatIds: [], groupBy: "run" },
      generatedAt: "2026-08-22T10:00:00.000Z",
      coverage: {
        overall: "complete",
        sections: Object.fromEntries([
          "overview", "usage", "latency", "agents", "tools", "commands", "changes", "verification", "failures",
        ].map((section) => [section, { state: "complete", reasonCodes: [] }])),
      },
      overview: { runCount: 1, succeeded: 1, failed: 0, cancelled: 0, retries: 0 },
      usage: { responseCount: 3, internalPurposeResponseCount: 1, coverage: { state: "complete", reasonCodes: [] }, costAvailable: false },
      latency: {},
      agents: [],
      tools: [],
      commands: { total: 0, failed: 0, running: 0 },
      changes: { changedFiles: 0, unviewedFiles: 0, unresolvedFindings: 0 },
      verification: { passed: 0, failed: 0, skipped: 0 },
      failures: { rows: [] },
      evidenceLinks: [],
      sourceCoverage: coverage,
    };
    expect(parseSessionRunReport(report).usage.costAvailable).toBe(false);
    expect(() => parseSessionRunReport({
      ...report,
      usage: { ...report.usage, costAvailable: true },
    })).toThrow();
  });
});
