import { useTranslation } from "react-i18next";
import { MeasuredVirtualList } from "../measured-virtual-list";
import type { MessageSpeaker } from "../../services/message-speaker";
import type { ChatMessage } from "../../types/chat";
import { workbenchSelectionKey } from "../../types/workbench-selection";
import { MessageItem } from "./MessageItem";
import type { MessageMemoryContext } from "./MessageMemoryMenu";
import { ScrollControl } from "./ScrollControl";
import { selectedToolCallIdFor, startsNewRun } from "./MessageList";
import { useVirtualizedMessageWindow } from "./use-virtualized-message-window";

type VirtualMessageItem =
  | { kind: "message"; message: ChatMessage }
  | { kind: "load-more" };

/**
 * `MessageList`'s counterpart above `MESSAGE_LIST_VIRTUALIZE_THRESHOLD` messages (task 10.12). A separate
 * component, not a branch inline in `MessageList`, so the far more common non-virtualized path
 * stays exactly as simple as it was before this task.
 *
 * `onLoadEarlier` becomes a virtual row at index 0 instead of a sibling above the scroll region
 * (`MessageList`'s own layout): `MeasuredVirtualList` owns exactly one scrollable region, and a
 * conversation long enough to be virtualized is also long enough to plausibly still have more
 * history beyond what's loaded — dropping that affordance once virtualized, rather than moving
 * it, would be a real reachability regression, not just a cosmetic one. Mirrors `logs-tab.tsx`'s
 * own `VirtualLogItem` union for the identical "one more row kind besides the real ones" problem.
 *
 * `anchorTo="end"` (task 21.8): loading earlier messages prepends rows ahead of whatever the
 * reader is currently looking at, which is not always the bottom (`useVirtualizedMessageWindow`'s
 * own `scrollToIndex`-on-append effect only re-anchors when `autoScroll` is true). Without this,
 * `MeasuredVirtualList`'s held `scrollOffset` would not move to compensate, and the reader would
 * see a jump the instant the prepended rows are measured in — this opts into
 * `@tanstack/react-virtual`'s own built-in edge-key-anchored repositioning to prevent exactly that,
 * the same mechanism the non-virtualized path gets for free from `useConversationWindowModel`'s
 * own `ResizeObserver`-driven `anchoredScrollTop`.
 */
export function VirtualizedMessageList({
  currentSelectionKey = null,
  hasMore,
  memoryContext = null,
  messages,
  onLoadEarlier,
  onSelectMessage,
  onSelectTool,
  speakers,
}: {
  currentSelectionKey?: string | null;
  hasMore: boolean;
  memoryContext?: MessageMemoryContext | null;
  messages: ChatMessage[];
  onLoadEarlier: () => void;
  onSelectMessage?: (messageId: string) => void;
  onSelectTool?: (messageId: string, toolCallId: string) => void;
  speakers?: Map<string | number, MessageSpeaker>;
}) {
  const { t } = useTranslation();
  const { autoScroll, listRef, onAtEndChange, scrollToBottom } = useVirtualizedMessageWindow(messages);

  const virtualItems: VirtualMessageItem[] = hasMore
    ? [{ kind: "load-more" }, ...messages.map((message) => ({ kind: "message" as const, message }))]
    : messages.map((message) => ({ kind: "message" as const, message }));

  return (
    <div className="relative min-h-0 flex-1 overflow-hidden">
      <MeasuredVirtualList
        anchorTo="end"
        ariaLabel={t("chat.conversationTranscript")}
        className="h-full px-3 py-5 sm:px-4 lg:px-6 xl:px-8"
        estimateSize={() => 96}
        getItemKey={(item) => (item.kind === "message" ? item.message.id : "load-more")}
        items={virtualItems}
        onAtEndChange={onAtEndChange}
        overscan={8}
        ref={listRef}
        renderItem={(item, virtualIndex) => {
          if (item.kind === "load-more") {
            return (
              <button className="mx-auto flex h-8 items-center rounded border border-border px-3 text-xs text-muted-foreground hover:bg-muted" onClick={onLoadEarlier} type="button">
                {t("chat.loadEarlier")}
              </button>
            );
          }
          const { message } = item;
          const messageIndex = hasMore ? virtualIndex - 1 : virtualIndex;
          const selected = currentSelectionKey !== null
            && currentSelectionKey === workbenchSelectionKey({ kind: "message", sessionId: message.sessionId, messageId: message.id });
          const showHeader = startsNewRun(message, messages[messageIndex - 1]);
          return (
            <div className={showHeader ? "pt-3" : "pt-0.5"}>
              <MessageItem
                memoryContext={memoryContext}
                message={message}
                onSelect={onSelectMessage}
                onSelectTool={onSelectTool}
                selected={selected}
                selectedToolCallId={selectedToolCallIdFor(message, currentSelectionKey)}
                showHeader={showHeader}
                speaker={speakers?.get(message.speakerSeatId ?? message.seatIndex ?? "") ?? null}
              />
            </div>
          );
        }}
        testId="message-scroll-region"
      />
      <ScrollControl onClick={scrollToBottom} visible={!autoScroll} />
    </div>
  );
}
