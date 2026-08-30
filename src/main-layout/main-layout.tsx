import { useEffect, useRef, useState, type MouseEvent, type PointerEvent as ReactPointerEvent } from "react";
import { ArrowLeft } from "lucide-react";
import { useTranslation } from "react-i18next";
import { NotificationHost, useNotifications } from "../notifications/notification-provider";
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
import { useMediaQuery } from "../hooks/use-media-query";

const sessionSidebarWidthStorageKey = "vanehub.session-sidebar.width.v1";
const minSessionSidebarWidth = 232;
const maxSessionSidebarWidth = 420;
const defaultSessionSidebarWidth = 232;

export function clampSessionSidebarWidth(width: number) {
  return Math.min(maxSessionSidebarWidth, Math.max(minSessionSidebarWidth, Math.round(width)));
}

function readSessionSidebarWidth() {
  if (typeof localStorage === "undefined") return defaultSessionSidebarWidth;
  const stored = Number(localStorage.getItem(sessionSidebarWidthStorageKey));
  return Number.isFinite(stored) ? clampSessionSidebarWidth(stored) : defaultSessionSidebarWidth;
}

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
  const narrowLayout = useMediaQuery("(max-width: 900px)");
  const { recoverSession, recoveringSessionId } = useSessionRuntimeRecovery();
  const [conversationFocusMode, setConversationFocusMode] = useState(false);
  const [infoPanelCollapsed, setInfoPanelCollapsed] = useState(narrowLayout);
  const [requestedInfoTab, setRequestedInfoTab] = useState<"im" | null>(null);
  /**
   * A tab the information panel asked for.
   *
   * The nonce is why this is a pair rather than a bare tab: a reader who clicks Changes, walks
   * back to Chat, and clicks Changes again means it both times, and a request keyed only on the
   * tab would be a no-op the second time.
   */
  const [panelTabRequest, setPanelTabRequest] = useState<{ nonce: number; tab: SessionTabId } | null>(null);
  const [sessionSidebarCollapsed, setSessionSidebarCollapsed] = useState(false);
  const [workspaceTabsCollapsed, setWorkspaceTabsCollapsed] = useState(false);
  const [sessionSidebarWidth, setSessionSidebarWidth] = useState(readSessionSidebarWidth);
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
  const sessionSidebarRef = useRef<HTMLDivElement>(null);
  const workspaceGridRef = useRef<HTMLDivElement>(null);
  const inspectionRequestRef = useRef(0);
  const activatedSessionIdRef = useRef<string | null>(null);
  const effectiveInfoPanelCollapsed = conversationFocusMode || infoPanelCollapsed;
  const effectiveSessionSidebarCollapsed = conversationFocusMode || sessionSidebarCollapsed;
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

  useEffect(() => {
    workspaceGridRef.current?.style.setProperty("--session-sidebar-width", `${sessionSidebarWidth}px`);
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(sessionSidebarWidthStorageKey, String(sessionSidebarWidth));
    }
  }, [sessionSidebarWidth]);

  // The URL and the backend's active session are two claims about the same thing.
  useWorkspaceSessionRoute({ activeSessionId, archivedSessions, location, onNavigate, sessions, switchSession });

  useEffect(() => {
    const previous = activatedSessionIdRef.current;
    activatedSessionIdRef.current = model.activeSessionId;
    if (previous && model.activeSessionId && previous !== model.activeSessionId) {
      setSessionActivationKey((value) => value + 1);
    }
  }, [model.activeSessionId]);

  useEffect(() => {
    if (narrowLayout) setInfoPanelCollapsed(true);
  }, [narrowLayout]);

  function startSessionSidebarResize(event: ReactPointerEvent<HTMLButtonElement>) {
    if (effectiveSessionSidebarCollapsed) return;
    event.preventDefault();
    const startX = event.clientX;
    const startWidth = sessionSidebarWidth;
    const resize = (moveEvent: PointerEvent) => {
      setSessionSidebarWidth(clampSessionSidebarWidth(startWidth + moveEvent.clientX - startX));
    };
    const stop = () => {
      window.removeEventListener("pointermove", resize);
      window.removeEventListener("pointerup", stop);
    };
    window.addEventListener("pointermove", resize);
    window.addEventListener("pointerup", stop, { once: true });
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
      if (target.surface === "usage") setInfoPanelCollapsed(false);
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
      <div className="relative flex h-screen min-h-0 flex-col overflow-hidden">
        <TopBar
          focusMode={conversationFocusMode}
          focusModeAvailable={destination === "sessions"}
          onFocusModeChange={setConversationFocusMode}
          onSearch={() => {
            // Search lives in the session sidebar, so the top bar entry has to reveal it before
            // it can hand over focus.
            goToSessions({});
            setConversationFocusMode(false);
            setSessionSidebarCollapsed(false);
            setSearchFocusToken((token) => token + 1);
          }}
        />
        <div className="relative flex min-h-0 flex-1" data-testid="workspace-frame">
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
              else setSessionSidebarCollapsed((collapsed) => !collapsed);
            }}
            sessionSidebarExpanded={!effectiveSessionSidebarCollapsed}
          />
          <div
            className={cn(
              "ucd-workspace-grid relative min-h-0 min-w-0 flex-1 gap-0",
              destination === "sessions" ? "grid" : "hidden",
            )}
            data-conversation-focus={conversationFocusMode ? "true" : "false"}
            data-info-collapsed={effectiveInfoPanelCollapsed ? "true" : "false"}
            data-session-collapsed={effectiveSessionSidebarCollapsed ? "true" : "false"}
            ref={workspaceGridRef}
          >
            <div
              aria-hidden={effectiveSessionSidebarCollapsed}
              className={cn("ucd-session-sidebar-shell relative flex min-h-0 min-w-0 overflow-visible border-r border-border transition-[opacity,transform] duration-200", effectiveSessionSidebarCollapsed ? "pointer-events-none -translate-x-2 opacity-0" : "opacity-100")}
              id="workspace-session-sidebar"
              // Declarative rather than set from an effect: child effects run before the parent's,
              // so expanding and focusing the search in one click hit a still-inert subtree.
              inert={effectiveSessionSidebarCollapsed}
              ref={sessionSidebarRef}
            >
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
              <button
                aria-label={t("layout.resizeSessionSidebar")}
                className="ucd-session-sidebar-resize"
                onPointerDown={startSessionSidebarResize}
                title={t("layout.resizeSessionSidebar")}
                type="button"
              />
            </div>
            <section className="ucd-conversation-shell flex min-h-0 min-w-0 flex-col border-l border-r border-border/70 bg-background">
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
                    infoPanelExpanded: !effectiveInfoPanelCollapsed,
                    onToggleInfoPanel: () => {
                      if (effectiveInfoPanelCollapsed) {
                        if (conversationFocusMode) setConversationFocusMode(false);
                        setInfoPanelCollapsed(false);
                      } else setInfoPanelCollapsed(true);
                    },
                    onOpenIm: () => {
                      if (conversationFocusMode) setConversationFocusMode(false);
                      setRequestedInfoTab("im");
                      setInfoPanelCollapsed(false);
                    },
                    onToggleSessionList: () => {
                      if (effectiveSessionSidebarCollapsed) {
                        if (conversationFocusMode) setConversationFocusMode(false);
                        setSessionSidebarCollapsed(false);
                      } else setSessionSidebarCollapsed(true);
                    },
                    onToggleWorkspaceTabs: () => {
                      if (conversationFocusMode) setConversationFocusMode(false);
                      setWorkspaceTabsCollapsed((collapsed) => conversationFocusMode ? false : !collapsed);
                    },
                    sessionListExpanded: !effectiveSessionSidebarCollapsed,
                    workspaceTabsExpanded: !(conversationFocusMode || workspaceTabsCollapsed),
                  }}
                  workspaceTabsCollapsed={workspaceTabsCollapsed}
                />
              </div>
            </section>
            <SessionInfoPanel
              activeSession={displayedSession}
              collapsed={effectiveInfoPanelCollapsed}
              currentSpeakerSeatId={loopInspection || model.turnStatus?.kind !== "agent" ? null : model.turnStatus.seatId ?? null}
              messages={loopInspection ? [] : model.messages}
              onNavigateToTab={(tab) => {
                // Focus mode hides the workspace entirely, so a request to show a tab has to leave
                // it first or the reader clicks a row and nothing appears to happen.
                if (conversationFocusMode) setConversationFocusMode(false);
                setPanelTabRequest((current) => ({ nonce: (current?.nonce ?? 0) + 1, tab }));
              }}
              onOpenImSettings={() => onOpenSettings("im")}
              onOpenSkillSettings={() => onOpenSettings("skills")}
              requestedTab={loopInspection?.target.surface === "usage" ? "usage" : requestedInfoTab}
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
          <div
            aria-hidden="true"
            className="pointer-events-none absolute inset-x-0 bottom-0 z-30 h-px bg-border"
            data-testid="workspace-bottom-divider"
          />
        </div>
      </div>
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
      <NotificationHost activeSessionId={model.activeSessionId} />
    </main>
  );
}
