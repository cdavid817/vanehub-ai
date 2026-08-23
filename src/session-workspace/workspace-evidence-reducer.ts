import type {
  EvidenceSessionId,
  WorkspaceEvidenceFocus,
  WorkspaceEvidenceScope,
  WorkspaceEvidenceTabId,
  WorkspaceEvidenceTarget,
} from "../types/session-workspace-evidence";
import type { SessionTabId } from "./session-tab-bar";
import {
  EVIDENCE_SCOPE_FIELDS,
  unsupportedScopeFields,
  withDependentScopeFields,
  type EvidenceScopeField,
  type WorkspaceEvidenceCorrelation,
} from "./workspace-evidence-navigation";

/**
 * The whole cross-panel selection, as one serializable value.
 *
 * Nothing here is a query client, a DOM node, a promise, or a callback. That is what makes a tab
 * switch survivable: the state can be compared and restored, and a panel that reads it cannot end
 * up holding a handle to a panel that has since unmounted.
 */
export interface WorkspaceEvidenceState {
  /** Whose evidence this is. Null when no session is selected; a scope has no meaning without it. */
  sessionId: EvidenceSessionId | null;
  activeTab: SessionTabId;
  correlation: WorkspaceEvidenceCorrelation;
  focus: WorkspaceEvidenceFocus | null;
  /**
   * Bumped by every accepted `navigate`. Choosing the same target twice produces the same tab and
   * the same correlation, so without a revision the destination has nothing to react to and the
   * second "open this span" would not re-focus.
   */
  navigationRevision: number;
  /** Fields the current tab was handed and will not apply. Rendered, never silently dropped. */
  unsupportedFields: readonly EvidenceScopeField[];
}

export type WorkspaceEvidenceAction =
  | { type: "select-session"; sessionId: EvidenceSessionId | null }
  | { type: "activate-tab"; tab: SessionTabId }
  | { type: "navigate"; target: WorkspaceEvidenceTarget }
  | { type: "patch-scope"; patch: WorkspaceEvidenceCorrelation }
  /** An empty list is Clear All; a named list also clears whatever those fields own. */
  | { type: "clear-scope"; fields: readonly EvidenceScopeField[] }
  | { type: "validate-seats"; seatIds: readonly string[] };

export function initialWorkspaceEvidenceState(
  sessionId: EvidenceSessionId | null,
): WorkspaceEvidenceState {
  return {
    sessionId,
    activeTab: "chat",
    correlation: {},
    focus: null,
    navigationRevision: 0,
    unsupportedFields: [],
  };
}

/**
 * The evidence destination each workspace tab maps to, or null for a tab that reads no evidence.
 *
 * Written out rather than derived from a key test so the compiler checks it: a tab added to the
 * bar without a decision here is an error, not a tab that silently consumes nothing.
 */
const TAB_DESTINATIONS: Record<SessionTabId, WorkspaceEvidenceTabId | null> = {
  chat: null,
  changes: "changes",
  documents: "documents",
  files: "files",
  terminal: "terminal",
  shell: "shell",
  logs: "logs",
  traces: "traces",
  report: "report",
};

export function evidenceTabOf(tab: SessionTabId): WorkspaceEvidenceTabId | null {
  return TAB_DESTINATIONS[tab];
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
  tab: SessionTabId,
  correlation: WorkspaceEvidenceCorrelation,
): readonly EvidenceScopeField[] {
  const destination = evidenceTabOf(tab);
  return destination === null ? [] : unsupportedScopeFields(destination, correlation);
}

/** Rebuilds the derived half of the state, keeping the previous array when nothing moved. */
function settle(
  previous: WorkspaceEvidenceState,
  next: Omit<WorkspaceEvidenceState, "unsupportedFields">,
): WorkspaceEvidenceState {
  const unsupported = unsupportedFor(next.activeTab, next.correlation);
  const unchanged =
    unsupported.length === previous.unsupportedFields.length &&
    unsupported.every((field, index) => previous.unsupportedFields[index] === field);
  return { ...next, unsupportedFields: unchanged ? previous.unsupportedFields : unsupported };
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

    case "activate-tab":
      if (action.tab === state.activeTab) return state;
      // Activating a tab is not navigating: the correlation carries over untouched, so a user who
      // filtered Logs to one run and glanced at Traces returns to the same filter.
      return settle(state, { ...state, activeTab: action.tab });

    case "navigate": {
      // Fail closed on a scope owned by another session. A partial application would be worse than
      // refusing outright: the tab would change while the filter did not.
      if (state.sessionId === null || action.target.scope.sessionId !== state.sessionId) {
        return state;
      }
      // Replace, never merge. Merging would leave the previous destination's filters in place, so
      // "show me this command" would quietly mean "this command, still inside yesterday's trace".
      return settle(state, {
        sessionId: state.sessionId,
        activeTab: action.target.tab,
        correlation: correlationOf(action.target.scope),
        focus: action.target.focus ?? null,
        navigationRevision: state.navigationRevision + 1,
      });
    }

    case "patch-scope": {
      // In-panel filtering: merges, does not move tabs, and does not re-focus. The user is
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
