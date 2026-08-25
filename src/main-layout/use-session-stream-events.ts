import { useEffect, useRef } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { applyChatEvents, hasEventsForUnknownMessages } from "../services/chat-events";
import { agentService } from "../services/runtime-agent-client";
import type { TurnStatusEvent } from "../services/turn-status";
import type { ChatMessage, ChatStreamEvent } from "../types/chat";

/**
 * The active session's chat-event subscription: buffers the token firehose into one array
 * rebuild per animation frame, keeps the turn-status bar immediate, and — because stream events
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
    // Token events arrive in the thousands per turn; buffer them and flush on an animation
    // frame so the message array is rebuilt once per frame instead of once per token.
    let pending: ChatStreamEvent[] = [];
    let frame = 0;
    // Events for a message the cache has never seen cannot create rows, so the list has to be
    // refetched; the flag collapses a burst into one refetch.
    let refreshingUnknown = false;
    let sawUnknown = false;
    const flush = () => {
      frame = 0;
      if (pending.length === 0) return;
      const batch = pending;
      pending = [];
      const current = queryClient.getQueryData<ChatMessage[]>(messagesKey) ?? [];
      queryClient.setQueryData<ChatMessage[]>(messagesKey, applyChatEvents(current, batch));
      if (!refreshingUnknown && hasEventsForUnknownMessages(current, batch)) {
        refreshingUnknown = true;
        sawUnknown = true;
        void queryClient
          .invalidateQueries({ queryKey: messagesKey })
          .finally(() => { refreshingUnknown = false; });
      }
    };
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
        if (frame !== 0) { cancelAnimationFrame(frame); frame = 0; }
        flush();
        // A stream that ever touched unknown rows may have dropped deltas that raced the
        // refetch; one settle-time refetch restores exact parity with the persisted thread.
        if (sawUnknown) {
          sawUnknown = false;
          void queryClient.invalidateQueries({ queryKey: messagesKey });
        }
      } else if (frame === 0) {
        frame = requestAnimationFrame(flush);
      }
    }).then((unsubscribe) => { if (cancelled) unsubscribe(); else cleanup = unsubscribe; });
    return () => {
      cancelled = true;
      if (frame !== 0) cancelAnimationFrame(frame);
      cleanup?.();
    };
  }, [messagesKey, queryClient, sessionId]);
}
