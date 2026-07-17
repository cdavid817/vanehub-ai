import { useEffect, useState, type MouseEvent } from "react";
import { ChatInputBox } from "../components/chat/ChatInputBox";
import { NotificationHost } from "../notifications/notification-provider";
import { SessionTabs } from "../session-workspace/session-tabs";
import type { Session } from "../types/agent";
import { CreateSessionDialog } from "./create-session-dialog";
import { SessionContextPanel, type ContextPanelState } from "./session-context-panel";
import { SessionInfoPanel } from "./session-info-panel";
import { SessionSidebar } from "./session-sidebar";
import { StatusBar } from "./status-bar";
import { TopBar } from "./top-bar";
import { useMainLayoutModel } from "./use-main-layout-model";
import { cn } from "../lib/utils";

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
  return (
    <button
      className={cn("ucd-list-row relative w-full rounded-lg p-2.5 text-left", active && "border-primary bg-[hsl(var(--nav-active-soft))]")}
      onClick={onSelect}
      onContextMenu={onContextMenu}
      type="button"
    >
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
  const [infoPanelCollapsed, setInfoPanelCollapsed] = useState(false);
  const [contextPanel, setContextPanel] = useState<ContextPanelState | null>(null);
  const [createSessionOpen, setCreateSessionOpen] = useState(openCreateSession);

  useEffect(() => {
    if (openCreateSession) setCreateSessionOpen(true);
  }, [openCreateSession]);

  function openContextMenu(event: MouseEvent<HTMLButtonElement>, session: Session) {
    event.preventDefault();
    setContextPanel({ session, mode: "menu", draftTitle: session.title });
  }

  return (
    <main className="min-h-screen bg-background text-foreground">
      <div className="pointer-events-none fixed inset-0 opacity-[0.035] [background-image:linear-gradient(hsl(var(--primary))_1px,transparent_1px),linear-gradient(90deg,hsl(var(--primary))_1px,transparent_1px)] [background-size:100px_100px]" />
      <div className="relative flex h-screen min-h-0 flex-col overflow-hidden">
        <TopBar />
        <div className="ucd-workspace-grid relative grid min-h-0 flex-1 gap-4 p-2 transition-[grid-template-columns] duration-200 max-[900px]:gap-2" data-info-collapsed={infoPanelCollapsed ? "true" : "false"}>
          <SessionSidebar
            activeSessionId={model.activeSessionId}
            agentsAvailable={model.agentsAvailable}
            archivedSessions={model.archivedSessions}
            onContextMenu={openContextMenu}
            onNew={() => setCreateSessionOpen(true)}
            onOpenSettings={onOpenSettings}
            onSelect={(session) => { setContextPanel(null); model.switchSession(session); }}
            sessions={model.sessions}
          />
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
                isStreaming={model.isStreaming}
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
        <StatusBar />
      </div>
      <SessionContextPanel
        onArchive={model.archiveSession}
        onChange={setContextPanel}
        onDelete={model.deleteSession}
        onDismiss={() => setContextPanel(null)}
        onPin={model.pinSession}
        onRename={model.renameSession}
        value={contextPanel}
      />
      <CreateSessionDialog agents={model.agents} onClose={() => setCreateSessionOpen(false)} onCreated={(session) => { setCreateSessionOpen(false); model.sessionCreated(session); }} open={createSessionOpen} />
      <NotificationHost activeSessionId={model.activeSessionId} />
    </main>
  );
}
