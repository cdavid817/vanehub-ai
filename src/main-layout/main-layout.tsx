import { useEffect, useRef, useState, type MouseEvent, type PointerEvent as ReactPointerEvent } from "react";
import { ArrowLeft } from "lucide-react";
import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "../components/agent-brand-icon";
import { LazyFeature, type LazyFeatureLoader } from "../components/lazy-feature";
import { NotificationHost, useNotifications } from "../notifications/notification-provider";
import { SessionTabs } from "../session-workspace/session-tabs";
import { ApiSessionComposer } from "../session-workspace/api-session-composer";
import type { SessionTabId } from "../session-workspace/session-tab-bar";
import { agentService } from "../services/runtime-agent-client";
import type { Session } from "../types/agent";
import type { ChatMessage } from "../types/chat";
import type { LoopInspectionTarget } from "../types/loop";
import { CreateSessionDialog } from "./create-session-dialog";
import { SessionContextPanel, type ContextPanelState } from "./session-context-panel";
import { SessionInfoPanel } from "./session-info-panel";
import { SessionSidebar } from "./session-sidebar";
import { ScheduledTasksDialog } from "./scheduled-tasks-dialog";
import { TopBar } from "./top-bar";
import { useMainLayoutModel } from "./use-main-layout-model";
import { WorkspaceActivityBar } from "./workspace-activity-bar";
import { cn } from "../lib/utils";
import { getAgentVisualIdentity } from "../lib/agent-visual-identity";
import type { SettingsPageId } from "../settings/settings-pages";
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

export function ConversationCard({
  active,
  lifecycleLabel,
  onContextMenu,
  onSelect,
  session,
  sourceLabel,
}: {
  active: boolean;
  lifecycleLabel: string;
  language: string;
  onContextMenu: (event: MouseEvent<HTMLButtonElement>) => void;
  onSelect: () => void;
  session: Session;
  sourceLabel?: string;
}) {
  const identity = getAgentVisualIdentity(session.agentId);
  return (
    <button
      className={cn("ucd-list-row relative w-full rounded-lg p-2.5 text-left", active && "border-primary bg-[hsl(var(--nav-active-soft))]")}
      onClick={onSelect}
      onContextMenu={onContextMenu}
      type="button"
    >
      <span className={cn("mr-2 inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-xl border align-middle", identity.tone)} title={identity.label}>
        <AgentBrandIcon agentId={session.agentId} className="h-4 w-4" />
      </span>
      <span className="truncate text-sm font-medium">{session.title}</span>
      <span className="ml-2 text-xs text-muted-foreground">{lifecycleLabel}</span>
      {sourceLabel ? <span className="ml-2 text-xs text-foreground">{sourceLabel}</span> : null}
    </button>
  );
}

export function MainLayout({
  onConfigureOnePiece,
  onOpenSettings,
  openCreateSession = false,
}: {
  onOpenSettings: (pageId?: SettingsPageId) => void;
  onConfigureOnePiece?: () => void;
  openCreateSession?: boolean;
}) {
  const model = useMainLayoutModel();
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
  const [createSessionOpen, setCreateSessionOpen] = useState(openCreateSession);
  const [scheduledTasksOpen, setScheduledTasksOpen] = useState(false);
  const [destination, setDestination] = useState<"sessions" | "loops">("sessions");
  const [loopCenterVisited, setLoopCenterVisited] = useState(false);
  const [loopInspection, setLoopInspection] = useState<LoopInspectionContext | null>(null);
  const [sessionActivationKey, setSessionActivationKey] = useState(0);
  const sessionSidebarRef = useRef<HTMLDivElement>(null);
  const workspaceGridRef = useRef<HTMLDivElement>(null);
  const inspectionRequestRef = useRef(0);
  const activatedSessionIdRef = useRef<string | null>(null);
  const effectiveInfoPanelCollapsed = conversationFocusMode || infoPanelCollapsed;
  const effectiveSessionSidebarCollapsed = conversationFocusMode || sessionSidebarCollapsed;

  useEffect(() => {
    if (sessionSidebarRef.current) sessionSidebarRef.current.inert = effectiveSessionSidebarCollapsed;
  }, [effectiveSessionSidebarCollapsed]);

  useEffect(() => {
    workspaceGridRef.current?.style.setProperty("--session-sidebar-width", `${sessionSidebarWidth}px`);
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(sessionSidebarWidthStorageKey, String(sessionSidebarWidth));
    }
  }, [sessionSidebarWidth]);

  useEffect(() => {
    if (openCreateSession) setCreateSessionOpen(true);
  }, [openCreateSession]);

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
      if (inspectionRequestRef.current !== requestId) return;
      setLoopInspection({ messages, session, target });
      setSessionActivationKey((value) => value + 1);
      if (target.surface === "usage") setInfoPanelCollapsed(false);
      setDestination("sessions");
    } catch (reason: unknown) {
      notify({
        type: "error",
        title: t("loops.inspection.errorTitle"),
        message: reason instanceof Error ? reason.message : String(reason),
        scope: { kind: "session", sessionId: target.sessionId },
      });
    }
  }

  const displayedSession = loopInspection?.session ?? model.activeSession;
  const displayedMessages = loopInspection?.messages ?? model.messages;
  const requestedWorkspaceTab: SessionTabId | null = loopInspection
    ? loopInspection.target.surface === "usage" ? "chat" : loopInspection.target.surface
    : null;
  const usesStructuredChat = Boolean(
    displayedSession && (displayedSession.interactionMode === "api" || seatsFromSession(displayedSession).length > 1),
  );
  const apiComposer = !loopInspection && usesStructuredChat ? (
    <ApiSessionComposer model={model} />
  ) : null;

  return (
    <main className="min-h-screen bg-background text-foreground">
      <div className="relative flex h-screen min-h-0 flex-col overflow-hidden">
        <TopBar
          focusMode={conversationFocusMode}
          focusModeAvailable={destination === "sessions"}
          onFocusModeChange={setConversationFocusMode}
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
              settings: t("layout.activityBar.settings"),
              help: t("layout.activityBar.help"),
            }}
            onOpenSettings={onOpenSettings}
            onLoops={() => {
              setLoopCenterVisited(true);
              setDestination("loops");
            }}
            onScheduledTasks={() => setScheduledTasksOpen(true)}
            onSessions={() => {
              if (destination !== "sessions") setDestination("sessions");
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
              ref={sessionSidebarRef}
            >
              <SessionSidebar
                activeSessionId={model.activeSessionId}
                agentsAvailable={model.agentsAvailable}
                archivedSessions={model.archivedSessions}
                categories={model.categories}
                deletingSessions={model.deletingSessions}
                onAssignCategory={model.assignCategory}
                onBatchDelete={model.deleteSessions}
                onContextMenu={openContextMenu}
                onNew={() => setCreateSessionOpen(true)}
                onSearchChange={model.setSessionSearchQuery}
                onSelect={(session) => {
                  setContextPanel(null);
                  setLoopInspection(null);
                  model.switchSession(session);
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
                    onClick={() => { setLoopInspection(null); setDestination("loops"); }}
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
        onCreateCategory={(session) => {
          const name = window.prompt(t("layout.newCategoryPrompt"));
          if (!name?.trim()) return;
          void model.createCategory(name.trim())
            .then((category) => model.assignCategory(session, category.id))
            .catch((reason: unknown) => {
              notify({ type: "error", title: t("app.error.title"), message: reason instanceof Error ? reason.message : String(reason), scope: { kind: "session", sessionId: session.id } });
            });
        }}
        onDelete={model.deleteSession}
        onDismiss={() => setContextPanel(null)}
        onExport={model.exportSession}
        onPin={model.pinSession}
        onRename={model.renameSession}
        value={contextPanel}
      />
      <CreateSessionDialog agents={model.agents} onClose={() => setCreateSessionOpen(false)} onConfigureOnePiece={() => { setCreateSessionOpen(false); (onConfigureOnePiece ?? onOpenSettings)(); }} onCreated={(session) => { setCreateSessionOpen(false); setLoopInspection(null); model.sessionCreated(session); }} open={createSessionOpen} />
      <ScheduledTasksDialog agents={model.agents} onClose={() => setScheduledTasksOpen(false)} open={scheduledTasksOpen} />
      <NotificationHost activeSessionId={model.activeSessionId} />
    </main>
  );
}
