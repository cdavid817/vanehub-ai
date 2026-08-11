import type { ReactNode } from "react";
import type { Session } from "../types/agent";
import type { ChatMessage } from "../types/chat";
import { MessageList } from "../components/chat/MessageList";
import { TurnStatusBar, type TurnStatus } from "../components/chat/TurnStatusBar";
import { useSessionSpeakers } from "../hooks/use-session-speakers";

export function ChatTab({
  activeSession,
  composer,
  messages,
  onLoadEarlier,
  turnStatus = null,
}: {
  activeSession: Session | null;
  composer: ReactNode;
  messages: ChatMessage[];
  onLoadEarlier: () => void;
  turnStatus?: TurnStatus | null;
}) {
  const speakers = useSessionSpeakers(activeSession);
  return (
    <div className="flex h-full min-h-0 flex-col bg-[hsl(var(--panel-muted))]" data-testid="contiguous-chat-workspace">
      <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
        {turnStatus ? <TurnStatusBar status={turnStatus} /> : null}
        <MessageList
          hasActiveSession={Boolean(activeSession)}
          hasMore={messages.length >= 50}
          messages={messages}
          onLoadEarlier={onLoadEarlier}
          speakers={speakers}
        />
      </div>
      {composer ? <div className="shrink-0 border-t border-border/70 bg-[hsl(var(--panel))]" data-testid="attached-chat-composer">{composer}</div> : null}
    </div>
  );
}

