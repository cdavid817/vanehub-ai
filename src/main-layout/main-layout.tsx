import { useEffect, useRef, useState, type MouseEvent, type PointerEvent as ReactPointerEvent } from "react";
import { ArrowLeft } from "lucide-react";
import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "../components/agent-brand-icon";
import { LazyFeature, type LazyFeatureLoader } from "../components/lazy-feature";
import { NotificationHost, useNotifications } from "../notifications/notification-provider";
import { SessionTabs } from "../session-workspace/session-tabs";
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
import { StatusBar } from "./status-bar";
import { TopBar } from "./top-bar";
import { useMainLayoutModel } from "./use-main-layout-model";
import { WorkspaceActivityBar } from "./workspace-activity-bar";
import { cn } from "../lib/utils";
import { getAgentVisualIdentity } from "../lib/agent-visual-identity";

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
  onOpenSettings,
  openCreateSession = false,
}: {
  onOpenSettings: () => void;
  openCreateSession?: boolean;
}) {
  const model = useMainLayoutModel();
  const { t } = useTranslation();
  const { notify } = useNotifications();
  const [infoPanelCollapsed, setInfoPanelCollapsed] = useState(false);
  const [sessionSidebarCollapsed, setSessionSidebarCollapsed] = useState(false);
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

  useEffect(() => {
    if (sessionSidebarRef.current) sessionSidebarRef.current.inert = sessionSidebarCollapsed;
  }, [sessionSidebarCollapsed]);

  useEffect(() => {
    workspaceGridRef.current?.style.setProperty("--session-sidebar-width", `${sessionSidebarWidth}px`);
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(sessionSidebarWidthStorageKey, String(sessionSidebarWidth));
    }
  }, [sessionSidebarWidth]);

  useEffect(() => {
    if (openCreateSession) setCreateSessionOpen(true);
  }, [openCreateSession]);

  function startSessionSidebarResize(event: ReactPointerEvent<HTMLButtonElement>) {
    if (sessionSidebarCollapsed) return;
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

  return (
    <main className="min-h-screen bg-background text-foreground">
      <div className="pointer-events-none fixed inset-0 opacity-[0.035] [background-image:linear-gradient(hsl(var(--primary))_1px,transparent_1px),linear-gradient(90deg,hsl(var(--primary))_1px,transparent_1px)] [background-size:100px_100px]" />
      <div className="relative flex h-screen min-h-0 flex-col overflow-hidden">
        <TopBar />
        <div className="relative flex min-h-0 flex-1">
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
              if (destination === "loops") setDestination("sessions");
              else setSessionSidebarCollapsed((collapsed) => !collapsed);
            }}
            sessionSidebarExpanded={!sessionSidebarCollapsed}
          />
          <div
            className={cn("ucd-workspace-grid relative min-h-0 min-w-0 flex-1 gap-4 p-2 transition-[grid-template-columns] duration-200 max-[900px]:gap-2", destination === "sessions" ? "grid" : "hidden")}
            data-info-collapsed={infoPanelCollapsed ? "true" : "false"}
            data-session-collapsed={sessionSidebarCollapsed ? "true" : "false"}
            ref={workspaceGridRef}
          >
            <div
              aria-hidden={sessionSidebarCollapsed}
              className={cn("ucd-session-sidebar-shell relative flex min-h-0 min-w-0 overflow-visible transition-[opacity,transform] duration-200", sessionSidebarCollapsed ? "pointer-events-none -translate-x-2 opacity-0" : "opacity-100")}
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
                  setSessionActivationKey((value) => value + 1);
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
            <section className="ucd-panel flex min-h-0 min-w-0 flex-col rounded-lg p-3">
              {loopInspection ? (
                <div className="mb-3 flex min-h-9 shrink-0 items-center gap-2 border-b border-border/70 pb-2">
                  <button
                    aria-label={t("loops.inspection.back")}
                    className="grid h-8 w-8 shrink-0 place-items-center rounded-md border border-border text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
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
                  messages={displayedMessages}
                  messagesPartial={loopInspection ? false : model.messagesPartial}
                  onOpenSettings={onOpenSettings}
                  requestedTab={requestedWorkspaceTab}
                  sessionActivationKey={sessionActivationKey}
                />
              </div>
            </section>
            <SessionInfoPanel
              activeSession={displayedSession}
              collapsed={infoPanelCollapsed}
              messages={displayedMessages}
              onCollapsedChange={setInfoPanelCollapsed}
              requestedTab={loopInspection?.target.surface === "usage" ? "usage" : null}
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
        </div>
        <StatusBar />
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
      <CreateSessionDialog agents={model.agents} onClose={() => setCreateSessionOpen(false)} onCreated={(session) => { setCreateSessionOpen(false); setLoopInspection(null); model.sessionCreated(session); }} open={createSessionOpen} />
      <ScheduledTasksDialog agents={model.agents} onClose={() => setScheduledTasksOpen(false)} open={scheduledTasksOpen} />
      <NotificationHost activeSessionId={model.activeSessionId} />
    </main>
  );
}
