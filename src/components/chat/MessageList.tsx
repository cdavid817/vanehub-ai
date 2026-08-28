import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { MessageSpeaker } from "../../services/message-speaker";
import type { ChatMessage } from "../../types/chat";
import { MessageItem } from "./MessageItem";
import type { MessageMemoryContext } from "./MessageMemoryMenu";
import { ScrollControl } from "./ScrollControl";
import { WelcomeScreen } from "./WelcomeScreen";

function isNearBottom(element: HTMLDivElement) {
  return element.scrollHeight - element.scrollTop - element.clientHeight < 96;
}

export function anchoredScrollTop(autoScroll: boolean, previousHeight: number, currentHeight: number, currentTop: number) {
  if (autoScroll) return currentHeight;
  return Math.max(0, currentTop + currentHeight - previousHeight);
}

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
  const scrollRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const autoScrollRef = useRef(true);

  useLayoutEffect(() => {
    const element = scrollRef.current;
    if (!element || typeof ResizeObserver === "undefined") return;
    let previousHeight = element.scrollHeight;
    let animationFrame = 0;
    const observer = new ResizeObserver(() => {
      cancelAnimationFrame(animationFrame);
      animationFrame = requestAnimationFrame(() => {
        const currentHeight = element.scrollHeight;
        if (currentHeight !== previousHeight) {
          element.scrollTop = anchoredScrollTop(autoScrollRef.current, previousHeight, currentHeight, element.scrollTop);
        }
        previousHeight = currentHeight;
      });
    });
    observer.observe(element);
    if (element.firstElementChild) observer.observe(element.firstElementChild);
    return () => {
      cancelAnimationFrame(animationFrame);
      observer.disconnect();
    };
  }, []);

  useEffect(() => {
    const element = scrollRef.current;
    if (!element || !autoScroll) return;
    element.scrollTop = element.scrollHeight;
  }, [autoScroll, messages]);

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
        onScroll={(event) => {
          const nextAutoScroll = isNearBottom(event.currentTarget);
          autoScrollRef.current = nextAutoScroll;
          setAutoScroll(nextAutoScroll);
        }}
        ref={scrollRef}
      >
        <div className="grid w-full content-start gap-4" data-message-canvas="adaptive" data-testid="message-readable-measure">
        {hasMore ? (
          <button className="mx-auto h-8 rounded border border-border px-3 text-xs text-muted-foreground hover:bg-muted" onClick={onLoadEarlier} type="button">
            {t("chat.loadEarlier")}
          </button>
        ) : null}
        {messages.map((message) => (
          <MessageItem
            key={message.id}
            memoryContext={memoryContext}
            message={message}
            speaker={speakers?.get(message.speakerSeatId ?? message.seatIndex ?? "") ?? null}
          />
        ))}
        </div>
      </div>
      <ScrollControl
        onClick={() => {
          const element = scrollRef.current;
          if (!element) return;
          element.scrollTop = element.scrollHeight;
          setAutoScroll(true);
        }}
        visible={!autoScroll}
      />
    </div>
  );
}
