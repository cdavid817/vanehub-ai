import { useTranslation } from "react-i18next";
import type { MessageSpeaker } from "../../services/message-speaker";
import type { ChatMessage } from "../../types/chat";
import { workbenchSelectionKey } from "../../types/workbench-selection";
import { MessageItem } from "./MessageItem";
import type { MessageMemoryContext } from "./MessageMemoryMenu";
import { ScrollControl } from "./ScrollControl";
import { useConversationWindowModel } from "./use-conversation-window-model";
import { WelcomeScreen } from "./WelcomeScreen";

function selectedToolCallIdFor(message: ChatMessage, currentSelectionKey: string | null): string | null {
  if (currentSelectionKey === null) return null;
  const match = message.toolUse?.find(
    (tool) => workbenchSelectionKey({ kind: "tool", sessionId: message.sessionId, messageId: message.id, toolCallId: tool.id }) === currentSelectionKey,
  );
  return match?.id ?? null;
}

/**
 * Whether `message` starts a new run (task 10.4's continuous transcript hierarchy) rather than
 * continuing the previous one — same role, and for an assistant message, the same seat. A
 * message with no predecessor always starts a run; `MessageItem` itself additionally forces a
 * header for any non-"completed" status regardless of what this returns (task 10.5).
 */
function startsNewRun(message: ChatMessage, previous: ChatMessage | undefined): boolean {
  if (!previous || previous.role !== message.role) return true;
  if (message.role !== "assistant") return true;
  return (message.speakerSeatId ?? message.seatIndex ?? null) !== (previous.speakerSeatId ?? previous.seatIndex ?? null);
}

export function MessageList({
  currentSelectionKey = null,
  hasActiveSession,
  hasMore,
  memoryContext = null,
  messages,
  onLoadEarlier,
  onSelectMessage,
  onSelectTool,
  speakers,
}: {
  /** `workbenchSelectionKey` of the Inspector's current selection, when it is a message or tool
   * call in this session; null otherwise. Absent entirely until a caller wires selection. */
  currentSelectionKey?: string | null;
  hasActiveSession: boolean;
  hasMore: boolean;
  /** Threaded from the session rather than read here: a message knows neither Agent nor project. */
  memoryContext?: MessageMemoryContext | null;
  messages: ChatMessage[];
  onLoadEarlier: () => void;
  onSelectMessage?: (messageId: string) => void;
  onSelectTool?: (messageId: string, toolCallId: string) => void;
  /** Empty for a single-Agent session, which renders exactly as it did before seats existed. */
  speakers?: Map<string | number, MessageSpeaker>;
}) {
  const { t } = useTranslation();
  const { autoScroll, onScroll, registerItemRef, scrollRef, scrollToBottom } = useConversationWindowModel(messages);

  if (messages.length === 0) {
    return (
      <div className="min-h-0 flex-1 overflow-hidden">
        <WelcomeScreen hasActiveSession={hasActiveSession} />
      </div>
    );
  }

  return (
    <div className="relative min-h-0 flex-1 overflow-hidden">
      <div
        className="h-full overflow-y-auto px-3 py-5 sm:px-4 lg:px-6 xl:px-8"
        data-testid="message-scroll-region"
        onScroll={onScroll}
        ref={scrollRef}
      >
        <div className="grid w-full content-start gap-1" data-message-canvas="adaptive" data-testid="message-readable-measure">
        {hasMore ? (
          <button className="mx-auto h-8 rounded border border-border px-3 text-xs text-muted-foreground hover:bg-muted" onClick={onLoadEarlier} type="button">
            {t("chat.loadEarlier")}
          </button>
        ) : null}
        {messages.map((message, index) => {
          const selected = currentSelectionKey !== null
            && currentSelectionKey === workbenchSelectionKey({ kind: "message", sessionId: message.sessionId, messageId: message.id });
          return (
            <div key={message.id} ref={registerItemRef(message.id)}>
              <MessageItem
                memoryContext={memoryContext}
                message={message}
                onSelect={onSelectMessage ? () => onSelectMessage(message.id) : undefined}
                onSelectTool={onSelectTool ? (toolCallId: string) => onSelectTool(message.id, toolCallId) : undefined}
                selected={selected}
                selectedToolCallId={selectedToolCallIdFor(message, currentSelectionKey)}
                showHeader={startsNewRun(message, messages[index - 1])}
                speaker={speakers?.get(message.speakerSeatId ?? message.seatIndex ?? "") ?? null}
              />
            </div>
          );
        })}
        </div>
      </div>
      <ScrollControl onClick={scrollToBottom} visible={!autoScroll} />
    </div>
  );
}
