import { useEffect, useRef } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { applyChatEvents, hasEventsForUnknownMessages } from "../services/chat-events";
import { agentService } from "../services/runtime-agent-client";
import type { ChatMessage, ChatStreamEvent } from "../types/chat";
import { createStreamRenderPacer } from "./stream-render-pacing";

export function useActiveSessionQuery() {
  return useQuery({
    queryKey: ["sessions", "active"],
    queryFn: () => agentService.getActiveSession(),
  });
}

export function useSessionMessageEvents({
  onTerminal,
  queryKey,
  sessionId,
}: {
  onTerminal?: () => void;
  queryKey: readonly unknown[];
  sessionId: string | null;
}) {
  const queryClient = useQueryClient();
  const onTerminalRef = useRef(onTerminal);
  onTerminalRef.current = onTerminal;

  useEffect(() => {
    if (!sessionId) return;
    let active = true;
    let cleanup: (() => void) | undefined;
    // A turn can emit thousands of `token` events; applying each one immediately rebuilds
    // the message array (O(n)) and re-renders every subscriber. Buffer events and flush on the
    // shared paced interval — this surface renders the same message components as the main window,
    // so pacing only one of them would leave the other re-parsing Markdown per frame.
    let pending: ChatStreamEvent[] = [];
    // Stream events can only mutate rows the cache already holds; events for an unknown id mean
    // the thread advanced outside this client (a programmatic send, an IM message, a seat turn)
    // and the list has to be refetched for the new rows to appear.
    let refreshingUnknown = false;
    const flush = () => {
      if (pending.length === 0) return;
      const batch = pending;
      pending = [];
      const current = queryClient.getQueryData<ChatMessage[]>(queryKey) ?? [];
      queryClient.setQueryData<ChatMessage[]>(queryKey, applyChatEvents(current, batch));
      if (!refreshingUnknown && hasEventsForUnknownMessages(current, batch)) {
        refreshingUnknown = true;
        void queryClient
          .invalidateQueries({ queryKey })
          .finally(() => { refreshingUnknown = false; });
      }
    };
    const pacer = createStreamRenderPacer(flush);
    void agentService.subscribeMessageEvents(sessionId, (event) => {
      pending.push(event);
      if (event.type === "completed" || event.type === "failed" || event.type === "cancelled") {
        // Terminal events must reach the UI promptly; flush now so the stop indicator and the
        // onTerminal callback are not delayed by the render interval.
        pacer.flushNow();
        onTerminalRef.current?.();
      } else {
        pacer.schedule();
      }
    }).then((unsubscribe) => {
      if (active) cleanup = unsubscribe;
      else unsubscribe();
    });
    return () => {
      active = false;
      pacer.cancel();
      cleanup?.();
    };
  }, [queryClient, queryKey, sessionId]);
}
