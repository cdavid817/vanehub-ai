import type {
  ExecutionRecord,
  WorkspaceEvidenceTarget,
} from "../types/session-workspace-evidence";

export type ExecutionRecordActionId = "trace" | "logs" | "report" | "files" | "changes";

export interface ExecutionRecordAction {
  id: ExecutionRecordActionId;
  target: WorkspaceEvidenceTarget;
}

/**
 * The cross-panel jumps a record can actually support.
 *
 * An action is offered only when its destination can be built from what the record carries. The
 * alternative — showing every action and letting the destination cope — puts a button on screen
 * that lands on an unfiltered panel, and an unfiltered panel looks exactly like a filter that
 * matched everything. A missing button is a smaller lie than a button that goes nowhere.
 *
 * Shell is deliberately absent. Re-attaching to the shell a command ran in needs the retained
 * registry that Task Group 7 introduces; offering it now would either open an unrelated shell or
 * open nothing, and both read as the console having lost the session's work.
 */
export function executionRecordActions(record: ExecutionRecord): ExecutionRecordAction[] {
  const actions: ExecutionRecordAction[] = [];
  const sessionId = record.sessionId;

  // The trace view resolves a span within its trace, or a run's whole waterfall. A span id with no
  // trace is a filter the destination cannot join, so it does not count as available on its own.
  if (record.traceId !== undefined) {
    actions.push({
      id: "trace",
      target: {
        tab: "traces",
        focus: "detail",
        scope: {
          sessionId,
          traceId: record.traceId,
          ...(record.spanId === undefined ? {} : { spanId: record.spanId }),
          ...(record.runId === undefined ? {} : { runId: record.runId }),
        },
      },
    });
  } else if (record.runId !== undefined) {
    actions.push({
      id: "trace",
      target: { tab: "traces", focus: "row", scope: { sessionId, runId: record.runId } },
    });
  }

  // Logs filter by run, trace, span, seat, or operation. Without one of those the jump would open
  // the whole session's log, which is not what "the logs for this record" means.
  const logScope = {
    sessionId,
    ...(record.runId === undefined ? {} : { runId: record.runId }),
    ...(record.traceId === undefined ? {} : { traceId: record.traceId }),
    ...(record.spanId === undefined ? {} : { spanId: record.spanId }),
    ...(record.operationId === undefined ? {} : { operationId: record.operationId }),
    ...(record.seatId === undefined ? {} : { seatId: record.seatId }),
  };
  if (Object.keys(logScope).length > 1) {
    actions.push({ id: "logs", target: { tab: "logs", focus: "filter", scope: logScope } });
  }

  if (record.runId !== undefined) {
    actions.push({
      id: "report",
      target: { tab: "report", focus: "row", scope: { sessionId, runId: record.runId } },
    });
  }

  // Files and Changes filter by path, and no execution record carries one: a command's working
  // directory is where it ran, not what it changed. The mutation evidence that does carry a path
  // is a different record type, and inventing the link from a `cwd` would send the reader to a
  // directory the command may never have touched.
  return actions;
}

/** Whether a record can offer any cross-panel jump at all. */
export function hasExecutionRecordActions(record: ExecutionRecord): boolean {
  return executionRecordActions(record).length > 0;
}
