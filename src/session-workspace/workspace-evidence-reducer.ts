import type {
  EvidenceSessionId,
  WorkspaceEvidenceFocus,
  WorkspaceEvidenceScope,
  WorkspaceEvidenceTabId,
  WorkspaceEvidenceTarget,
} from "../types/session-workspace-evidence";
import {
  isRuntimeSurface,
  type SessionPrimarySurfaceId,
  type SessionRuntimeSurfaceId,
  type SessionSurfaceId,
} from "./session-surface-registry";
import {
  EVIDENCE_SCOPE_FIELDS,
  unsupportedScopeFields,
  withDependentScopeFields,
  type EvidenceScopeField,
  type WorkspaceEvidenceCorrelation,
} from "./workspace-evidence-navigation";

const DEFAULT_RUNTIME_SURFACE: SessionRuntimeSurfaceId = "terminal-history";

/**
 * The whole cross-panel selection, as one serializable value.
 *
 * Nothing here is a query client, a DOM node, a promise, or a callback. That is what makes a tab
 * switch survivable: the state can be compared and restored, and a panel that reads it cannot end
 * up holding a handle to a panel that has since unmounted.
 *
 * `activePrimarySurface` and the Runtime Panel's own fields are separate on purpose — design.md
 * Decision 7's own scenario requires opening a runtime surface to *preserve* the active primary
 * surface, not replace it. A single shared "active tab" field, as the pre-Runtime-Panel nine-tab
 * model used, cannot represent "Files is showing, and Shell is also open" at the same time.
 */
export interface WorkspaceEvidenceState {
  /** Whose evidence this is. Null when no session is selected; a scope has no meaning without it. */
  sessionId: EvidenceSessionId | null;
  activePrimarySurface: SessionPrimarySurfaceId;
  runtimePanelOpen: boolean;
  activeRuntimeSurface: SessionRuntimeSurfaceId;
  correlation: WorkspaceEvidenceCorrelation;
  focus: WorkspaceEvidenceFocus | null;
  /**
   * Bumped by every accepted `navigate`. Choosing the same target twice produces the same surface
   * and the same correlation, so without a revision the destination has nothing to react to and
   * the second "open this span" would not re-focus.
   */
  navigationRevision: number;
  /** Fields the current surface was handed and will not apply. Rendered, never silently dropped. */
  unsupportedFields: readonly EvidenceScopeField[];
}

export type WorkspaceEvidenceAction =
  | { type: "select-session"; sessionId: EvidenceSessionId | null }
  | { type: "activate-surface"; id: SessionSurfaceId }
  | { type: "open-runtime-panel" }
  | { type: "close-runtime-panel" }
  | { type: "navigate"; target: WorkspaceEvidenceTarget }
  | { type: "patch-scope"; patch: WorkspaceEvidenceCorrelation }
  /** An empty list is Clear All; a named list also clears whatever those fields own. */
  | { type: "clear-scope"; fields: readonly EvidenceScopeField[] }
  | { type: "validate-seats"; seatIds: readonly string[] };

export function initialWorkspaceEvidenceState(
  sessionId: EvidenceSessionId | null,
  /** The persisted "preferred Runtime Panel tab" — only meaningful for the very first mount. */
  activeRuntimeSurface: SessionRuntimeSurfaceId = DEFAULT_RUNTIME_SURFACE,
): WorkspaceEvidenceState {
  return {
    sessionId,
    activePrimarySurface: "work",
    runtimePanelOpen: false,
    activeRuntimeSurface,
    correlation: {},
    focus: null,
    navigationRevision: 0,
    unsupportedFields: [],
  };
}

/**
 * The evidence destination each workspace surface maps to, or null for a surface that reads no
 * evidence.
 *
 * Written out rather than derived from a key test so the compiler checks it: a surface added to
 * either region without a decision here is an error, not a surface that silently consumes
 * nothing.
 */
const SURFACE_DESTINATIONS: Record<SessionSurfaceId, WorkspaceEvidenceTabId | null> = {
  work: null,
  changes: "changes",
  files: "files",
  report: "report",
  "terminal-history": "terminal-history",
  shell: "shell",
  logs: "logs",
  traces: "traces",
};

export function evidenceTabOf(id: SessionSurfaceId): WorkspaceEvidenceTabId | null {
  return SURFACE_DESTINATIONS[id];
}

/**
 * Drops absent and blank values so two spellings of "no filter" cannot produce two query keys. An
 * empty string is the shape a cleared input arrives in, and it is not a filter.
 */
function normalizeCorrelation(
  correlation: WorkspaceEvidenceCorrelation,
): WorkspaceEvidenceCorrelation {
  const next = { ...correlation };
  for (const field of EVIDENCE_SCOPE_FIELDS) {
    const value = next[field];
    if (value === undefined || value.length === 0) delete next[field];
  }
  return next;
}

/**
 * The correlation half of a scope, built field by field.
 *
 * Spreading the scope would be shorter and wrong: the spread carries `sessionId` at runtime while
 * the type says it does not, and the two would only disagree where it matters — inside a query
 * key, where a stray owner field silently splits the cache. `satisfies` makes a field added to the
 * scope a compile error here rather than a field that quietly never travels.
 */
function correlationOf(scope: WorkspaceEvidenceScope): WorkspaceEvidenceCorrelation {
  return normalizeCorrelation({
    seatId: scope.seatId,
    runId: scope.runId,
    traceId: scope.traceId,
    spanId: scope.spanId,
    operationId: scope.operationId,
    commandId: scope.commandId,
    relativePath: scope.relativePath,
    hunkFingerprint: scope.hunkFingerprint,
    occurredAt: scope.occurredAt,
  } satisfies Record<EvidenceScopeField, string | undefined>);
}

function dropFields(
  correlation: WorkspaceEvidenceCorrelation,
  fields: ReadonlySet<EvidenceScopeField>,
): WorkspaceEvidenceCorrelation {
  const next = { ...correlation };
  for (const field of fields) delete next[field];
  return next;
}

function sameCorrelation(
  left: WorkspaceEvidenceCorrelation,
  right: WorkspaceEvidenceCorrelation,
): boolean {
  return EVIDENCE_SCOPE_FIELDS.every((field) => left[field] === right[field]);
}

function unsupportedFor(
  id: SessionSurfaceId,
  correlation: WorkspaceEvidenceCorrelation,
): readonly EvidenceScopeField[] {
  const destination = evidenceTabOf(id);
  return destination === null ? [] : unsupportedScopeFields(destination, correlation);
}

/** Which surface `unsupportedFields` should be computed against: whichever one is "active" now. */
function focusedSurface(state: {
  activePrimarySurface: SessionPrimarySurfaceId;
  activeRuntimeSurface: SessionRuntimeSurfaceId;
  runtimePanelOpen: boolean;
}): SessionSurfaceId {
  return state.runtimePanelOpen ? state.activeRuntimeSurface : state.activePrimarySurface;
}

/** Rebuilds the derived half of the state, keeping the previous array when nothing moved. */
function settle(
  previous: WorkspaceEvidenceState,
  next: Omit<WorkspaceEvidenceState, "unsupportedFields">,
): WorkspaceEvidenceState {
  const unsupported = unsupportedFor(focusedSurface(next), next.correlation);
  const unchanged =
    unsupported.length === previous.unsupportedFields.length &&
    unsupported.every((field, index) => previous.unsupportedFields[index] === field);
  return { ...next, unsupportedFields: unchanged ? previous.unsupportedFields : unsupported };
}

/** Activating a surface is not navigating: the correlation carries over untouched. */
function withActiveSurface(
  state: WorkspaceEvidenceState,
  id: SessionSurfaceId,
): WorkspaceEvidenceState {
  return isRuntimeSurface(id)
    ? settle(state, { ...state, activeRuntimeSurface: id, runtimePanelOpen: true })
    : settle(state, { ...state, activePrimarySurface: id });
}

export function workspaceEvidenceReducer(
  state: WorkspaceEvidenceState,
  action: WorkspaceEvidenceAction,
): WorkspaceEvidenceState {
  switch (action.type) {
    case "select-session":
      // Same session, same object identity, so a re-render caused by something else does not read
      // as a session switch to a memoized consumer.
      if (action.sessionId === state.sessionId) return state;
      return initialWorkspaceEvidenceState(action.sessionId);

    case "activate-surface":
      if (
        (isRuntimeSurface(action.id) && action.id === state.activeRuntimeSurface && state.runtimePanelOpen)
        || (!isRuntimeSurface(action.id) && action.id === state.activePrimarySurface)
      ) {
        return state;
      }
      return withActiveSurface(state, action.id);

    // Closing hides the panel; it does not touch which runtime surface reopens to. A running
    // Shell or a live Terminal History subscription follows its own retention policy, not this
    // toggle — RuntimePanel keeps a closed tab's mount exactly as design.md Decision 7 requires.
    case "close-runtime-panel":
      if (!state.runtimePanelOpen) return state;
      return settle(state, { ...state, runtimePanelOpen: false });

    case "open-runtime-panel":
      if (state.runtimePanelOpen) return state;
      return settle(state, { ...state, runtimePanelOpen: true });

    case "navigate": {
      // Fail closed on a scope owned by another session. A partial application would be worse than
      // refusing outright: the surface would change while the filter did not.
      if (state.sessionId === null || action.target.scope.sessionId !== state.sessionId) {
        return state;
      }
      // Replace, never merge. Merging would leave the previous destination's filters in place, so
      // "show me this command" would quietly mean "this command, still inside yesterday's trace".
      const id = action.target.tab;
      const base = {
        ...state,
        correlation: correlationOf(action.target.scope),
        focus: action.target.focus ?? null,
        navigationRevision: state.navigationRevision + 1,
      };
      return settle(
        state,
        isRuntimeSurface(id)
          ? { ...base, activeRuntimeSurface: id, runtimePanelOpen: true }
          : { ...base, activePrimarySurface: id },
      );
    }

    case "patch-scope": {
      // In-panel filtering: merges, does not move surfaces, and does not re-focus. The user is
      // adjusting what they are already looking at.
      const merged = normalizeCorrelation({ ...state.correlation, ...action.patch });
      if (sameCorrelation(merged, state.correlation)) return state;
      return settle(state, { ...state, correlation: merged });
    }

    case "clear-scope": {
      const cleared =
        action.fields.length === 0
          ? {}
          : dropFields(state.correlation, withDependentScopeFields(action.fields));
      if (sameCorrelation(cleared, state.correlation)) return state;
      return settle(state, { ...state, correlation: cleared });
    }

    case "validate-seats": {
      const seatId = state.correlation.seatId;
      // A seat that has left is not a filter any query can honour, and keeping it would render an
      // empty panel that reads as "this seat did nothing".
      if (seatId === undefined || action.seatIds.includes(seatId)) return state;
      return settle(state, {
        ...state,
        correlation: dropFields(state.correlation, new Set<EvidenceScopeField>(["seatId"])),
      });
    }
  }
}
