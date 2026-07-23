import { useEffect, useMemo, useState } from "react";
import { LazyFeature } from "../components/lazy-feature";
import { cn } from "../lib/utils";
import type { Session } from "../types/agent";
import type { ChatMessage } from "../types/chat";
import { AgentTerminalTab } from "./agent-terminal-tab";
import { SessionTabBar, sessionTabDefinitions, type SessionTabId } from "./session-tab-bar";
import { toolUseCount } from "./terminal-utils";

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
  messages,
  messagesPartial,
  onOpenSettings,
  requestedTab,
  sessionActivationKey,
}: {
  activeSession: Session | null;
  messages: ChatMessage[];
  messagesPartial: boolean;
  onOpenSettings: () => void;
  requestedTab?: SessionTabId | null;
  sessionActivationKey: number;
}) {
  const sessionId = activeSession?.id ?? null;
  const [activeTab, setActiveTab] = useState<SessionTabId>("chat");
  const [mountedTabs, setMountedTabs] = useState<Set<SessionTabId>>(() => new Set(["chat"]));
  const terminalCount = useMemo(() => toolUseCount(messages), [messages]);

  useEffect(() => {
    setActiveTab("chat");
    setMountedTabs(new Set(["chat"]));
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
      return <AgentTerminalTab active={activeTab === "chat"} session={activeSession} sessionActivationKey={sessionActivationKey} />;
    }
    if (id === "changes") return <LazyFeature componentProps={{ sessionId }} loader={loadChangesTab} />;
    if (id === "documents") return <LazyFeature componentProps={{ sessionId }} loader={loadDocumentsTab} />;
    if (id === "files") return <LazyFeature componentProps={{ sessionId }} loader={loadFilesTab} />;
    if (id === "terminal") {
      return <LazyFeature componentProps={{ messages, partial: messagesPartial }} loader={loadTerminalTab} />;
    }
    if (id === "shell") {
      return <LazyFeature componentProps={{ active: activeTab === "shell", sessionId }} loader={loadShellTab} />;
    }
    if (id === "logs") return <LazyFeature componentProps={{ sessionId }} loader={loadLogsTab} />;
    if (id === "traces") return <LazyFeature componentProps={{ sessionId }} loader={loadExecutionTimelineTab} />;
    return <LazyFeature componentProps={{ messages, partial: messagesPartial }} loader={loadReportTab} />;
  }

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <SessionTabBar
        activeTab={activeTab}
        badges={{ terminal: terminalCount }}
        onActivate={activate}
        onOpenSettings={onOpenSettings}
        session={activeSession}
      />
      <div className="min-h-0 flex-1 overflow-hidden">
        {sessionTabDefinitions.map(({ id }) => mountedTabs.has(id) ? (
          <section
            aria-labelledby={`session-tab-${id}`}
            className={cn("h-full min-h-0", activeTab === id ? "block" : "hidden")}
            id={`session-tab-panel-${id}`}
            key={`${sessionId ?? "none"}-${id}`}
            role="tabpanel"
          >
            {renderPanel(id)}
          </section>
        ) : null)}
      </div>
    </div>
  );
}
