import { useEffect, useRef, useState, type MouseEvent, type PointerEvent as ReactPointerEvent } from "react";
import { ArrowLeft } from "lucide-react";
import { useTranslation } from "react-i18next";
import { LazyFeature, type LazyFeatureLoader } from "../components/lazy-feature";
import { NotificationHost, useNotifications } from "../notifications/notification-provider";
import { SessionTabs } from "../session-workspace/session-tabs";
import { ApiSessionComposer } from "../session-workspace/api-session-composer";
import type { SessionTabId } from "../session-workspace/session-tab-bar";
import { agentService } from "../services/runtime-agent-client";
import type { Session } from "../types/agent";
import type { ChatMessage } from "../types/chat";
import type { LoopInspectionTarget } from "../types/loop";
import { CreateCategoryDialog } from "./create-category-dialog";
import { CreateSessionDialog } from "./create-session-dialog";
import { SessionContextPanel, type ContextPanelState } from "./session-context-panel";
import { SessionInfoPanel } from "./session-info-panel";
import { SessionSidebar } from "./session-sidebar";
import { nextSlashTabRequestState, type SlashTabRequest } from "./slash-tab-request";
import { ScheduledTasksDialog } from "./scheduled-tasks-dialog";
import { TopBar } from "./top-bar";
import { useMainLayoutModel } from "./use-main-layout-model";
import { useWorkspaceSessionRoute } from "./use-workspace-session-route";
import { WorkspaceActivityBar } from "./workspace-activity-bar";
import { cn } from "../lib/utils";
import type { SettingsPageId } from "../settings/settings-pages";
import type { WorkspaceLocation } from "./workspace-route";
import { seatsFromSession } from "../services/session-seats";
import { SessionRecoveryNotice } from "../session-workspace/session-recovery-notice";
import { useMediaQuery } from "../hooks/use-media-query";

const sessionSidebarWidthStorageKey = "vanehub.session-sidebar.width.v1";
const minSessionSidebarWidth = 220;
const maxSessionSidebarWidth = 420;
const defaultSessionSidebarWidth = 220;
type LoopCenterProps = { onInspect?: (target: LoopInspectionTarget) => void };
const loadLoopCenter: LazyFeatureLoader<LoopCenterProps> = () => import("../loop-center/loop-center")
  .then((module) => ({ default: module.LoopCenter }));
const loadWorkBoard: LazyFeatureLoader<Record<string, never>> = () => import("../work-board/work-board")
  .then((module) => ({ default: module.WorkBoard }));
const loadGoalCenter: LazyFeatureLoader<Record<string, never>> = () => import("../goal-center/goal-center")
  .then((module) => ({ default: module.GoalCenter }));
const loadEvaluationCenter: LazyFeatureLoader<Record<string, never>> = () => import("../evaluation-center/evaluation-center")
  .then((module) => ({ default: module.EvaluationCenter }));
type MissionControlProps = { onNavigate?: (target: import("../types/mission-control").MissionControlNavigationTarget) => void };
const loadMissionControl: LazyFeatureLoader<MissionControlProps> = () => import("../mission-control/mission-control")
  .then((module) => ({ default: module.MissionControl }));

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
  location: WorkspaceLocation;
  onOpenSettings: (pageId?: SettingsPageId) => void;
  onConfigureOnePiece?: () => void;
  onNavigate: (next: WorkspaceLocation, options?: { replace?: boolean }) => void;
}) {
  const model = useMainLayoutModel();
  const destination = location.destination;
  const { activeSessionId, archivedSessions, sessions, switchSession } = model;
  const goTo = (next: Partial<WorkspaceLocation>, options?: { replace?: boolean }) =>
    onNavigate({ ...location, ...next }, options);
  const { t } = useTranslation();
  const { notify } = useNotifications();
  const narrowLayout = useMediaQuery("(max-width: 900px)");
  const [conversationFocusMode, setConversationFocusMode] = useState(false);
  const [infoPanelCollapsed, setInfoPanelCollapsed] = useState(narrowLayout);
  const [requestedInfoTab, setRequestedInfoTab] = useState<"im" | null>(null);
  const [sessionSidebarCollapsed, setSessionSidebarCollapsed] = useState(false);
  const [workspaceTabsCollapsed, setWorkspaceTabsCollapsed] = useState(false);
  const [sessionSidebarWidth, setSessionSidebarWidth] = useState(readSessionSidebarWidth);
  const [contextPanel, setContextPanel] = useState<ContextPanelState | null>(null);
  const [scheduledTasksOpen, setScheduledTasksOpen] = useState(false);
  const [loopCenterVisited, setLoopCenterVisited] = useState(false);
  const [workBoardVisited, setWorkBoardVisited] = useState(false);
  const [goalCenterVisited, setGoalCenterVisited] = useState(false);
  const [evaluationCenterVisited, setEvaluationCenterVisited] = useState(false);
  const [missionControlVisited, setMissionControlVisited] = useState(false);
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

  // Visited flags gate the hidden-but-mounted destinations. Deriving them from the destination
  // rather than from click handlers is what makes a deep link render content instead of nothing.
  useEffect(() => {
    if (destination === "loops") setLoopCenterVisited(true);
    if (destination === "work-board") setWorkBoardVisited(true);
    if (destination === "goals") setGoalCenterVisited(true);
    if (destination === "evaluations") setEvaluationCenterVisited(true);
    if (destination === "mission-control") setMissionControlVisited(true);
  }, [destination]);

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
      goTo({ destination: "sessions", sessionId: target.sessionId, creatingSession: false });
    } catch (reason: unknown) {
      notify({
        type: "error",
        title: t("loops.inspection.errorTitle"),
        message: reason instanceof Error ? reason.message : String(reason),
        scope: { kind: "session", sessionId: target.sessionId },
      });
    }
  }

  const displayedMessages = loopInspection?.messages ?? model.messages;
  const requestedWorkspaceTab: SessionTabId | null = loopInspection
    ? loopInspection.target.surface === "usage" ? "chat" : loopInspection.target.surface
    : slashTabRequest?.tab ?? null;
  const usesStructuredChat = Boolean(
    displayedSession && (displayedSession.interactionMode === "api" || seatsFromSession(displayedSession).length > 1),
  );
  const apiComposer = !loopInspection && usesStructuredChat ? (
    <ApiSessionComposer
      model={model}
      navigation={{
        // No visited-flag bookkeeping here: those are derived from `destination` above, which is
        // what lets a deep link render content. A command is just another way to change it.
        openDestination: (target) => goTo({ destination: target }),
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
            goTo({ destination: "sessions" });
            setConversationFocusMode(false);
            setSessionSidebarCollapsed(false);
            setSearchFocusToken((token) => token + 1);
          }}
        />
        <div className="relative flex min-h-0 flex-1" data-testid="workspace-frame">
          <WorkspaceActivityBar
            activeDestination={destination}
            labels={{
              navigation: t("layout.activityBar.label"),
              sessions: t("layout.activityBar.sessions"),
              expandSessions: t("layout.activityBar.expandSessions"),
              collapseSessions: t("layout.activityBar.collapseSessions"),
              loops: t("layout.activityBar.loops"),
              scheduledTasks: t("layout.activityBar.scheduledTasks"),
              todoBoard: t("layout.activityBar.todoBoard"),
              goals: t("layout.activityBar.goals"),
              evaluations: t("layout.activityBar.evaluations"),
              missionControl: t("layout.activityBar.missionControl"),
              settings: t("layout.activityBar.settings"),
              help: t("layout.activityBar.help"),
            }}
            onHelp={() => onOpenSettings("about")}
            onOpenSettings={onOpenSettings}
            onLoops={() => goTo({ destination: "loops" })}
            onScheduledTasks={() => setScheduledTasksOpen(true)}
            onWorkBoard={() => goTo({ destination: "work-board" })}
            onGoals={() => goTo({ destination: "goals" })}
            onEvaluations={() => goTo({ destination: "evaluations" })}
            onMissionControl={() => goTo({ destination: "mission-control" })}
            onSessions={() => {
              if (destination !== "sessions") goTo({ destination: "sessions" });
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
                onNew={() => goTo({ destination: "sessions", creatingSession: true })}
                onSearchChange={model.setSessionSearchQuery}
                onSelect={(session) => {
                  setContextPanel(null);
                  setLoopInspection(null);
                  // The reconciliation effect performs the switch; navigating is what records it.
                  goTo({ destination: "sessions", sessionId: session.id, creatingSession: false });
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
            <section className="flex min-h-0 min-w-0 flex-col border-r border-border/70 bg-background">
              {loopInspection ? (
                <div className="flex min-h-11 shrink-0 items-center gap-2 border-b border-border/70 px-3">
                  <button
                    aria-label={t("loops.inspection.back")}
                    className="grid h-8 w-8 shrink-0 place-items-center rounded-md border border-border text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
                    onClick={() => {
                      // Navigate before clearing the inspected loop session so the session-route
                      // reconciler never observes that hidden role session as a normal deep link.
                      goTo({ destination: "loops" });
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
                    <SessionRecoveryNotice
                      acknowledging={model.acknowledgingRecovery}
                      onAcknowledge={model.acknowledgeRecovery}
                      session={model.activeSession}
                      summary={model.recoverySummary}
                    />
                  ) : null}
                  requestedTab={requestedWorkspaceTab}
                  requestedTabNonce={slashTabRequest?.nonce ?? 0}
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
              onOpenImSettings={() => onOpenSettings("im")}
              onOpenSkillSettings={() => onOpenSettings("skills")}
              requestedTab={loopInspection?.target.surface === "usage" ? "usage" : requestedInfoTab}
            />
          </div>
          <section
            aria-label={t("layout.activityBar.todoBoard")}
            className={cn("min-h-0 min-w-0 flex-1 p-2", destination === "work-board" ? "flex" : "hidden")}
            id="work-board"
          >
            {workBoardVisited ? <LazyFeature className="h-full min-h-0 flex-1" componentProps={{}} loader={loadWorkBoard} /> : null}
          </section>
          <section
            aria-label={t("layout.activityBar.goals")}
            className={cn("min-h-0 min-w-0 flex-1 p-2", destination === "goals" ? "flex" : "hidden")}
            id="goal-center"
          >
            {goalCenterVisited ? <LazyFeature className="h-full min-h-0 flex-1" componentProps={{}} loader={loadGoalCenter} /> : null}
          </section>
          <section aria-label={t("layout.activityBar.evaluations")} className={cn("min-h-0 min-w-0 flex-1 p-2", destination === "evaluations" ? "flex" : "hidden")} id="evaluation-center">
            {evaluationCenterVisited ? <LazyFeature className="h-full min-h-0 flex-1" componentProps={{}} loader={loadEvaluationCenter} /> : null}
          </section>
          <section aria-label={t("layout.activityBar.missionControl")} className={cn("min-h-0 min-w-0 flex-1 p-2", destination === "mission-control" ? "flex" : "hidden")} id="mission-control">
            {missionControlVisited ? <LazyFeature className="h-full min-h-0 flex-1" componentProps={{ onNavigate: (target) => {
              if (target.kind === "review") setSlashTabRequest((current) => ({ tab: "changes", nonce: (current?.nonce ?? 0) + 1 }));
              goTo({ destination: "sessions", sessionId: target.sessionId ?? target.id });
            } }} loader={loadMissionControl} /> : null}
          </section>
          <section
            aria-label={t("layout.activityBar.loops")}
            className={cn("min-h-0 min-w-0 flex-1 p-2", destination === "loops" ? "flex" : "hidden")}
            id="loop-center"
          >
            {loopCenterVisited ? (
              <LazyFeature
                className="h-full min-h-0 flex-1"
                componentProps={{ onInspect: inspectLoopSession }}
                loader={loadLoopCenter}
              />
            ) : null}
          </section>
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
        onRename={model.renameSession}
        value={contextPanel}
      />
      <CreateSessionDialog
        agents={model.agents}
        onClose={() => goTo({ creatingSession: false })}
        onConfigureOnePiece={() => { goTo({ creatingSession: false }); (onConfigureOnePiece ?? onOpenSettings)(); }}
        onCreated={(session) => {
          setLoopInspection(null);
          model.sessionCreated(session);
          goTo({ destination: "sessions", sessionId: session.id, creatingSession: false }, { replace: true });
        }}
        open={location.creatingSession}
      />
      <ScheduledTasksDialog agents={model.agents} onClose={() => setScheduledTasksOpen(false)} open={scheduledTasksOpen} />
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
