import { useEffect, useMemo, useState, type ReactNode } from "react";
import { LazyFeature } from "../components/lazy-feature";
import { cn } from "../lib/utils";
import type { Session } from "../types/agent";
import type { TurnStatus } from "../components/chat/TurnStatusBar";
import { useSessionRoles } from "../hooks/use-session-speakers";
import { activeSeatsFromSession, seatsFromSession } from "../services/session-seats";
import { SeatSwitcher } from "./seat-switcher";
import { showsSeatSwitcher } from "./tab-scope";
import type { ChatMessage } from "../types/chat";
import { AgentTerminalTab } from "./agent-terminal-tab";
import { ChatTab } from "./chat-tab";
import { SessionTabBar, sessionTabDefinitions, type SessionTabId } from "./session-tab-bar";
import { toolUseCount } from "./terminal-utils";
import { ConversationOverflowMenu } from "./conversation-overflow-menu";
import { SessionConversationHeader } from "./session-conversation-header";

export interface ConversationVisibilityControls {
  infoPanelExpanded: boolean;
  onToggleInfoPanel: () => void;
  onOpenIm: () => void;
  onToggleSessionList: () => void;
  onToggleWorkspaceTabs: () => void;
  sessionListExpanded: boolean;
  workspaceTabsExpanded: boolean;
}

const loadChangesTab = () => import("./changes-tab").then((module) => ({ default: module.ChangesTab }));
const loadDocumentsTab = () => import("./documents-tab").then((module) => ({ default: module.DocumentsTab }));
const loadFilesTab = () => import("./files-tab").then((module) => ({ default: module.FilesTab }));
const loadTerminalTab = () => import("./terminal-tab").then((module) => ({ default: module.TerminalTab }));
const loadShellTab = () => import("./shell-tab").then((module) => ({ default: module.ShellTab }));
const loadLogsTab = () => import("./logs-tab").then((module) => ({ default: module.LogsTab }));
const loadExecutionTimelineTab = () => import("./execution-timeline-tab")
  .then((module) => ({ default: module.ExecutionTimelineTab }));
const loadReportTab = () => import("./report-tab").then((module) => ({ default: module.ReportTab }));

export function SessionTabs({
  activeSession,
  apiComposer,
  focusMode = false,
  isStreaming = false,
  messages,
  messagesPartial,
  onLoadEarlier = () => undefined,
  onOpenSettings,
  recoveryNotice,
  requestedTab,
  sessionActivationKey,
  turnStatus = null,
  visibilityControls,
  workspaceTabsCollapsed = false,
}: {
  activeSession: Session | null;
  apiComposer?: ReactNode;
  focusMode?: boolean;
  isStreaming?: boolean;
  messages: ChatMessage[];
  messagesPartial: boolean;
  onLoadEarlier?: () => void;
  onOpenSettings: () => void;
  recoveryNotice?: ReactNode;
  requestedTab?: SessionTabId | null;
  sessionActivationKey: number;
  /** Null in a single-seat session, which has no turn to hand off. */
  turnStatus?: TurnStatus | null;
  visibilityControls?: ConversationVisibilityControls;
  workspaceTabsCollapsed?: boolean;
}) {
  const sessionId = activeSession?.id ?? null;
  const seats = useMemo(() => (activeSession ? activeSeatsFromSession(activeSession) : []), [activeSession]);
  const isSharedThread = Boolean(activeSession && seatsFromSession(activeSession).length > 1);
  const [activeTab, setActiveTab] = useState<SessionTabId>("chat");
  const [selectedSeat, setSelectedSeat] = useState(0);
  const roles = useSessionRoles(seats.length > 1);
  const [mountedTabs, setMountedTabs] = useState<Set<SessionTabId>>(() => new Set(["chat"]));
  const terminalCount = useMemo(() => toolUseCount(messages), [messages]);

  useEffect(() => {
    setActiveTab("chat");
    setMountedTabs(new Set(["chat"]));
    setSelectedSeat(0);
  }, [sessionId]);

  useEffect(() => {
    if (!requestedTab) return;
    setMountedTabs((current) => new Set(current).add(requestedTab));
    setActiveTab(requestedTab);
  }, [requestedTab, sessionId]);

  function activate(tab: SessionTabId) {
    setMountedTabs((current) => new Set(current).add(tab));
    setActiveTab(tab);
  }

  function renderPanel(id: SessionTabId) {
    if (id === "chat") {
      if (activeSession?.interactionMode === "api" || isSharedThread) {
        return (
          <ChatTab
            activeSession={activeSession}
            composer={apiComposer}
            messages={messages}
            onLoadEarlier={onLoadEarlier}
            turnStatus={turnStatus}
          />
        );
      }
      return <AgentTerminalTab active={activeTab === "chat"} session={activeSession} sessionActivationKey={sessionActivationKey} />;
    }
    if (id === "changes") return <LazyFeature componentProps={{ sessionId }} loader={loadChangesTab} />;
    if (id === "documents") return <LazyFeature componentProps={{ sessionId }} loader={loadDocumentsTab} />;
    if (id === "files") return <LazyFeature componentProps={{ sessionId }} loader={loadFilesTab} />;
    if (id === "terminal") {
      return <LazyFeature componentProps={{ builtinToolsAvailable: activeSession?.agentId === "onepiece", messages, partial: messagesPartial, sessionId, targetRoot: activeSession?.worktreePath ?? activeSession?.projectPath ?? "" }} loader={loadTerminalTab} />;
    }
    if (id === "shell") {
      return <LazyFeature componentProps={{ active: activeTab === "shell", sessionId }} loader={loadShellTab} />;
    }
    if (id === "logs") return <LazyFeature componentProps={{ sessionId }} loader={loadLogsTab} />;
    if (id === "traces") return <LazyFeature componentProps={{ session: activeSession, sessionId }} loader={loadExecutionTimelineTab} />;
    return <LazyFeature componentProps={{ messages, partial: messagesPartial }} loader={loadReportTab} />;
  }

  return (
    <div
      className="flex h-full min-h-0 flex-col"
      data-focus-mode={focusMode ? "true" : "false"}
      data-testid="session-workspace"
    >
      <SessionConversationHeader
        actions={visibilityControls ? <ConversationOverflowMenu {...visibilityControls} /> : null}
        isStreaming={isStreaming}
        onOpenIm={visibilityControls?.onOpenIm}
        session={activeSession}
      />
      {recoveryNotice}
      {focusMode || workspaceTabsCollapsed ? null : (
        <div className="shrink-0 border-b border-border/70 bg-[hsl(var(--panel))] px-3 py-2">
          <SessionTabBar
            activeTab={activeTab}
            badges={{ terminal: terminalCount > 0 ? terminalCount : undefined }}
            onActivate={activate}
            onOpenSettings={onOpenSettings}
            session={activeSession}
          />
        </div>
      )}
      <div className="min-h-0 flex-1 overflow-hidden">
        {sessionTabDefinitions.map(({ id }) => mountedTabs.has(id) ? (
          <section
            aria-labelledby={`session-tab-${id}`}
            className={cn("h-full min-h-0", activeTab === id ? "block" : "hidden")}
            id={`session-tab-panel-${id}`}
            key={`${sessionId ?? "none"}-${id}`}
            role="tabpanel"
          >
            {showsSeatSwitcher(id, seats.length) ? (
              <SeatSwitcher
                onSelect={setSelectedSeat}
                roles={roles}
                seats={seats}
                selectedIndex={selectedSeat}
              />
            ) : null}
            {renderPanel(id)}
          </section>
        ) : null)}
      </div>
    </div>
  );
}
