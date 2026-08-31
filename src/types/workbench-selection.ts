/**
 * design.md Decision 8: what the Inspector shows detail for, as one closed union rather than a
 * bag of optional ids a consumer would have to keep consistent by hand.
 *
 * `pathId` (not a filesystem path) for `file`/`change`: an arbitrary path string used as a
 * cross-component command is exactly what this contract exists to rule out — the owning service
 * validates a relative path into a stable id, and only that id crosses this boundary.
 */
export type WorkbenchSelection =
  | { kind: "session"; sessionId: string }
  | { kind: "message"; sessionId: string; messageId: string }
  | { kind: "tool"; sessionId: string; messageId: string; toolCallId: string }
  | { kind: "file"; sessionId?: string; projectId?: string; pathId: string }
  | { kind: "change"; sessionId: string; changeId: string; pathId?: string }
  | { kind: "run"; runId: string }
  | { kind: "loop-iteration"; loopRunId: string; iterationId: string }
  | { kind: "evaluation-result"; experimentId: string; resultId: string };

export type WorkbenchSelectionKind = WorkbenchSelection["kind"];

/**
 * What a selection is checked against before it's trusted enough to query a provider — the
 * session currently displayed, if any. Selections that don't carry a `sessionId` at all
 * (`run`/`loop-iteration`/`evaluation-result`) have no session scope to violate.
 */
export interface WorkbenchSelectionScope {
  activeSessionId: string | null;
}

/**
 * Whether a selection still belongs to what's on screen — the cheap, synchronous half of task
 * 9.2's "validate every selection against its owning route." A selection that fails this check
 * is stale by definition and must not reach a provider at all, let alone render its last-known
 * detail as if it were current; the *other* half (does the object still exist, is it permitted)
 * is inherently a provider-level, asynchronous concern and belongs in that provider's own
 * `AsyncViewState.error` — this function cannot answer it without making a request.
 */
export function isSelectionInScope(
  selection: WorkbenchSelection,
  scope: WorkbenchSelectionScope,
): boolean {
  switch (selection.kind) {
    case "session":
      return selection.sessionId === scope.activeSessionId;
    case "message":
    case "tool":
    case "change":
      return selection.sessionId === scope.activeSessionId;
    case "file":
      // A file selection may be project-scoped rather than session-scoped (design.md's own
      // `sessionId?`) — only a *session-scoped* file selection can go stale this way.
      return selection.sessionId === undefined || selection.sessionId === scope.activeSessionId;
    case "run":
    case "loop-iteration":
    case "evaluation-result":
      // No session scope to violate — these belong to Runs/Loop/Evaluation destinations, not a
      // Session, so a session switch elsewhere in the workbench cannot make them stale.
      return true;
  }
}

/** A short, stable-enough identity string for deduping/keying — never shown to the user as-is. */
export function workbenchSelectionKey(selection: WorkbenchSelection): string {
  switch (selection.kind) {
    case "session":
      return `session:${selection.sessionId}`;
    case "message":
      return `message:${selection.sessionId}:${selection.messageId}`;
    case "tool":
      return `tool:${selection.sessionId}:${selection.messageId}:${selection.toolCallId}`;
    case "file":
      return `file:${selection.sessionId ?? ""}:${selection.projectId ?? ""}:${selection.pathId}`;
    case "change":
      return `change:${selection.sessionId}:${selection.changeId}:${selection.pathId ?? ""}`;
    case "run":
      return `run:${selection.runId}`;
    case "loop-iteration":
      return `loop-iteration:${selection.loopRunId}:${selection.iterationId}`;
    case "evaluation-result":
      return `evaluation-result:${selection.experimentId}:${selection.resultId}`;
  }
}
