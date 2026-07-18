import { useEffect, useRef, useState, type MouseEvent } from "react";
import { useTranslation } from "react-i18next";
import { ChatInputBox } from "../components/chat/ChatInputBox";
import { NotificationHost, useNotifications } from "../notifications/notification-provider";
import { SessionTabs } from "../session-workspace/session-tabs";
import type { Session } from "../types/agent";
import { CreateSessionDialog } from "./create-session-dialog";
import { SessionContextPanel, type ContextPanelState } from "./session-context-panel";
import { SessionInfoPanel } from "./session-info-panel";
import { SessionSidebar } from "./session-sidebar";
import { StatusBar } from "./status-bar";
import { TopBar } from "./top-bar";
import { useMainLayoutModel } from "./use-main-layout-model";
import { WorkspaceActivityBar } from "./workspace-activity-bar";
import { cn } from "../lib/utils";
import { getAgentVisualIdentity } from "../lib/agent-visual-identity";

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
        <identity.Icon className="h-3.5 w-3.5" aria-hidden="true" />
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
  const [contextPanel, setContextPanel] = useState<ContextPanelState | null>(null);
  const [createSessionOpen, setCreateSessionOpen] = useState(openCreateSession);
  const sessionSidebarRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (sessionSidebarRef.current) sessionSidebarRef.current.inert = sessionSidebarCollapsed;
  }, [sessionSidebarCollapsed]);

  useEffect(() => {
    if (openCreateSession) setCreateSessionOpen(true);
  }, [openCreateSession]);

  function openContextMenu(event: MouseEvent<HTMLButtonElement>, session: Session) {
    event.preventDefault();
    setContextPanel({ session, mode: "menu", draftTitle: session.title });
  }

  function showScheduledTasksPlaceholder() {
    notify({
      type: "info",
      title: t("layout.activityBar.scheduledTasksTitle"),
      message: t("layout.activityBar.scheduledTasksMessage"),
      scope: { kind: "global" },
    });
  }

  return (
    <main className="min-h-screen bg-background text-foreground">
      <div className="pointer-events-none fixed inset-0 opacity-[0.035] [background-image:linear-gradient(hsl(var(--primary))_1px,transparent_1px),linear-gradient(90deg,hsl(var(--primary))_1px,transparent_1px)] [background-size:100px_100px]" />
      <div className="relative flex h-screen min-h-0 flex-col overflow-hidden">
        <TopBar />
        <div className="relative flex min-h-0 flex-1">
          <WorkspaceActivityBar
            labels={{
              navigation: t("layout.activityBar.label"),
              sessions: t("layout.activityBar.sessions"),
              expandSessions: t("layout.activityBar.expandSessions"),
              collapseSessions: t("layout.activityBar.collapseSessions"),
              scheduledTasks: t("layout.activityBar.scheduledTasks"),
              settings: t("layout.activityBar.settings"),
              help: t("layout.activityBar.help"),
            }}
            onOpenSettings={onOpenSettings}
            onScheduledTasks={showScheduledTasksPlaceholder}
            onToggleSessions={() => setSessionSidebarCollapsed((collapsed) => !collapsed)}
            sessionSidebarExpanded={!sessionSidebarCollapsed}
          />
          <div
            className="ucd-workspace-grid relative grid min-h-0 min-w-0 flex-1 gap-4 p-2 transition-[grid-template-columns] duration-200 max-[900px]:gap-2"
            data-info-collapsed={infoPanelCollapsed ? "true" : "false"}
            data-session-collapsed={sessionSidebarCollapsed ? "true" : "false"}
          >
            <div
              aria-hidden={sessionSidebarCollapsed}
              className={cn("ucd-session-sidebar-shell flex min-h-0 min-w-0 overflow-hidden transition-[opacity,transform] duration-200", sessionSidebarCollapsed ? "pointer-events-none -translate-x-2 opacity-0" : "opacity-100")}
              id="workspace-session-sidebar"
              ref={sessionSidebarRef}
            >
              <SessionSidebar
                activeSessionId={model.activeSessionId}
                agentsAvailable={model.agentsAvailable}
                archivedSessions={model.archivedSessions}
                categories={model.categories}
                onAssignCategory={model.assignCategory}
                onContextMenu={openContextMenu}
                onNew={() => setCreateSessionOpen(true)}
                onSearchChange={model.setSessionSearchQuery}
                onSelect={(session) => { setContextPanel(null); model.switchSession(session); }}
                searchQuery={model.sessionSearchQuery}
                searchResults={model.sessionSearchResults}
                sessions={model.sessions}
              />
            </div>
            <section className="ucd-panel flex min-h-0 min-w-0 flex-col rounded-lg p-3">
              <SessionTabs
                activeSession={model.activeSession}
                composer={<ChatInputBox
                  agents={model.chatConfig.availableAgents.length > 0 ? model.chatConfig.availableAgents : model.agents}
                  availableModes={model.chatConfig.availableModes}
                  availableModels={model.chatConfig.availableModels}
                  availableReasoning={model.chatConfig.availableReasoning}
                  config={model.chatConfig.config}
                  disabled={!model.activeSession || model.isSending}
                  fileReferenceCandidates={model.fileReferenceCandidates}
                  fileReferences={model.fileReferences}
                  isStreaming={model.isStreaming}
                  onAddFileReference={model.addFileReference}
                  onChange={model.setDraft}
                  onClear={() => model.setDraft("")}
                  onConfigAgentChange={model.chatConfig.changeAgent}
                  onConfigLongContextChange={model.chatConfig.setLongContext}
                  onConfigModeChange={model.chatConfig.setPermissionMode}
                  onConfigModelChange={model.chatConfig.changeModel}
                  onConfigProviderChange={model.chatConfig.changeProvider}
                  onConfigReasoningChange={model.chatConfig.setReasoningDepth}
                  onConfigStreamingChange={model.chatConfig.setStreaming}
                  onConfigThinkingChange={model.chatConfig.setThinking}
                  onRemoveFileReference={model.removeFileReference}
                  onStop={model.stop}
                  onSubmit={model.submit}
                  value={model.draft}
                />}
                isStreaming={model.isStreaming}
                messages={model.messages}
                messagesPartial={model.messagesPartial}
                onLoadEarlier={model.loadEarlier}
              />
            </section>
            <SessionInfoPanel activeSession={model.activeSession} collapsed={infoPanelCollapsed} onCollapsedChange={setInfoPanelCollapsed} />
          </div>
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
      <CreateSessionDialog agents={model.agents} onClose={() => setCreateSessionOpen(false)} onCreated={(session) => { setCreateSessionOpen(false); model.sessionCreated(session); }} open={createSessionOpen} />
      <NotificationHost activeSessionId={model.activeSessionId} />
    </main>
  );
}
