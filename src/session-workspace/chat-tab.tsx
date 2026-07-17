import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { Session } from "../types/agent";
import type { ChatMessage } from "../types/chat";
import { MessageList } from "../components/chat/MessageList";

export function ChatTab({
  activeSession,
  composer,
  isStreaming,
  messages,
  onLoadEarlier,
}: {
  activeSession: Session | null;
  composer: ReactNode;
  isStreaming: boolean;
  messages: ChatMessage[];
  onLoadEarlier: () => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border border-border bg-[hsl(var(--panel-muted))] shadow-sm">
        <div className="flex items-center justify-between gap-3 border-b border-border p-4">
          <div className="min-w-0">
            <h3 className="truncate text-sm font-semibold">{activeSession?.title ?? t("layout.noSession")}</h3>
            <p className="mt-1 truncate text-xs text-muted-foreground">
              {activeSession ? `${activeSession.agentId} · ${activeSession.interactionMode}` : t("layout.startChat")}
            </p>
          </div>
          <span className="shrink-0 rounded-full bg-[hsl(var(--success-soft))] px-2 py-1 text-xs text-[hsl(var(--success))]">
            {isStreaming ? t("layout.generating") : t("layout.ready")}
          </span>
        </div>
        <MessageList
          hasActiveSession={Boolean(activeSession)}
          hasMore={messages.length >= 50}
          messages={messages}
          onLoadEarlier={onLoadEarlier}
        />
      </div>
      <div className="mt-3">{composer}</div>
    </div>
  );
}

