import { useMemo, type ReactNode } from "react";
import type { Session } from "../types/agent";
import type { ChatMessage } from "../types/chat";
import { MessageList } from "../components/chat/MessageList";
import { TurnStatusBar, type TurnStatus } from "../components/chat/TurnStatusBar";
import { useSessionSpeakers } from "../hooks/use-session-speakers";

export function ChatTab({
  activeSession,
  composer,
  currentSelectionKey = null,
  messages,
  onLoadEarlier,
  onSelectMessage,
  onSelectTool,
  turnStatus = null,
}: {
  activeSession: Session | null;
  composer: ReactNode;
  /** `workbenchSelectionKey` of the Inspector's current selection, when it is a message or tool
   * call; null otherwise. Absent entirely until a caller wires selection (design.md Decision 8). */
  currentSelectionKey?: string | null;
  messages: ChatMessage[];
  onLoadEarlier: () => void;
  onSelectMessage?: (messageId: string) => void;
  onSelectTool?: (messageId: string, toolCallId: string) => void;
  turnStatus?: TurnStatus | null;
}) {
  const speakers = useSessionSpeakers(activeSession);
  // Memoized on the two fields it carries. A fresh object each render would defeat `MessageItem`'s
  // `memo`, and every historical row would re-render -- and re-parse its markdown -- on every
  // streamed token, which is exactly what that memo exists to prevent.
  const agentId = activeSession?.agentId ?? null;
  const projectPath = activeSession?.projectPath ?? null;
  const memoryContext = useMemo(
    () => (agentId ? { agentId, projectPath } : null),
    [agentId, projectPath],
  );
  return (
    <div className="flex h-full min-h-0 flex-col bg-[hsl(var(--panel-muted))]" data-testid="contiguous-chat-workspace">
      <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
        {turnStatus ? <TurnStatusBar status={turnStatus} /> : null}
        <MessageList
          currentSelectionKey={currentSelectionKey}
          hasActiveSession={Boolean(activeSession)}
          memoryContext={memoryContext}
          hasMore={messages.length >= 50}
          messages={messages}
          onLoadEarlier={onLoadEarlier}
          onSelectMessage={onSelectMessage}
          onSelectTool={onSelectTool}
          speakers={speakers}
        />
      </div>
      {composer ? <div className="shrink-0 border-t border-border/70 bg-[hsl(var(--panel))]" data-testid="attached-chat-composer">{composer}</div> : null}
    </div>
  );
}

