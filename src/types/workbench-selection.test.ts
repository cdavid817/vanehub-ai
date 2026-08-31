import { describe, expect, it } from "vitest";
import { isSelectionInScope, workbenchSelectionKey, type WorkbenchSelection } from "./workbench-selection";

describe("isSelectionInScope", () => {
  it("keeps a session-scoped selection only while its session is the one displayed", () => {
    const message: WorkbenchSelection = { kind: "message", sessionId: "s-1", messageId: "m-1" };
    expect(isSelectionInScope(message, { activeSessionId: "s-1" })).toBe(true);
    expect(isSelectionInScope(message, { activeSessionId: "s-2" })).toBe(false);
    expect(isSelectionInScope(message, { activeSessionId: null })).toBe(false);
  });

  it("treats a tool and a change selection the same way as a message selection", () => {
    const tool: WorkbenchSelection = { kind: "tool", sessionId: "s-1", messageId: "m-1", toolCallId: "t-1" };
    const change: WorkbenchSelection = { kind: "change", sessionId: "s-1", changeId: "c-1" };
    expect(isSelectionInScope(tool, { activeSessionId: "s-1" })).toBe(true);
    expect(isSelectionInScope(tool, { activeSessionId: "s-2" })).toBe(false);
    expect(isSelectionInScope(change, { activeSessionId: "s-1" })).toBe(true);
    expect(isSelectionInScope(change, { activeSessionId: "s-2" })).toBe(false);
  });

  it("keeps a session selection itself in scope only for that same session", () => {
    const session: WorkbenchSelection = { kind: "session", sessionId: "s-1" };
    expect(isSelectionInScope(session, { activeSessionId: "s-1" })).toBe(true);
    expect(isSelectionInScope(session, { activeSessionId: "s-2" })).toBe(false);
  });

  it("keeps a project-scoped file selection regardless of which session is active", () => {
    const file: WorkbenchSelection = { kind: "file", projectId: "p-1", pathId: "path-1" };
    expect(isSelectionInScope(file, { activeSessionId: "s-1" })).toBe(true);
    expect(isSelectionInScope(file, { activeSessionId: null })).toBe(true);
  });

  it("goes stale for a session-scoped file selection once the session changes", () => {
    const file: WorkbenchSelection = { kind: "file", sessionId: "s-1", pathId: "path-1" };
    expect(isSelectionInScope(file, { activeSessionId: "s-1" })).toBe(true);
    expect(isSelectionInScope(file, { activeSessionId: "s-2" })).toBe(false);
  });

  it("never goes stale from a session switch for run/loop-iteration/evaluation-result selections", () => {
    const run: WorkbenchSelection = { kind: "run", runId: "r-1" };
    const loopIteration: WorkbenchSelection = { kind: "loop-iteration", loopRunId: "lr-1", iterationId: "i-1" };
    const evaluationResult: WorkbenchSelection = { kind: "evaluation-result", experimentId: "e-1", resultId: "res-1" };
    for (const selection of [run, loopIteration, evaluationResult]) {
      expect(isSelectionInScope(selection, { activeSessionId: "s-1" })).toBe(true);
      expect(isSelectionInScope(selection, { activeSessionId: null })).toBe(true);
    }
  });
});

describe("workbenchSelectionKey", () => {
  it("produces a distinct key per selection kind and identity", () => {
    const selections: WorkbenchSelection[] = [
      { kind: "session", sessionId: "s-1" },
      { kind: "message", sessionId: "s-1", messageId: "m-1" },
      { kind: "tool", sessionId: "s-1", messageId: "m-1", toolCallId: "t-1" },
      { kind: "file", sessionId: "s-1", pathId: "path-1" },
      { kind: "change", sessionId: "s-1", changeId: "c-1" },
      { kind: "run", runId: "r-1" },
      { kind: "loop-iteration", loopRunId: "lr-1", iterationId: "i-1" },
      { kind: "evaluation-result", experimentId: "e-1", resultId: "res-1" },
    ];
    const keys = selections.map(workbenchSelectionKey);
    expect(new Set(keys).size).toBe(selections.length);
  });

  it("keeps the same key for the same selection", () => {
    const a: WorkbenchSelection = { kind: "message", sessionId: "s-1", messageId: "m-1" };
    const b: WorkbenchSelection = { kind: "message", sessionId: "s-1", messageId: "m-1" };
    expect(workbenchSelectionKey(a)).toBe(workbenchSelectionKey(b));
  });
});
