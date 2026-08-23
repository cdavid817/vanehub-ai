import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
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
  ToolExecutionRecord,
} from "../types/session-workspace-evidence";
import { executionRecordActions, hasExecutionRecordActions } from "./execution-record-actions";

const sessionId = evidenceSessionIdSchema.parse("session-a");
const runId = evidenceRunIdSchema.parse("run-1");
const traceId = evidenceTraceIdSchema.parse("trace-1");
const spanId = evidenceSpanIdSchema.parse("span-1");
const seatId = evidenceSeatIdSchema.parse("seat-1");
const coverage: QueryCoverage = { state: "complete", reasonCodes: [], truncated: false };

// Narrowed to one member of the union so the spread needs no assertion: every field these tests
// override is a base field, shared by every record kind.
function record(overrides: Partial<ToolExecutionRecord> = {}): ToolExecutionRecord {
  return {
    id: evidenceRecordIdSchema.parse("tool:call-1"),
    kind: "tool",
    sessionId,
    status: "succeeded",
    fidelity: "native",
    coverage,
    toolName: "read_file",
    source: "native",
    ...overrides,
  };
}

function actionIds(input: ExecutionRecord): string[] {
  return executionRecordActions(input).map((action) => action.id);
}

describe("execution record cross-panel actions", () => {
  it("offers nothing for a record that carries no correlation", () => {
    // A button that lands on an unfiltered panel is worse than a missing button: an unfiltered
    // panel looks exactly like a filter that matched everything.
    expect(actionIds(record())).toEqual([]);
    expect(hasExecutionRecordActions(record())).toBe(false);
  });

  it("opens the span within its trace when both are known", () => {
    const [action] = executionRecordActions(record({ traceId, spanId, runId }));
    expect(action.id).toBe("trace");
    expect(action.target).toEqual({
      tab: "traces",
      focus: "detail",
      scope: { sessionId, traceId, spanId, runId },
    });
  });

  it("does not open a span that has no trace to resolve it against", () => {
    // A span id alone is a filter the trace view cannot join, so it is not availability there.
    // Logs does filter by span on its own, so that destination stays offered.
    expect(actionIds(record({ spanId }))).toEqual(["logs"]);
  });

  it("falls back to the run's waterfall when there is no trace", () => {
    const [action] = executionRecordActions(record({ runId }));
    expect(action.target).toEqual({
      tab: "traces",
      focus: "row",
      scope: { sessionId, runId },
    });
  });

  it("opens logs for any correlation the logs panel can filter by", () => {
    expect(actionIds(record({ seatId }))).toEqual(["logs"]);
    expect(actionIds(record({ runId }))).toEqual(["trace", "logs", "report"]);
  });

  it("never offers Files or Changes from an execution record", () => {
    // A command's working directory is where it ran, not what it changed. Linking the two would
    // send the reader to a directory the command may never have touched.
    const withCwd: ExecutionRecord = {
      id: evidenceRecordIdSchema.parse("command:cmd-1"),
      kind: "command",
      sessionId,
      status: "succeeded",
      fidelity: "native",
      coverage,
      commandId: evidenceCommandIdSchema.parse("cmd-1"),
      runtimeKind: "local-shell",
      outputAvailability: "merged",
      outputTruncated: false,
      cwdDisplay: "src/",
      runId,
    };
    expect(actionIds(withCwd)).not.toContain("files");
    expect(actionIds(withCwd)).not.toContain("changes");
  });

  it("does not offer Shell before the retained registry exists", () => {
    const source = readFileSync("src/session-workspace/execution-record-actions.ts", "utf8");
    // Re-attaching to the shell a command ran in needs Task Group 7's registry. Offering it now
    // would open an unrelated shell or open nothing, and both read as lost work.
    expect(actionIds(record({ runId, traceId, spanId, seatId }))).not.toContain("shell");
    expect(source).toContain("Task Group 7");
  });
});
