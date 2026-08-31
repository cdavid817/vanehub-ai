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
import type {
  SessionPrimarySurfaceId,
  SessionRuntimeSurfaceId,
  SessionSurfaceId,
} from "./session-surface-registry";
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
  activePrimarySurface: SessionPrimarySurfaceId;
  runtimePanelOpen: boolean;
  activeRuntimeSurface: SessionRuntimeSurfaceId;
  /** Null when no session is selected: a correlation without an owner is not a scope. */
  scope: WorkspaceEvidenceScope | null;
  correlation: WorkspaceEvidenceCorrelation;
  focus: WorkspaceEvidenceFocus | null;
  navigationRevision: number;
  unsupportedFields: readonly EvidenceScopeField[];
  /** Activates a primary surface, or opens the Runtime Panel to a runtime surface. */
  activateSurface: (id: SessionSurfaceId) => void;
  openRuntimePanel: () => void;
  /** Hides the Runtime Panel without touching what it holds — see design.md Decision 7. */
  closeRuntimePanel: () => void;
  /** Moves the active surface and scope together. Replaces the correlation rather than merging. */
  navigate: (target: WorkspaceEvidenceTarget) => void;
  /** In-panel filtering: merges into the current correlation without moving surfaces. */
  patchScope: (patch: WorkspaceEvidenceCorrelation) => void;
  /** Named fields plus whatever they own; no argument means Clear All. */
  clearScope: (fields?: readonly EvidenceScopeField[]) => void;
}

const WorkspaceEvidenceScopeContext = createContext<WorkspaceEvidenceNavigation | null>(null);

/**
 * The state half of the provider below, without the JSX wrapping.
 *
 * Exported so a caller that needs the same value to reach two separate subtrees — `SessionTabs`'
 * primary content and the Runtime Panel's content, which `DestinationLayoutBody` renders in two
 * different parts of its own tree, not as siblings either side can wrap together — can compute it
 * once and hand it to two `WorkspaceEvidenceScopeValueProvider`s instead of two independent
 * reducers that would silently drift apart.
 */
export function useWorkspaceEvidenceScopeValue({
  initialRuntimeSurface,
  seatIds,
  sessionId,
}: {
  /**
   * The persisted "preferred Runtime Panel tab" — read once, for this component's very first
   * mount only. A session switch resets to the registry default regardless, the same way it
   * always has: this is a workbench-level preference like navigation/inspector width, not
   * something that should chase the reader from session to session.
   */
  initialRuntimeSurface?: SessionRuntimeSurfaceId;
  /** Seats currently in the session. A scope naming one that has left is dropped. */
  seatIds: readonly string[];
  sessionId: EvidenceSessionId | null;
}): WorkspaceEvidenceNavigation {
  const [stored, dispatch] = useReducer(
    workspaceEvidenceReducer,
    sessionId,
    (id) => initialWorkspaceEvidenceState(id, initialRuntimeSurface),
  );

  // Reset during render, not in an effect, for two reasons that both bite.
  //
  // A child's effects run before its parent's, so an effect here would land after every panel had
  // already rendered and after their own effects had run — the first frame of a new session would
  // build query keys from the previous session's run and trace, and a panel effect asking for a
  // surface would be undone by a reset that arrives later.
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

  const activateSurface = useCallback((id: SessionSurfaceId) => dispatch({ type: "activate-surface", id }), []);
  const openRuntimePanel = useCallback(() => dispatch({ type: "open-runtime-panel" }), []);
  const closeRuntimePanel = useCallback(() => dispatch({ type: "close-runtime-panel" }), []);
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

  return useMemo<WorkspaceEvidenceNavigation>(
    () => ({
      activePrimarySurface: state.activePrimarySurface,
      runtimePanelOpen: state.runtimePanelOpen,
      activeRuntimeSurface: state.activeRuntimeSurface,
      scope,
      correlation: state.correlation,
      focus: state.focus,
      navigationRevision: state.navigationRevision,
      unsupportedFields: state.unsupportedFields,
      activateSurface,
      openRuntimePanel,
      closeRuntimePanel,
      navigate,
      patchScope,
      clearScope,
    }),
    [activateSurface, clearScope, closeRuntimePanel, navigate, openRuntimePanel, patchScope, scope, state],
  );
}

/** Wraps children in an already-computed value, for the two-subtree case described above. */
export function WorkspaceEvidenceScopeValueProvider({
  children,
  value,
}: {
  children: ReactNode;
  value: WorkspaceEvidenceNavigation;
}) {
  return (
    <WorkspaceEvidenceScopeContext.Provider value={value}>
      {children}
    </WorkspaceEvidenceScopeContext.Provider>
  );
}

export function WorkspaceEvidenceScopeProvider({
  children,
  seatIds,
  sessionId,
}: {
  children: ReactNode;
  seatIds: readonly string[];
  sessionId: EvidenceSessionId | null;
}) {
  const value = useWorkspaceEvidenceScopeValue({ seatIds, sessionId });
  return (
    <WorkspaceEvidenceScopeValueProvider value={value}>{children}</WorkspaceEvidenceScopeValueProvider>
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
