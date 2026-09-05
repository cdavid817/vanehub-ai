import { useEffect, useRef } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { applyChatEvents } from "../services/chat-events";
import { agentService } from "../services/runtime-agent-client";
import type { TurnStatusEvent } from "../services/turn-status";
import type { ChatMessage, ChatStreamEvent } from "../types/chat";
import { createStreamRenderPacer } from "../hooks/stream-render-pacing";

function partitionChatEventsByKnownMessages(
  messages: readonly ChatMessage[],
  events: readonly ChatStreamEvent[],
): { known: ChatStreamEvent[]; unknown: ChatStreamEvent[] } {
  const messageIds = new Set(messages.map((message) => message.id));
  const known: ChatStreamEvent[] = [];
  const unknown: ChatStreamEvent[] = [];
  for (const event of events) {
    if (event.type === "turn_status" || messageIds.has(event.messageId)) known.push(event);
    else unknown.push(event);
  }
  return { known, unknown };
}

/**
 * The active session's chat-event subscription: buffers the token firehose into one array
 * rebuild per render interval, keeps the turn-status bar immediate, and — because stream events
 * can only mutate rows the cache already holds — refetches the message list whenever events
 * target a message this client has never seen (a programmatic send through the service
 * boundary, an IM-originated message, or a seat turn dispatched by the multi-agent
 * coordinator).
 */
export function useSessionStreamEvents({
  invalidateSessions,
  messagesKey,
  onTurnStatus,
  sessionId,
}: {
  invalidateSessions: () => void;
  messagesKey: readonly unknown[];
  onTurnStatus: (status: TurnStatusEvent) => void;
  sessionId: string | null;
}) {
  const queryClient = useQueryClient();
  // Callback props ride refs so a per-render identity does not tear down the subscription.
  const onTurnStatusRef = useRef(onTurnStatus);
  onTurnStatusRef.current = onTurnStatus;
  const invalidateSessionsRef = useRef(invalidateSessions);
  invalidateSessionsRef.current = invalidateSessions;

  useEffect(() => {
    if (!sessionId) return;
    let cleanup: (() => void) | null = null;
    let cancelled = false;
    // Token events arrive in the thousands per turn; buffer them and flush on a paced interval so
    // the message array is rebuilt a few times a second instead of once per token.
    let pending: ChatStreamEvent[] = [];
    // Events for a message the cache has never seen cannot create rows, so the list has to be
    // refetched; the flag collapses a burst into one refetch.
    let refreshingUnknown = false;
    let deferredUnknown: ChatStreamEvent[] = [];
    let reconcileTimer = 0;
    let reconcileAttempts = 0;
    const reconcileUnknown = () => {
      if (refreshingUnknown || deferredUnknown.length === 0 || reconcileAttempts >= 5) return;
      refreshingUnknown = true;
      reconcileAttempts += 1;
      void queryClient.invalidateQueries({ queryKey: messagesKey }).then(() => {
        if (cancelled) return;
        const refreshed = queryClient.getQueryData<ChatMessage[]>(messagesKey) ?? [];
        const partitioned = partitionChatEventsByKnownMessages(refreshed, deferredUnknown);
        deferredUnknown = partitioned.unknown;
        if (partitioned.known.length > 0) {
          reconcileAttempts = 0;
          queryClient.setQueryData<ChatMessage[]>(
            messagesKey,
            applyChatEvents(refreshed, partitioned.known),
          );
        }
      }).catch(() => undefined).finally(() => {
        refreshingUnknown = false;
        if (!cancelled && deferredUnknown.length > 0) {
          reconcileTimer = window.setTimeout(reconcileUnknown, 50);
        }
      });
    };
    const flush = () => {
      if (pending.length === 0) return;
      const batch = pending;
      pending = [];
      const current = queryClient.getQueryData<ChatMessage[]>(messagesKey) ?? [];
      const partitioned = partitionChatEventsByKnownMessages(current, batch);
      queryClient.setQueryData<ChatMessage[]>(messagesKey, applyChatEvents(current, partitioned.known));
      deferredUnknown.push(...partitioned.unknown);
      reconcileUnknown();
    };
    const pacer = createStreamRenderPacer(flush);
    void agentService.subscribeMessageEvents(sessionId, (event) => {
      if (event.type === "turn_status") {
        // turn_status is session-scoped, not message-scoped, and drives the waiting bar; it
        // must update immediately rather than ride a frame delay.
        onTurnStatusRef.current(event.status);
        return;
      }
      pending.push(event);
      if (event.type === "completed" && event.tokenUsage) {
        void queryClient.invalidateQueries({ queryKey: ["session-usage-summary", event.sessionId] });
        void queryClient.invalidateQueries({ queryKey: ["usage-statistics"] });
      }
      // A round that ends leaves nobody holding the turn, so the bar has to go rather than freeze.
      if (["completed", "failed", "cancelled"].includes(event.type)) {
        invalidateSessionsRef.current();
        // Flush the buffered tokens now so the terminal message lands with the status change.
        pacer.flushNow();
      } else {
        pacer.schedule();
      }
    }).then((unsubscribe) => { if (cancelled) unsubscribe(); else cleanup = unsubscribe; });
    return () => {
      cancelled = true;
      pacer.cancel();
      if (reconcileTimer !== 0) window.clearTimeout(reconcileTimer);
      cleanup?.();
    };
  }, [messagesKey, queryClient, sessionId]);
}
