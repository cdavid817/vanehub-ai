import { useEffect, useMemo, useState } from "react";
import type { Session } from "../types/agent";
import type { ChatMessage } from "../types/chat";
import { cn } from "../lib/utils";
import { AgentTerminalTab } from "./agent-terminal-tab";
import { ChangesTab } from "./changes-tab";
import { DocumentsTab } from "./documents-tab";
import { FilesTab } from "./files-tab";
import { LogsTab } from "./logs-tab";
import { ReportTab } from "./report-tab";
import { SessionTabBar, sessionTabDefinitions, type SessionTabId } from "./session-tab-bar";
import { ShellTab } from "./shell-tab";
import { TerminalTab, toolUseCount } from "./terminal-tab";

export function SessionTabs({
  activeSession,
  messages,
  messagesPartial,
  onOpenSettings,
  sessionActivationKey,
}: {
  activeSession: Session | null;
  messages: ChatMessage[];
  messagesPartial: boolean;
  onOpenSettings: () => void;
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

  function activate(tab: SessionTabId) {
    setMountedTabs((current) => new Set(current).add(tab));
    setActiveTab(tab);
  }

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <SessionTabBar activeTab={activeTab} badges={{ terminal: terminalCount }} onActivate={activate} onOpenSettings={onOpenSettings} session={activeSession} />
      <div className="min-h-0 flex-1 overflow-hidden">
        {sessionTabDefinitions.map(({ id }) => mountedTabs.has(id) ? (
          <section
            aria-labelledby={`session-tab-${id}`}
            className={cn("h-full min-h-0", activeTab === id ? "block" : "hidden")}
            id={`session-tab-panel-${id}`}
            key={`${sessionId ?? "none"}-${id}`}
            role="tabpanel"
          >
            {id === "chat" ? <AgentTerminalTab active={activeTab === "chat"} session={activeSession} sessionActivationKey={sessionActivationKey} /> : null}
            {id === "changes" ? <ChangesTab sessionId={sessionId} /> : null}
            {id === "documents" ? <DocumentsTab sessionId={sessionId} /> : null}
            {id === "files" ? <FilesTab sessionId={sessionId} /> : null}
            {id === "terminal" ? <TerminalTab messages={messages} partial={messagesPartial} /> : null}
            {id === "shell" ? <ShellTab active={activeTab === "shell"} sessionId={sessionId} /> : null}
            {id === "logs" ? <LogsTab sessionId={sessionId} /> : null}
            {id === "report" ? <ReportTab messages={messages} partial={messagesPartial} /> : null}
          </section>
        ) : null)}
      </div>
    </div>
  );
}

