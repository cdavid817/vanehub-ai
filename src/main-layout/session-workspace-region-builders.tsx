import { useEffect } from "react";
import { Bot } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { Session } from "../types/agent";
import type { ChatMessage } from "../types/chat";
import type { ConversationVisibilityControls } from "../session-workspace/session-tabs";
import type { SessionRuntimeSurfaceId, SessionSurfaceId } from "../session-workspace/session-surface-registry";
import { SessionNotices } from "../session-workspace/session-notices";
import type { HorizontalPaneRegion } from "../ui/destination-layout/regions";
import type { LayoutTier } from "../ui/destination-layout/use-layout-tier";
import { Inspector } from "../ui/inspector/Inspector";
import { useWorkbenchInspection, type WorkbenchInspection } from "../ui/inspector/use-workbench-inspection";
import type { MainLayoutModel } from "./use-main-layout-model";
import { INSPECTOR_WIDTH_BOUNDS, patchDestinationLayoutPreference } from "./workbench-layout-preferences";

/** Remembers whichever Runtime Panel tab the reader last had active, across app restarts. */
export function usePersistPreferredRuntimeTab(activeRuntimeSurface: SessionRuntimeSurfaceId) {
  useEffect(() => {
    patchDestinationLayoutPreference("sessions", { preferredRuntimeTab: activeRuntimeSurface });
  }, [activeRuntimeSurface]);
}

/**
 * Wires `useWorkbenchInspection` to the Sessions destination: the displayed session is Inspector's
 * default follow target (design.md: "主区选择:follow,自动更新"), and workspace-tab/section jump
 * requests route through the same state main-layout.tsx already threads to the workspace tabs.
 */
export function useSessionInspection({
  conversationFocusMode,
  currentSpeakerSeatId,
  displayedMessages,
  displayedSession,
  loopInspectionUsageSurface,
  requestedInfoTab,
  setConversationFocusMode,
  setPanelTabRequest,
}: {
  conversationFocusMode: boolean;
  /** Absent whenever nothing is actively streaming, e.g. during loop inspection. */
  currentSpeakerSeatId: string | null;
  displayedMessages: ChatMessage[];
  displayedSession: Session | null;
  loopInspectionUsageSurface: boolean;
  requestedInfoTab: "im" | null;
  setConversationFocusMode: (focusMode: boolean) => void;
  setPanelTabRequest: (updater: (current: { nonce: number; tab: SessionSurfaceId } | null) => { nonce: number; tab: SessionSurfaceId }) => void;
}): WorkbenchInspection {
  const inspection = useWorkbenchInspection(
    { activeSessionId: displayedSession?.id ?? null },
    {
      currentSpeakerSeatId,
      messages: displayedMessages,
      onNavigateToSessionTab: (tab) => {
        // Focus mode hides the workspace entirely, so a request to show a tab has to leave it
        // first or the reader clicks a row and nothing appears to happen.
        if (conversationFocusMode) setConversationFocusMode(false);
        // `tab` crossed the feature-agnostic InspectorProviderContext boundary as a plain string
        // (ARCH-FE-005) — this feature file is where it is trusted to be a real SessionSurfaceId
        // again, since the only caller is SessionEvidenceSummary's own correctly-typed prop.
        setPanelTabRequest((current) => ({ nonce: (current?.nonce ?? 0) + 1, tab: tab as SessionSurfaceId }));
      },
      requestedSessionSection: loopInspectionUsageSurface ? "usage" : requestedInfoTab,
    },
  );
  useEffect(() => {
    // A no-op while pinned to something else. Keyed only on the session id itself, not on
    // `inspection.follow` (whose identity changes with pin state): re-running this on every
    // pin/unpin would fight a reader who just unpinned a still-relevant selection back to the
    // current session.
    if (displayedSession) inspection.follow({ kind: "session", sessionId: displayedSession.id });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [displayedSession?.id]);
  return inspection;
}

/** `Inspector`'s `overview` slot when nothing is selected/followed — the old panel's own "no session" copy. */
function InspectorOverviewEmptyState() {
  const { t } = useTranslation();
  return (
    <div className="ucd-muted-panel grid gap-2 rounded-lg p-4 text-center">
      <Bot aria-hidden="true" className="mx-auto h-5 w-5 text-muted-foreground" />
      <p className="text-xs font-medium">{t("layout.noSession")}</p>
      <p className="text-[11px] leading-5 text-muted-foreground">{t("layout.startChat")}</p>
    </div>
  );
}

/**
 * The Sessions destination's `inspector` region: `Inspector`'s own `onClose` is only meaningful
 * hosted in a Sheet (narrower tiers have no close affordance of their own beyond Escape/backdrop);
 * inline at `wide`, the pane's own collapse control already covers it.
 */
export function useInspectorRegion({
  commitInfoPanelWidth,
  effectiveInfoPanelOpen,
  infoPanelWidth,
  inspection,
  inspectorTier,
  setInfoPanelOpen,
  setInfoPanelWidth,
}: {
  commitInfoPanelWidth: (width: number) => void;
  effectiveInfoPanelOpen: boolean;
  infoPanelWidth: number;
  inspection: WorkbenchInspection;
  inspectorTier: LayoutTier;
  setInfoPanelOpen: (open: boolean) => void;
  setInfoPanelWidth: (width: number) => void;
}): HorizontalPaneRegion {
  const { t } = useTranslation();
  return {
    content: (
      <Inspector
        detail={inspection.detail}
        mode={inspection.mode}
        onClose={inspectorTier === "wide" ? undefined : () => setInfoPanelOpen(false)}
        onPin={inspection.pin}
        onReturnToOverview={inspection.returnToOverview}
        onUnpin={inspection.unpin}
        overview={<InspectorOverviewEmptyState />}
        title={inspection.title}
      />
    ),
    label: t("layout.infoPanel"),
    max: INSPECTOR_WIDTH_BOUNDS.max,
    min: INSPECTOR_WIDTH_BOUNDS.min,
    onOpenChange: setInfoPanelOpen,
    onWidthChange: setInfoPanelWidth,
    onWidthCommit: commitInfoPanelWidth,
    open: effectiveInfoPanelOpen,
    width: infoPanelWidth,
  };
}

/**
 * Toggling any of the three panes leaves focus mode first — a focus-mode reader who asks for the
 * info panel or session list means "show me that," not "and also drop focus mode for good," but
 * showing either while focus mode still hides the workspace would look like nothing happened.
 */
export function buildConversationVisibilityControls({
  conversationFocusMode,
  effectiveInfoPanelOpen,
  effectiveSessionSidebarOpen,
  setConversationFocusMode,
  setInfoPanelOpen,
  setRequestedInfoTab,
  setSessionSidebarOpen,
  setWorkspaceTabsCollapsed,
  workspaceTabsCollapsed,
}: {
  conversationFocusMode: boolean;
  effectiveInfoPanelOpen: boolean;
  effectiveSessionSidebarOpen: boolean;
  setConversationFocusMode: (focusMode: boolean) => void;
  setInfoPanelOpen: (open: boolean) => void;
  setRequestedInfoTab: (tab: "im") => void;
  setSessionSidebarOpen: (open: boolean) => void;
  setWorkspaceTabsCollapsed: (updater: (collapsed: boolean) => boolean) => void;
  workspaceTabsCollapsed: boolean;
}): ConversationVisibilityControls {
  return {
    infoPanelExpanded: effectiveInfoPanelOpen,
    onToggleInfoPanel: () => {
      if (effectiveInfoPanelOpen) setInfoPanelOpen(false);
      else {
        if (conversationFocusMode) setConversationFocusMode(false);
        setInfoPanelOpen(true);
      }
    },
    onOpenIm: () => {
      if (conversationFocusMode) setConversationFocusMode(false);
      setRequestedInfoTab("im");
      setInfoPanelOpen(true);
    },
    onToggleSessionList: () => {
      if (effectiveSessionSidebarOpen) setSessionSidebarOpen(false);
      else {
        if (conversationFocusMode) setConversationFocusMode(false);
        setSessionSidebarOpen(true);
      }
    },
    onToggleWorkspaceTabs: () => {
      if (conversationFocusMode) setConversationFocusMode(false);
      setWorkspaceTabsCollapsed((collapsed) => conversationFocusMode ? false : !collapsed);
    },
    sessionListExpanded: effectiveSessionSidebarOpen,
    workspaceTabsExpanded: !(conversationFocusMode || workspaceTabsCollapsed),
  };
}

/** Absent during loop inspection: that banner is about another session's transcript, not this one's recovery. */
export function buildRecoveryNotice({
  model,
  recoverSession,
  recoveringSessionId,
  showing,
}: {
  model: MainLayoutModel;
  recoverSession: (session: Session) => Promise<void>;
  recoveringSessionId: string | null;
  showing: boolean;
}) {
  if (!showing) return null;
  return (
    <SessionNotices
      acknowledging={model.acknowledgingRecovery}
      messages={model.messages}
      onAcknowledge={model.acknowledgeRecovery}
      onRecover={(target) => void recoverSession(target)}
      recovering={recoveringSessionId === model.activeSession?.id}
      recoverySummary={model.recoverySummary}
      session={model.activeSession}
    />
  );
}
