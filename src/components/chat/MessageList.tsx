import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { ChatMessage } from "../../types/chat";
import { MessageItem } from "./MessageItem";
import { ScrollControl } from "./ScrollControl";
import { WelcomeScreen } from "./WelcomeScreen";

function isNearBottom(element: HTMLDivElement) {
  return element.scrollHeight - element.scrollTop - element.clientHeight < 96;
}

export function MessageList({
  hasActiveSession,
  hasMore,
  messages,
  onLoadEarlier,
}: {
  hasActiveSession: boolean;
  hasMore: boolean;
  messages: ChatMessage[];
  onLoadEarlier: () => void;
}) {
  const { t } = useTranslation();
  const scrollRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);

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
        className="grid h-full content-start gap-3 overflow-y-auto p-4"
        onScroll={(event) => setAutoScroll(isNearBottom(event.currentTarget))}
        ref={scrollRef}
      >
        {hasMore ? (
          <button className="mx-auto h-8 rounded border border-border px-3 text-xs text-muted-foreground hover:bg-muted" onClick={onLoadEarlier} type="button">
            {t("chat.loadEarlier")}
          </button>
        ) : null}
        {messages.map((message) => (
          <MessageItem key={message.id} message={message} />
        ))}
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
