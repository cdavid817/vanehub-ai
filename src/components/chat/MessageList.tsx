import { useTranslation } from "react-i18next";
import { cn } from "../../lib/utils";
import type { MessageSpeaker } from "../../services/message-speaker";
import type { ChatMessage } from "../../types/chat";
import { workbenchSelectionKey } from "../../types/workbench-selection";
import { MessageItem } from "./MessageItem";
import type { MessageMemoryContext } from "./MessageMemoryMenu";
import { ScrollControl } from "./ScrollControl";
import { useConversationWindowModel } from "./use-conversation-window-model";
import { VirtualizedMessageList } from "./VirtualizedMessageList";
import { WelcomeScreen } from "./WelcomeScreen";

/**
 * At or above this many messages, `MessageList` switches onto `VirtualizedMessageList` instead
 * (task 10.12: "keep DOM rows bounded for the 5,000-message fixture"). No spec names an exact number
 * for chat the way `SESSION_LIST_VIRTUALIZE_THRESHOLD` (`session-row-list.tsx`) cites one for the
 * session list (`specs/main-layout-ui/spec.md`'s "at least one thousand sessions") — chosen
 * instead with wide margin under the 5,000-message fixture (5,000 / 300 ≈ 17x) so the switch
 * never happens for an ordinary conversation, and the far more battle-tested non-virtualized path
 * below — everything task 10.10/10.11 already anchor-tested — stays untouched for every session
 * that will ever hit this component in normal use. Exported so a test suite can size a fixture
 * around it deliberately without hard-coding the number twice, mirroring that same precedent.
 */
export const MESSAGE_LIST_VIRTUALIZE_THRESHOLD = 300;

export function selectedToolCallIdFor(message: ChatMessage, currentSelectionKey: string | null): string | null {
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
export function startsNewRun(message: ChatMessage, previous: ChatMessage | undefined): boolean {
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

  if (messages.length >= MESSAGE_LIST_VIRTUALIZE_THRESHOLD) {
    return (
      <VirtualizedMessageList
        currentSelectionKey={currentSelectionKey}
        hasMore={hasMore}
        memoryContext={memoryContext}
        messages={messages}
        onLoadEarlier={onLoadEarlier}
        onSelectMessage={onSelectMessage}
        onSelectTool={onSelectTool}
        speakers={speakers}
      />
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
        <div className="grid w-full content-start" data-message-canvas="adaptive" data-testid="message-readable-measure">
        {hasMore ? (
          <button className="mx-auto h-8 rounded border border-border px-3 text-xs text-muted-foreground hover:bg-muted" onClick={onLoadEarlier} type="button">
            {t("chat.loadEarlier")}
          </button>
        ) : null}
        {messages.map((message, index) => {
          const selected = currentSelectionKey !== null
            && currentSelectionKey === workbenchSelectionKey({ kind: "message", sessionId: message.sessionId, messageId: message.id });
          return (
            <div className={cn(startsNewRun(message, messages[index - 1]) ? "pt-3" : "pt-0.5")} key={message.id} ref={registerItemRef(message.id)}>
              <MessageItem
                memoryContext={memoryContext}
                message={message}
                onSelect={onSelectMessage}
                onSelectTool={onSelectTool}
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
