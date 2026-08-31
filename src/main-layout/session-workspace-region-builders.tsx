import { useEffect } from "react";
import type { Session } from "../types/agent";
import type { ConversationVisibilityControls } from "../session-workspace/session-tabs";
import type { SessionRuntimeSurfaceId } from "../session-workspace/session-surface-registry";
import { SessionNotices } from "../session-workspace/session-notices";
import type { MainLayoutModel } from "./use-main-layout-model";
import { patchDestinationLayoutPreference } from "./workbench-layout-preferences";

/** Remembers whichever Runtime Panel tab the reader last had active, across app restarts. */
export function usePersistPreferredRuntimeTab(activeRuntimeSurface: SessionRuntimeSurfaceId) {
  useEffect(() => {
    patchDestinationLayoutPreference("sessions", { preferredRuntimeTab: activeRuntimeSurface });
  }, [activeRuntimeSurface]);
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
