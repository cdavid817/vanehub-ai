import { useEffect, useRef, useState, type MouseEvent } from "react";
import { ArrowLeft } from "lucide-react";
import { useTranslation } from "react-i18next";
import { NotificationHost, useNotifications } from "../notifications/notification-provider";
import { AppShell } from "../ui/app-shell/AppShell";
import { DestinationLayout } from "../ui/destination-layout/DestinationLayout";
import { SessionTabs } from "../session-workspace/session-tabs";
import { ApiSessionComposer } from "../session-workspace/api-session-composer";
import type { SessionTabId } from "../session-workspace/session-tab-bar";
import { agentService } from "../services/runtime-agent-client";
import type { Session } from "../types/agent";
import type { ChatMessage } from "../types/chat";
import type { LoopInspectionTarget } from "../types/loop";
import type { MissionControlNavigationTarget } from "../types/mission-control";
import { CreateCategoryDialog } from "./create-category-dialog";
import { CreateSessionDialog } from "./create-session-dialog";
import { PlanDestination } from "./plan-destination";
import { ProjectsDestination } from "./projects-destination";
import { QualityDestination } from "./quality-destination";
import { RunsDestination } from "./runs-destination";
import { SessionContextPanel, type ContextPanelState } from "./session-context-panel";
import { SessionInfoPanel } from "./session-info-panel";
import { SessionSidebar } from "./session-sidebar";
import { nextSlashTabRequestState, type SlashTabRequest } from "./slash-tab-request";
import { TopBar } from "./top-bar";
import { useMainLayoutModel } from "./use-main-layout-model";
import { useWorkspaceSessionRoute } from "./use-workspace-session-route";
import { workspaceActivityBarLabels, WorkspaceActivityBar } from "./workspace-activity-bar";
import { cn } from "../lib/utils";
import type { SettingsPageId } from "../settings/settings-pages";
import type { WorkbenchLocation } from "./workbench-route";
import { seatsFromSession } from "../services/session-seats";
import { SessionNotices } from "../session-workspace/session-notices";
import { useSessionRuntimeRecovery } from "./use-session-runtime-recovery";
import {
  INSPECTOR_WIDTH_BOUNDS,
  NAVIGATION_WIDTH_BOUNDS,
  patchDestinationLayoutPreference,
  readInitialSessionsLayout,
} from "./workbench-layout-preferences";

interface LoopInspectionContext {
  messages: ChatMessage[];
  session: Session;
  target: LoopInspectionTarget;
}

export function MainLayout({
  location,
  onConfigureOnePiece,
  onNavigate,
  onOpenSettings,
}: {
  location: WorkbenchLocation;
  onOpenSettings: (pageId?: SettingsPageId) => void;
  onConfigureOnePiece?: () => void;
  onNavigate: (next: WorkbenchLocation, options?: { replace?: boolean }) => void;
}) {
  const model = useMainLayoutModel();
  const destination = location.destination;
  const { activeSessionId, archivedSessions, sessions, switchSession } = model;
  // Sessions is not a peer of the other four domains — it is the entire session workspace
  // apparatus below, so it gets its own navigation helper that only ever targets its own shape
  // rather than a generic partial-merge across a discriminated union (design.md Decision 1).
  const goToSessions = (next: Partial<Extract<WorkbenchLocation, { destination: "sessions" }>>, options?: { replace?: boolean }) => {
    const base: Extract<WorkbenchLocation, { destination: "sessions" }> = location.destination === "sessions"
      ? location
      : { destination: "sessions", sessionId: null, creatingSession: false };
    onNavigate({ ...base, ...next }, options);
  };
  const { t } = useTranslation();
  const { notify } = useNotifications();
  const { recoverSession, recoveringSessionId } = useSessionRuntimeRecovery();
  // Read once per mount, not per field: three separate localStorage reads could observe a patch
  // landing in between them and seed the sidebar and inspector from inconsistent snapshots.
  const [initialSessionsLayout] = useState(readInitialSessionsLayout);
  const [conversationFocusMode, setConversationFocusMode] = useState(false);
  const [infoPanelOpenState, setInfoPanelOpenState] = useState(initialSessionsLayout.inspectorOpen);
  const [requestedInfoTab, setRequestedInfoTab] = useState<"im" | null>(null);
  /**
   * A tab the information panel asked for.
   *
   * The nonce is why this is a pair rather than a bare tab: a reader who clicks Changes, walks
   * back to Chat, and clicks Changes again means it both times, and a request keyed only on the
   * tab would be a no-op the second time.
   */
  const [panelTabRequest, setPanelTabRequest] = useState<{ nonce: number; tab: SessionTabId } | null>(null);
  // Unlike the inspector's open state, design.md Decision 3's persisted shape has no
  // `preferredNavigationOpen` field — the session list is expected back open on every fresh load.
  const [sessionSidebarOpen, setSessionSidebarOpen] = useState(true);
  const [workspaceTabsCollapsed, setWorkspaceTabsCollapsed] = useState(false);
  const [sessionSidebarWidth, setSessionSidebarWidth] = useState(initialSessionsLayout.navigationWidth);
  const [infoPanelWidth, setInfoPanelWidth] = useState(initialSessionsLayout.inspectorWidth);
  const [contextPanel, setContextPanel] = useState<ContextPanelState | null>(null);
  // Nonce, not just the tab id: requesting the same tab twice in a row (e.g. `/logs` again after
  // the user manually switched back to chat) must still re-trigger `SessionTabs`' activation effect.
  const [slashTabRequest, setSlashTabRequest] = useState<SlashTabRequest | null>(null);
  // Not seeded from a session id: the guard below reconciles this against displayedSession?.id on
  // every render, including the first, so any value that isn't a real session id settles safely.
  const [slashTabRequestSessionId, setSlashTabRequestSessionId] = useState<string | null>(null);
  const [loopInspection, setLoopInspection] = useState<LoopInspectionContext | null>(null);
  const [sessionActivationKey, setSessionActivationKey] = useState(0);
  const [searchFocusToken, setSearchFocusToken] = useState(0);
  const [categoryDialogSession, setCategoryDialogSession] = useState<Session | null>(null);
  const inspectionRequestRef = useRef(0);
  const activatedSessionIdRef = useRef<string | null>(null);
  // The inspector's open preference is a value worth persisting on its own, so every write goes
  // through this instead of the raw setter — unlike width, which only persists on resize-commit.
  function setInfoPanelOpen(open: boolean) {
    setInfoPanelOpenState(open);
    patchDestinationLayoutPreference("sessions", { preferredInspectorOpen: open });
  }
  const effectiveInfoPanelOpen = !conversationFocusMode && infoPanelOpenState;
  const effectiveSessionSidebarOpen = !conversationFocusMode && sessionSidebarOpen;
  // SessionTabs' resets key on its `activeSession` prop (displayedSession here), not on
  // model.activeSessionId: loop inspection displays a different session without the sidebar's
  // active session ever changing. A pending slash-tab request must be invalidated on that same
  // identity — during render, not in an Effect, since a child's effects run before its parent's in
  // the same commit and would already have re-applied a stale tab by the time an Effect ran.
  const displayedSession = loopInspection?.session ?? model.activeSession;
  const nextSlashTabState = nextSlashTabRequestState(slashTabRequest, slashTabRequestSessionId, displayedSession?.id ?? null);
  if (nextSlashTabState.trackedSessionId !== slashTabRequestSessionId) {
    setSlashTabRequestSessionId(nextSlashTabState.trackedSessionId);
    setSlashTabRequest(nextSlashTabState.request);
  }

  // The URL and the backend's active session are two claims about the same thing.
  useWorkspaceSessionRoute({ activeSessionId, archivedSessions, location, onNavigate, sessions, switchSession });

  useEffect(() => {
    const previous = activatedSessionIdRef.current;
    activatedSessionIdRef.current = model.activeSessionId;
    if (previous && model.activeSessionId && previous !== model.activeSessionId) {
      setSessionActivationKey((value) => value + 1);
    }
  }, [model.activeSessionId]);

  // `SplitPane`'s `onResizeEnd` fires once per drag/keypress commit, so these are where a width
  // actually gets persisted — `onSizeChange` alone would write on every pointermove.
  function commitSessionSidebarWidth(width: number) {
    patchDestinationLayoutPreference("sessions", { navigationWidth: width });
  }
  function commitInfoPanelWidth(width: number) {
    patchDestinationLayoutPreference("sessions", { inspectorWidth: width });
  }

  function openContextMenu(event: MouseEvent<HTMLButtonElement>, session: Session) {
    event.preventDefault();
    event.stopPropagation();
    setContextPanel({
      session,
      mode: "menu",
      draftTitle: session.title,
      position: {
        x: event.clientX,
        y: event.clientY,
      },
    });
  }

  async function inspectLoopSession(target: LoopInspectionTarget) {
    const requestId = inspectionRequestRef.current + 1;
    inspectionRequestRef.current = requestId;
    try {
      const [session, messages] = await Promise.all([
        agentService.getSession(target.sessionId),
        agentService.listMessages({ sessionId: target.sessionId }),
      ]);
      // Guarded because the user may have navigated away while this was in flight; navigating
      // from a stale closure would yank them back.
      if (inspectionRequestRef.current !== requestId) return;
      setLoopInspection({ messages, session, target });
      setSessionActivationKey((value) => value + 1);
      if (target.surface === "usage") setInfoPanelOpen(true);
      goToSessions({ sessionId: target.sessionId, creatingSession: false });
    } catch (reason: unknown) {
      notify({
        type: "error",
        title: t("loops.inspection.errorTitle"),
        message: reason instanceof Error ? reason.message : String(reason),
        scope: { kind: "session", sessionId: target.sessionId },
      });
    }
  }

  function navigateFromMissionControl(target: MissionControlNavigationTarget) {
    if (target.kind === "review") setSlashTabRequest((current) => ({ tab: "changes", nonce: (current?.nonce ?? 0) + 1 }));
    goToSessions({ sessionId: target.sessionId ?? target.id, creatingSession: false });
  }

  const displayedMessages = loopInspection?.messages ?? model.messages;
  // Loop inspection wins: it is showing another session's transcript, and a panel row that moved
  // the tab out from under it would leave the reader looking at this session's workspace beside
  // that one's messages.
  const requestedWorkspaceTab: SessionTabId | null = loopInspection
    ? loopInspection.target.surface === "usage" ? "chat" : loopInspection.target.surface
    : panelTabRequest?.tab ?? slashTabRequest?.tab ?? null;
  const usesStructuredChat = Boolean(
    displayedSession && (displayedSession.interactionMode === "api" || seatsFromSession(displayedSession).length > 1),
  );
  const apiComposer = !loopInspection && usesStructuredChat ? (
    <ApiSessionComposer
      model={model}
      navigation={{
        // No visited-flag bookkeeping here: destinations no longer use one. A command is just
        // another way to change the route, and it already names the exact section it wants.
        openDestination: (target) => onNavigate(target),
        openSessionTab: (tab) => setSlashTabRequest((current) => ({ tab, nonce: (current?.nonce ?? 0) + 1 })),
      }}
    />
  ) : null;

  return (
    <main className="min-h-screen bg-background text-foreground">
      <AppShell
        activityRail={(
          <WorkspaceActivityBar
            activeDestination={destination}
            labels={workspaceActivityBarLabels(t)}
            onHelp={() => onOpenSettings("help")}
            onOpenSettings={() => onOpenSettings()}
            onPlan={() => onNavigate({ destination: "plan", section: "board", viewId: undefined, workItemId: undefined })}
            onProjects={() => onNavigate({ destination: "projects", projectId: undefined })}
            onQuality={() => onNavigate({ destination: "quality", section: "evaluations", experimentId: undefined, comparisonIds: undefined })}
            onRuns={() => onNavigate({ destination: "runs", section: "attention", runId: undefined })}
            onSessions={() => {
              if (destination !== "sessions") goToSessions({});
              else if (conversationFocusMode) setConversationFocusMode(false);
              else setSessionSidebarOpen((open) => !open);
            }}
            sessionSidebarExpanded={effectiveSessionSidebarOpen}
          />
        )}
        className="relative h-screen overflow-hidden"
        topBar={(
          <>
            <TopBar
              focusMode={conversationFocusMode}
              focusModeAvailable={destination === "sessions"}
              onFocusModeChange={setConversationFocusMode}
              onSearch={() => {
                // Search lives in the session sidebar, so the top bar entry has to reveal it before
                // it can hand over focus.
                goToSessions({});
                setConversationFocusMode(false);
                setSessionSidebarOpen(true);
                setSearchFocusToken((token) => token + 1);
              }}
            />
            {/* Fixed-position toast viewport (design.md Decision 2's "NotificationAndUtility"
                slice of TopBar) — its placement in the tree has no visual effect since it never
                participates in TopBar's own layout, only in the viewport's. */}
            <NotificationHost activeSessionId={model.activeSessionId} />
          </>
        )}
      >
        {/* `h-full`, not just `flex-1`: `AppShell`'s content slot is `display: block`, and
            `flex-1`/`min-h-0` only affect an element's own sizing as a flex item of a flex
            parent — plain block-level height defaults to content size, not the parent's, which is
            what let this whole subtree grow past the viewport instead of capping the message
            list's own internal scroll region. */}
        <div className="relative flex h-full min-h-0 flex-1" data-testid="workspace-frame">
          <div
            className={cn("relative min-h-0 min-w-0 flex-1", destination === "sessions" ? "block" : "hidden")}
            data-conversation-focus={conversationFocusMode ? "true" : "false"}
            data-info-collapsed={effectiveInfoPanelOpen ? "false" : "true"}
            data-session-collapsed={effectiveSessionSidebarOpen ? "false" : "true"}
            data-testid="sessions-destination-layout"
          >
            <DestinationLayout
              inspector={{
                content: (
                  <SessionInfoPanel
                    activeSession={displayedSession}
                    currentSpeakerSeatId={loopInspection || model.turnStatus?.kind !== "agent" ? null : model.turnStatus.seatId ?? null}
                    messages={loopInspection ? [] : model.messages}
                    onNavigateToTab={(tab) => {
                      // Focus mode hides the workspace entirely, so a request to show a tab has to
                      // leave it first or the reader clicks a row and nothing appears to happen.
                      if (conversationFocusMode) setConversationFocusMode(false);
                      setPanelTabRequest((current) => ({ nonce: (current?.nonce ?? 0) + 1, tab }));
                    }}
                    onOpenImSettings={() => onOpenSettings("im")}
                    onOpenSkillSettings={() => onOpenSettings("skills")}
                    requestedTab={loopInspection?.target.surface === "usage" ? "usage" : requestedInfoTab}
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
              }}
              main={(
                <section className="flex h-full min-h-0 min-w-0 flex-1 flex-col bg-background" data-testid="session-conversation-shell">
                  {/* `flex-1` is load-bearing, not decorative: with the navigation pane closed,
                      `DestinationLayoutBody` renders `main` as a direct flex child of its own
                      row container instead of nested inside `SplitPane`'s block-level wrapper, and
                      a flex item with no grow stays sized to its content instead of filling the
                      freed width. */}
                  {loopInspection ? (
                    <div className="flex min-h-11 shrink-0 items-center gap-2 border-b border-border/70 px-3">
                      <button
                        aria-label={t("loops.inspection.back")}
                        className="grid h-8 w-8 shrink-0 place-items-center rounded-md border border-border text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
                        onClick={() => {
                          // Navigate before clearing the inspected loop session so the session-route
                          // reconciler never observes that hidden role session as a normal deep link.
                          onNavigate({ destination: "runs", section: "loops", definitionId: undefined, loopRunId: undefined });
                          setLoopInspection(null);
                        }}
                        title={t("loops.inspection.back")}
                        type="button"
                      >
                        <ArrowLeft aria-hidden="true" className="h-4 w-4" />
                      </button>
                      <div className="min-w-0">
                        <p className="truncate text-xs font-semibold">{t("loops.inspection.title")}</p>
                        <p className="truncate text-[11px] text-muted-foreground">{loopInspection.session.title}</p>
                      </div>
                    </div>
                  ) : null}
                  <div className="min-h-0 flex-1">
                    <SessionTabs
                      activeSession={displayedSession}
                      apiComposer={apiComposer}
                      focusMode={conversationFocusMode}
                      isStreaming={loopInspection ? false : model.isStreaming}
                      messages={displayedMessages}
                      messagesPartial={loopInspection ? false : model.messagesPartial}
                      onLoadEarlier={model.loadEarlier}
                      onOpenSettings={onOpenSettings}
                      recoveryNotice={!loopInspection ? (
                        <SessionNotices
                          acknowledging={model.acknowledgingRecovery}
                          messages={model.messages}
                          onAcknowledge={model.acknowledgeRecovery}
                          onRecover={(target) => void recoverSession(target)}
                          recovering={recoveringSessionId === model.activeSession?.id}
                          recoverySummary={model.recoverySummary}
                          session={model.activeSession}
                        />
                      ) : null}
                      requestedTab={requestedWorkspaceTab}
                      requestedTabNonce={(panelTabRequest?.nonce ?? 0) + (slashTabRequest?.nonce ?? 0)}
                      sessionActivationKey={sessionActivationKey}
                      turnStatus={loopInspection ? null : model.turnStatus}
                      visibilityControls={{
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
                      }}
                      workspaceTabsCollapsed={workspaceTabsCollapsed}
                    />
                  </div>
                </section>
              )}
              navigation={{
                content: (
                  <div className="h-full min-w-0 bg-[hsl(var(--panel-muted))]" id="workspace-session-sidebar">
                    <SessionSidebar
                      activeSessionId={model.activeSessionId}
                      agentsAvailable={model.agentsAvailable}
                      archivedSessions={model.archivedSessions}
                      categories={model.categories}
                      deletingSessions={model.deletingSessions}
                      focusSearchToken={searchFocusToken}
                      onAssignCategory={model.assignCategory}
                      onBatchDelete={model.deleteSessions}
                      onContextMenu={openContextMenu}
                      onNew={() => goToSessions({ creatingSession: true })}
                      onSearchChange={model.setSessionSearchQuery}
                      onSelect={(session) => {
                        setContextPanel(null);
                        setLoopInspection(null);
                        // The reconciliation effect performs the switch; navigating is what records it.
                        goToSessions({ sessionId: session.id, creatingSession: false });
                      }}
                      searchQuery={model.sessionSearchQuery}
                      searchResults={model.sessionSearchResults}
                      sessions={model.sessions}
                    />
                  </div>
                ),
                label: t("layout.sessions"),
                max: NAVIGATION_WIDTH_BOUNDS.max,
                min: NAVIGATION_WIDTH_BOUNDS.min,
                onOpenChange: setSessionSidebarOpen,
                onWidthChange: setSessionSidebarWidth,
                onWidthCommit: commitSessionSidebarWidth,
                open: effectiveSessionSidebarOpen,
                width: sessionSidebarWidth,
              }}
            />
          </div>
          <div className={cn("min-h-0 min-w-0 flex-1", destination === "sessions" ? "hidden" : "flex")} id="workbench-route-outlet">
            {location.destination === "projects" ? <ProjectsDestination /> : null}
            {location.destination === "runs" ? (
              <RunsDestination
                agents={model.agents}
                location={location}
                onInspectLoop={inspectLoopSession}
                onMissionControlNavigate={navigateFromMissionControl}
                onSectionChange={(section) => onNavigate({ destination: "runs", ...section })}
              />
            ) : null}
            {location.destination === "plan" ? (
              <PlanDestination location={location} onSectionChange={(section) => onNavigate({ destination: "plan", ...section })} />
            ) : null}
            {location.destination === "quality" ? <QualityDestination /> : null}
          </div>
        </div>
      </AppShell>
      {/* A sibling of AppShell, not nested inside its content slot: AppShell's activity rail is a
          normal-flow column ahead of that slot, so an `inset-x-0` line inside it would start after
          the rail instead of running the full window width. */}
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-x-0 bottom-0 z-30 h-px bg-border"
        data-testid="workspace-bottom-divider"
      />
      <SessionContextPanel
        categories={model.categories}
        onArchive={model.archiveSession}
        onAssignCategory={model.assignCategory}
        onChange={setContextPanel}
        onCreateCategory={(session) => setCategoryDialogSession(session)}
        onDelete={model.deleteSession}
        onDismiss={() => setContextPanel(null)}
        onExport={model.exportSession}
        onPin={model.pinSession}
        onRecover={(session) => void recoverSession(session)}
        onRename={model.renameSession}
        recovering={recoveringSessionId !== null}
        value={contextPanel}
      />
      <CreateSessionDialog
        agents={model.agents}
        onClose={() => goToSessions({ creatingSession: false })}
        onConfigureOnePiece={() => { goToSessions({ creatingSession: false }); (onConfigureOnePiece ?? onOpenSettings)(); }}
        onCreated={(session) => {
          setLoopInspection(null);
          model.sessionCreated(session);
          goToSessions({ sessionId: session.id, creatingSession: false }, { replace: true });
        }}
        open={location.destination === "sessions" && location.creatingSession}
      />
      {categoryDialogSession ? (
        <CreateCategoryDialog
          onClose={() => setCategoryDialogSession(null)}
          onCreate={async (name) => {
            const category = await model.createCategory(name);
            model.assignCategory(categoryDialogSession, category.id);
          }}
        />
      ) : null}
    </main>
  );
}
