import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useReducer,
  type ReactNode,
} from "react";
import type {
  EvidenceSessionId,
  WorkspaceEvidenceFocus,
  WorkspaceEvidenceScope,
  WorkspaceEvidenceTarget,
} from "../types/session-workspace-evidence";
import type { SessionTabId } from "./session-tab-bar";
import type {
  EvidenceScopeField,
  WorkspaceEvidenceCorrelation,
} from "./workspace-evidence-navigation";
import {
  initialWorkspaceEvidenceState,
  workspaceEvidenceReducer,
} from "./workspace-evidence-reducer";

/** The one cross-panel selection API. Panels read the scope; nothing else may write it. */
export interface WorkspaceEvidenceNavigation {
  activeTab: SessionTabId;
  /** Null when no session is selected: a correlation without an owner is not a scope. */
  scope: WorkspaceEvidenceScope | null;
  correlation: WorkspaceEvidenceCorrelation;
  focus: WorkspaceEvidenceFocus | null;
  navigationRevision: number;
  unsupportedFields: readonly EvidenceScopeField[];
  activateTab: (tab: SessionTabId) => void;
  /** Moves tab and scope together. Replaces the correlation rather than merging into it. */
  navigate: (target: WorkspaceEvidenceTarget) => void;
  /** In-panel filtering: merges into the current correlation without moving tabs. */
  patchScope: (patch: WorkspaceEvidenceCorrelation) => void;
  /** Named fields plus whatever they own; no argument means Clear All. */
  clearScope: (fields?: readonly EvidenceScopeField[]) => void;
}

const WorkspaceEvidenceScopeContext = createContext<WorkspaceEvidenceNavigation | null>(null);

export function WorkspaceEvidenceScopeProvider({
  children,
  seatIds,
  sessionId,
}: {
  children: ReactNode;
  /** Seats currently in the session. A scope naming one that has left is dropped. */
  seatIds: readonly string[];
  sessionId: EvidenceSessionId | null;
}) {
  const [stored, dispatch] = useReducer(
    workspaceEvidenceReducer,
    sessionId,
    initialWorkspaceEvidenceState,
  );

  // Reset during render, not in an effect, for two reasons that both bite.
  //
  // A child's effects run before its parent's, so an effect here would land after every panel had
  // already rendered and after their own effects had run — the first frame of a new session would
  // build query keys from the previous session's run and trace, and a panel effect asking for a
  // tab would be undone by a reset that arrives later.
  //
  // React re-runs this component immediately on a render-phase dispatch and discards the child
  // render, so the children below only ever see the settled state.
  if (stored.sessionId !== sessionId) dispatch({ type: "select-session", sessionId });
  const state = stored.sessionId === sessionId ? stored : initialWorkspaceEvidenceState(sessionId);

  // Re-validated after the reset above, so a seat filter cleared by a session switch is never
  // re-examined against the new session's roster. Re-running on an unmemoized array is harmless:
  // the reducer answers with the same state when the seat still exists, so there is no loop.
  useEffect(() => {
    dispatch({ type: "validate-seats", seatIds });
  }, [seatIds]);

  const activateTab = useCallback((tab: SessionTabId) => dispatch({ type: "activate-tab", tab }), []);
  const navigate = useCallback(
    (target: WorkspaceEvidenceTarget) => dispatch({ type: "navigate", target }),
    [],
  );
  const patchScope = useCallback(
    (patch: WorkspaceEvidenceCorrelation) => dispatch({ type: "patch-scope", patch }),
    [],
  );
  const clearScope = useCallback(
    (fields?: readonly EvidenceScopeField[]) =>
      dispatch({ type: "clear-scope", fields: fields ?? [] }),
    [],
  );

  const scope = useMemo<WorkspaceEvidenceScope | null>(
    () => (state.sessionId === null ? null : { ...state.correlation, sessionId: state.sessionId }),
    [state.correlation, state.sessionId],
  );

  const value = useMemo<WorkspaceEvidenceNavigation>(
    () => ({
      activeTab: state.activeTab,
      scope,
      correlation: state.correlation,
      focus: state.focus,
      navigationRevision: state.navigationRevision,
      unsupportedFields: state.unsupportedFields,
      activateTab,
      navigate,
      patchScope,
      clearScope,
    }),
    [activateTab, clearScope, navigate, patchScope, scope, state],
  );

  return (
    <WorkspaceEvidenceScopeContext.Provider value={value}>
      {children}
    </WorkspaceEvidenceScopeContext.Provider>
  );
}

/**
 * Throws outside the provider rather than answering with an empty scope. A panel that silently got
 * "no filters" would render the whole session and look correct, which is the failure this contract
 * exists to make impossible.
 */
export function useWorkspaceEvidenceScope(): WorkspaceEvidenceNavigation {
  const value = useContext(WorkspaceEvidenceScopeContext);
  if (value === null) {
    throw new Error("useWorkspaceEvidenceScope requires a WorkspaceEvidenceScopeProvider ancestor.");
  }
  return value;
}
