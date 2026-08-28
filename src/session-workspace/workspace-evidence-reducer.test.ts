import { describe, expect, it } from "vitest";
import {
  evidenceCommandIdSchema,
  evidenceOperationIdSchema,
  evidenceRunIdSchema,
  evidenceSeatIdSchema,
  evidenceSessionIdSchema,
  evidenceSpanIdSchema,
  evidenceTraceIdSchema,
} from "../contracts/session-workspace-evidence-ids";
import {
  evidenceTabOf,
  initialWorkspaceEvidenceState,
  workspaceEvidenceReducer,
  type WorkspaceEvidenceAction,
  type WorkspaceEvidenceState,
} from "./workspace-evidence-reducer";

const sessionId = evidenceSessionIdSchema.parse("session-a");
const otherSessionId = evidenceSessionIdSchema.parse("session-b");
const seatId = evidenceSeatIdSchema.parse("seat-1");
const runId = evidenceRunIdSchema.parse("run-1");
const traceId = evidenceTraceIdSchema.parse("trace-1");
const spanId = evidenceSpanIdSchema.parse("span-1");
const operationId = evidenceOperationIdSchema.parse("operation-1");
const commandId = evidenceCommandIdSchema.parse("command-1");

function reduce(
  state: WorkspaceEvidenceState,
  ...actions: WorkspaceEvidenceAction[]
): WorkspaceEvidenceState {
  return actions.reduce(workspaceEvidenceReducer, state);
}

const opened = initialWorkspaceEvidenceState(sessionId);

describe("workspace evidence reducer", () => {
  it("starts on the conversation with no filters", () => {
    expect(opened).toEqual({
      sessionId,
      activeTab: "chat",
      correlation: {},
      focus: null,
      navigationRevision: 0,
      unsupportedFields: [],
    });
  });

  it("stays serializable, so a tab switch cannot strand a handle", () => {
    const state = reduce(opened, {
      type: "navigate",
      target: { tab: "traces", scope: { sessionId, runId, traceId }, focus: "row" },
    });
    expect(JSON.parse(JSON.stringify(state))).toEqual(state);
  });

  it("moves the tab and the scope in one step", () => {
    const state = reduce(opened, {
      type: "navigate",
      target: { tab: "logs", scope: { sessionId, runId, spanId }, focus: "row" },
    });
    expect(state.activeTab).toBe("logs");
    expect(state.correlation).toEqual({ runId, spanId });
    expect(state.focus).toBe("row");
    expect(state.navigationRevision).toBe(1);
  });

  it("replaces the correlation rather than merging into it", () => {
    const state = reduce(
      opened,
      { type: "navigate", target: { tab: "logs", scope: { sessionId, runId, traceId, spanId } } },
      { type: "navigate", target: { tab: "terminal", scope: { sessionId, commandId } } },
    );
    // A merge would leave the previous trace in place, so "show me this command" would silently
    // mean "this command, still inside the trace I was reading a moment ago".
    expect(state.correlation).toEqual({ commandId });
  });

  it("re-focuses when the same target is chosen twice", () => {
    const target = {
      tab: "traces" as const,
      scope: { sessionId, spanId },
      focus: "detail" as const,
    };
    const first = reduce(opened, { type: "navigate", target });
    const second = reduce(first, { type: "navigate", target });
    expect(second.correlation).toEqual(first.correlation);
    expect(second.navigationRevision).toBe(first.navigationRevision + 1);
  });

  it("refuses a target that carries another session's scope", () => {
    const state = reduce(opened, {
      type: "navigate",
      target: { tab: "logs", scope: { sessionId: otherSessionId, runId } },
    });
    // Fail closed: changing the tab while refusing the filter would be worse than refusing both.
    expect(state).toBe(opened);
  });

  it("records the fields the destination will not apply", () => {
    const state = reduce(opened, {
      type: "navigate",
      target: { tab: "files", scope: { sessionId, commandId, relativePath: "src/main.rs" } },
    });
    expect(state.unsupportedFields).toEqual(["commandId"]);
  });

  it("keeps filters when a tab is activated rather than navigated to", () => {
    const filtered = reduce(opened, {
      type: "navigate",
      target: { tab: "logs", scope: { sessionId, runId } },
    });
    const switched = reduce(filtered, { type: "activate-tab", tab: "traces" });
    expect(switched.correlation).toEqual({ runId });
    expect(switched.navigationRevision).toBe(filtered.navigationRevision);
  });

  it("merges an in-panel filter without moving the tab or re-focusing", () => {
    const filtered = reduce(opened, {
      type: "navigate",
      target: { tab: "logs", scope: { sessionId, runId } },
    });
    const patched = reduce(filtered, { type: "patch-scope", patch: { seatId } });
    expect(patched.activeTab).toBe("logs");
    expect(patched.correlation).toEqual({ runId, seatId });
    expect(patched.navigationRevision).toBe(filtered.navigationRevision);
  });

  it("treats a blank value as no filter at all", () => {
    const patched = reduce(opened, { type: "patch-scope", patch: { relativePath: "" } });
    // Two spellings of "no filter" would otherwise produce two cache entries for one answer.
    expect(patched.correlation).toEqual({});
    expect(patched).toBe(opened);
  });

  it("clears a field together with everything that field owns", () => {
    const filtered = reduce(opened, {
      type: "navigate",
      target: { tab: "logs", scope: { sessionId, seatId, runId, traceId, spanId, operationId } },
    });
    const cleared = reduce(filtered, { type: "clear-scope", fields: ["runId"] });
    expect(cleared.correlation).toEqual({ seatId });
  });

  it("clears everything when no field is named", () => {
    const filtered = reduce(opened, {
      type: "navigate",
      target: { tab: "logs", scope: { sessionId, seatId, runId } },
    });
    expect(reduce(filtered, { type: "clear-scope", fields: [] }).correlation).toEqual({});
  });

  it("drops a seat filter naming a seat that has left", () => {
    const filtered = reduce(opened, {
      type: "navigate",
      target: { tab: "logs", scope: { sessionId, seatId, runId } },
    });
    const validated = reduce(filtered, { type: "validate-seats", seatIds: ["seat-2"] });
    // Keeping it would render an empty panel, which reads as "this seat did nothing".
    expect(validated.correlation).toEqual({ runId });
    expect(reduce(filtered, { type: "validate-seats", seatIds: ["seat-1"] })).toBe(filtered);
  });

  it("drops the previous session's filters when the session changes", () => {
    const filtered = reduce(opened, {
      type: "navigate",
      target: { tab: "logs", scope: { sessionId, seatId, runId, operationId } },
    });
    const switched = reduce(filtered, { type: "select-session", sessionId: otherSessionId });
    expect(switched).toEqual(initialWorkspaceEvidenceState(otherSessionId));
  });

  it("keeps object identity when nothing changed", () => {
    expect(reduce(opened, { type: "select-session", sessionId })).toBe(opened);
    expect(reduce(opened, { type: "activate-tab", tab: "chat" })).toBe(opened);
    expect(reduce(opened, { type: "clear-scope", fields: ["runId"] })).toBe(opened);
  });

  it("refuses to navigate before a session is selected", () => {
    const empty = initialWorkspaceEvidenceState(null);
    expect(reduce(empty, { type: "navigate", target: { tab: "logs", scope: { sessionId } } })).toBe(
      empty,
    );
  });

  it("maps only the tabs that read evidence to a destination", () => {
    expect(evidenceTabOf("chat")).toBeNull();
    expect(evidenceTabOf("logs")).toBe("logs");
    expect(evidenceTabOf("report")).toBe("report");
  });
});
