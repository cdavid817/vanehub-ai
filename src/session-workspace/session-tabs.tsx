import { useEffect, useMemo, useState, type ReactNode } from "react";
import { evidenceSessionIdSchema } from "../contracts/session-workspace-evidence-ids";
import { useSessionRoles } from "../hooks/use-session-speakers";
import { activeSeatsFromSession, seatsFromSession } from "../services/session-seats";
import type { SessionRuntimeSurfaceId } from "./session-surface-registry";
import { SessionPrimarySurfaces } from "./session-primary-surfaces";
// Chrome stays eager, matching the primary tab strip's own pattern (`SessionTabBar` is never
// lazy) — each runtime surface's own content is independently behind its own `LazyFeature` inside
// this module, which is the boundary that actually costs bundle size (xterm.js-adjacent Shell and
// Terminal History content, not the tab strip around it).
import { SessionRuntimePanel } from "./session-runtime-panel";
import { useWorkspaceInvalidation } from "./use-workspace-invalidation";
import {
  useWorkspaceEvidenceNotices,
  useWorkspaceEvidenceSummary,
} from "./use-workspace-evidence-summary";
import { workspaceTabBadges } from "./workspace-evidence-badges";
import {
  useWorkspaceEvidenceScopeValue,
  WorkspaceEvidenceScopeValueProvider,
} from "./workspace-evidence-scope";

export type {
  ConversationVisibilityControls,
  SessionWorkspaceRegionsProps,
} from "./session-workspace-regions-props";
import type { SessionWorkspaceRegionsProps } from "./session-workspace-regions-props";

/**
 * Everything `DestinationLayout` needs for the Sessions destination, built from one evidence-scope
 * value shared by two subtrees.
 *
 * A hook rather than a component: `main` and the Runtime Panel's content are sibling props at the
 * `<DestinationLayout>` call site (`DestinationLayoutBody` renders them in different parts of its
 * own tree, not as children either side could wrap together), yet both need the same live
 * navigation state — opening a runtime surface from a Files evidence link must show up in the
 * Runtime Panel without losing which primary surface Files itself was showing. Computing the
 * shared value once and handing it to two `WorkspaceEvidenceScopeValueProvider`s (same value, two
 * JSX subtrees) is what keeps that state a single source of truth instead of two reducers that
 * could drift apart.
 */
export function useSessionWorkspaceRegions(props: SessionWorkspaceRegionsProps): {
  activeRuntimeSurface: SessionRuntimeSurfaceId;
  main: ReactNode;
  runtimePanelContent: ReactNode;
  runtimePanelOpen: boolean;
} {
  const seats = useMemo(
    () => (props.activeSession ? activeSeatsFromSession(props.activeSession) : []),
    [props.activeSession],
  );
  // Kept separate from `seats` rather than folding departed participants into it: `seats` also
  // scopes Shell attachment and evidence filtering (`seatIds` below), and neither should offer a
  // departed participant as a live target. The seat switcher alone renders this list, marked, so a
  // reader can still find a departed participant's history without those other consumers changing.
  const departedSeats = useMemo(
    () => (props.activeSession ? seatsFromSession(props.activeSession).filter((seat) => seat.leftAt != null) : []),
    [props.activeSession],
  );
  const seatIds = useMemo(
    () => seats.flatMap((seat) => (seat.seatId === undefined ? [] : [seat.seatId])),
    [seats],
  );
  // Parsed rather than asserted: the brand is a claim that the value passed validation, and the
  // schema is the only place allowed to make it. A session without a usable id gets no scope,
  // which is the honest answer — not an empty scope that reads as "no filters applied".
  const sessionId = useMemo(() => {
    const parsed = evidenceSessionIdSchema.safeParse(props.activeSession?.id);
    return parsed.success ? parsed.data : null;
  }, [props.activeSession?.id]);

  const scope = useWorkspaceEvidenceScopeValue({
    initialRuntimeSurface: props.initialRuntimeSurface,
    seatIds,
    sessionId,
  });

  // One summary read and one notice subscription for the whole workspace, above every panel so
  // neither multiplies by the number of mounted primary or runtime surfaces.
  const { state, summary } = useWorkspaceEvidenceSummary(sessionId);
  const { recordsRevision } = useWorkspaceEvidenceNotices(sessionId);
  const badges = useMemo(() => workspaceTabBadges(summary, state), [state, summary]);
  useWorkspaceInvalidation(sessionId);

  const [selectedSeat, setSelectedSeat] = useState<number | null>(null);
  useEffect(() => {
    setSelectedSeat(null);
  }, [sessionId]);
  const roles = useSessionRoles(seats.length > 1);

  const main = (
    <WorkspaceEvidenceScopeValueProvider value={scope}>
      <SessionPrimarySurfaces {...props} badges={badges} scope={scope} sessionId={sessionId} />
    </WorkspaceEvidenceScopeValueProvider>
  );

  const runtimePanelContent = (
    <WorkspaceEvidenceScopeValueProvider value={scope}>
      <SessionRuntimePanel
        activeSession={props.activeSession}
        badges={badges}
        departedSeats={departedSeats}
        maximized={props.runtimeMaximized ?? false}
        messages={props.messages}
        messagesPartial={props.messagesPartial}
        onMaximizedChange={props.onRuntimeMaximizedChange ?? (() => undefined)}
        onSelectSeat={setSelectedSeat}
        recordsRevision={recordsRevision}
        roles={roles}
        seats={seats}
        selectedSeat={selectedSeat}
        sessionId={sessionId}
        turnStatus={props.turnStatus ?? null}
      />
    </WorkspaceEvidenceScopeValueProvider>
  );

  return {
    activeRuntimeSurface: scope.activeRuntimeSurface,
    main,
    runtimePanelContent,
    runtimePanelOpen: scope.runtimePanelOpen,
  };
}

/**
 * Both regions composed into one tree, for a host that has nowhere to put a separate
 * resizable Runtime Panel split — a test harness, or a future embed that just wants the whole
 * session workspace rendered without `DestinationLayout`'s pane composition.
 */
export function SessionWorkspaceRegionsHost(props: SessionWorkspaceRegionsProps) {
  const { main, runtimePanelContent, runtimePanelOpen } = useSessionWorkspaceRegions(props);
  return (
    <>
      {main}
      {runtimePanelOpen ? runtimePanelContent : null}
    </>
  );
}
