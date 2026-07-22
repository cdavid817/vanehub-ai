import { useEffect, useRef, useState, type MouseEvent, type PointerEvent as ReactPointerEvent } from "react";
import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "../components/agent-brand-icon";
import { NotificationHost, useNotifications } from "../notifications/notification-provider";
import { SessionTabs } from "../session-workspace/session-tabs";
import type { Session } from "../types/agent";
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

export function clampSessionSidebarWidth(width: number) {
  return Math.min(maxSessionSidebarWidth, Math.max(minSessionSidebarWidth, Math.round(width)));
}

function readSessionSidebarWidth() {
  if (typeof localStorage === "undefined") return defaultSessionSidebarWidth;
  const stored = Number(localStorage.getItem(sessionSidebarWidthStorageKey));
  return Number.isFinite(stored) ? clampSessionSidebarWidth(stored) : defaultSessionSidebarWidth;
}

export function ConversationCard({ active, lifecycleLabel, onContextMenu, onSelect, session, sourceLabel }: {
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
    <button className={cn("ucd-list-row relative w-full rounded-lg p-2.5 text-left", active && "border-primary bg-[hsl(var(--nav-active-soft))]")} onClick={onSelect} onContextMenu={onContextMenu} type="button">
      <span className={cn("mr-2 inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-xl border align-middle", identity.tone)} title={identity.label}>
        <AgentBrandIcon agentId={session.agentId} className="h-4 w-4" />
      </span>
      <span className="truncate text-sm font-medium">{session.title}</span>
      <span className="ml-2 text-xs text-muted-foreground">{lifecycleLabel}</span>
      {sourceLabel ? <span className="ml-2 text-xs text-foreground">{sourceLabel}</span> : null}
    </button>
  );
}

export function MainLayout({ onOpenSettings, openCreateSession = false }: { onOpenSettings: () => void; openCreateSession?: boolean }) {
  const model = useMainLayoutModel();
  const { t } = useTranslation();
  const { notify } = useNotifications();
  const [infoPanelCollapsed, setInfoPanelCollapsed] = useState(false);
  const [sessionSidebarCollapsed, setSessionSidebarCollapsed] = useState(false);
  const [sessionSidebarWidth, setSessionSidebarWidth] = useState(readSessionSidebarWidth);
  const [contextPanel, setContextPanel] = useState<ContextPanelState | null>(null);
  const [createSessionOpen, setCreateSessionOpen] = useState(openCreateSession);
  const [scheduledTasksOpen, setScheduledTasksOpen] = useState(false);
  const [sessionActivationKey, setSessionActivationKey] = useState(0);
  const sessionSidebarRef = useRef<HTMLDivElement>(null);
  const workspaceGridRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (sessionSidebarRef.current) sessionSidebarRef.current.inert = sessionSidebarCollapsed;
  }, [sessionSidebarCollapsed]);

  useEffect(() => {
    workspaceGridRef.current?.style.setProperty("--session-sidebar-width", `${sessionSidebarWidth}px`);
    if (typeof localStorage !== "undefined") localStorage.setItem(sessionSidebarWidthStorageKey, String(sessionSidebarWidth));
  }, [sessionSidebarWidth]);

  useEffect(() => {
    if (openCreateSession) setCreateSessionOpen(true);
  }, [openCreateSession]);

  function startSessionSidebarResize(event: ReactPointerEvent<HTMLButtonElement>) {
    if (sessionSidebarCollapsed) return;
    event.preventDefault();
    const startX = event.clientX;
    const startWidth = sessionSidebarWidth;
    const resize = (moveEvent: PointerEvent) => setSessionSidebarWidth(clampSessionSidebarWidth(startWidth + moveEvent.clientX - startX));
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
    setContextPanel({ session, mode: "menu", draftTitle: session.title, position: { x: event.clientX, y: event.clientY } });
  }

  return (
    <main className="min-h-screen bg-background text-foreground">
      <div className="pointer-events-none fixed inset-0 opacity-[0.035] [background-image:linear-gradient(hsl(var(--primary))_1px,transparent_1px),linear-gradient(90deg,hsl(var(--primary))_1px,transparent_1px)] [background-size:100px_100px]" />
      <div className="relative flex h-screen min-h-0 flex-col overflow-hidden">
        <TopBar />
        <div className="relative flex min-h-0 flex-1">
          <WorkspaceActivityBar labels={{ navigation: t("layout.activityBar.label"), sessions: t("layout.activityBar.sessions"), expandSessions: t("layout.activityBar.expandSessions"), collapseSessions: t("layout.activityBar.collapseSessions"), scheduledTasks: t("layout.activityBar.scheduledTasks"), settings: t("layout.activityBar.settings"), help: t("layout.activityBar.help") }} onOpenSettings={onOpenSettings} onScheduledTasks={() => setScheduledTasksOpen(true)} onToggleSessions={() => setSessionSidebarCollapsed((collapsed) => !collapsed)} sessionSidebarExpanded={!sessionSidebarCollapsed} />
          <div className="ucd-workspace-grid relative grid min-h-0 min-w-0 flex-1 gap-4 p-2 transition-[grid-template-columns] duration-200 max-[900px]:gap-2" data-info-collapsed={infoPanelCollapsed ? "true" : "false"} data-session-collapsed={sessionSidebarCollapsed ? "true" : "false"} ref={workspaceGridRef}>
            <div aria-hidden={sessionSidebarCollapsed} className={cn("ucd-session-sidebar-shell relative flex min-h-0 min-w-0 overflow-visible transition-[opacity,transform] duration-200", sessionSidebarCollapsed ? "pointer-events-none -translate-x-2 opacity-0" : "opacity-100")} id="workspace-session-sidebar" ref={sessionSidebarRef}>
              <SessionSidebar activeSessionId={model.activeSessionId} agentsAvailable={model.agentsAvailable} archivedSessions={model.archivedSessions} categories={model.categories} deletingSessions={model.deletingSessions} onAssignCategory={model.assignCategory} onBatchDelete={model.deleteSessions} onContextMenu={openContextMenu} onNew={() => setCreateSessionOpen(true)} onSearchChange={model.setSessionSearchQuery} onSelect={(session) => { setContextPanel(null); setSessionActivationKey((value) => value + 1); model.switchSession(session); }} searchQuery={model.sessionSearchQuery} searchResults={model.sessionSearchResults} sessions={model.sessions} />
              <button aria-label={t("layout.resizeSessionSidebar")} className="ucd-session-sidebar-resize" onPointerDown={startSessionSidebarResize} title={t("layout.resizeSessionSidebar")} type="button" />
            </div>
            <section className="ucd-panel flex min-h-0 min-w-0 flex-col rounded-lg p-3">
              <SessionTabs activeSession={model.activeSession} messages={model.messages} messagesPartial={model.messagesPartial} onOpenSettings={onOpenSettings} sessionActivationKey={sessionActivationKey} />
            </section>
            <SessionInfoPanel activeSession={model.activeSession} collapsed={infoPanelCollapsed} messages={model.messages} onCollapsedChange={setInfoPanelCollapsed} />
          </div>
        </div>
        <StatusBar />
      </div>
      <SessionContextPanel categories={model.categories} onArchive={model.archiveSession} onAssignCategory={model.assignCategory} onChange={setContextPanel} onCreateCategory={(session) => { const name = window.prompt(t("layout.newCategoryPrompt")); if (!name?.trim()) return; void model.createCategory(name.trim()).then((category) => model.assignCategory(session, category.id)).catch((reason: unknown) => { notify({ type: "error", title: t("app.error.title"), message: reason instanceof Error ? reason.message : String(reason), scope: { kind: "session", sessionId: session.id } }); }); }} onDelete={model.deleteSession} onDismiss={() => setContextPanel(null)} onExport={model.exportSession} onPin={model.pinSession} onRename={model.renameSession} value={contextPanel} />
      <CreateSessionDialog agents={model.agents} onClose={() => setCreateSessionOpen(false)} onCreated={(session) => { setCreateSessionOpen(false); model.sessionCreated(session); }} open={createSessionOpen} />
      <ScheduledTasksDialog agents={model.agents} onClose={() => setScheduledTasksOpen(false)} open={scheduledTasksOpen} />
      <NotificationHost activeSessionId={model.activeSessionId} />
    </main>
  );
}
