import { useTranslation } from "react-i18next";
import type { MessageSpeaker } from "../../services/message-speaker";
import type { ChatMessage } from "../../types/chat";
import { MessageItem } from "./MessageItem";
import type { MessageMemoryContext } from "./MessageMemoryMenu";
import { ScrollControl } from "./ScrollControl";
import { useConversationWindowModel } from "./use-conversation-window-model";
import { WelcomeScreen } from "./WelcomeScreen";

export function MessageList({
  hasActiveSession,
  hasMore,
  memoryContext = null,
  messages,
  onLoadEarlier,
  speakers,
}: {
  hasActiveSession: boolean;
  hasMore: boolean;
  /** Threaded from the session rather than read here: a message knows neither Agent nor project. */
  memoryContext?: MessageMemoryContext | null;
  messages: ChatMessage[];
  onLoadEarlier: () => void;
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
        <div className="grid w-full content-start gap-4" data-message-canvas="adaptive" data-testid="message-readable-measure">
        {hasMore ? (
          <button className="mx-auto h-8 rounded border border-border px-3 text-xs text-muted-foreground hover:bg-muted" onClick={onLoadEarlier} type="button">
            {t("chat.loadEarlier")}
          </button>
        ) : null}
        {messages.map((message) => (
          <div key={message.id} ref={registerItemRef(message.id)}>
            <MessageItem
              memoryContext={memoryContext}
              message={message}
              speaker={speakers?.get(message.speakerSeatId ?? message.seatIndex ?? "") ?? null}
            />
          </div>
        ))}
        </div>
      </div>
      <ScrollControl onClick={scrollToBottom} visible={!autoScroll} />
    </div>
  );
}
