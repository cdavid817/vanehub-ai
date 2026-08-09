import { useEffect, useRef } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { applyChatEvents } from "../services/chat-events";
import { agentService } from "../services/runtime-agent-client";
import type { ChatMessage, ChatStreamEvent } from "../types/chat";

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
    // the message array (O(n)) and re-renders every subscriber. Buffer events and flush on
    // an animation frame so a burst collapses into one array rebuild per frame.
    let pending: ChatStreamEvent[] = [];
    let frame = 0;
    const flush = () => {
      frame = 0;
      if (pending.length === 0) return;
      const batch = pending;
      pending = [];
      queryClient.setQueryData<ChatMessage[]>(queryKey, (current) =>
        applyChatEvents(current ?? [], batch),
      );
    };
    void agentService.subscribeMessageEvents(sessionId, (event) => {
      pending.push(event);
      if (frame === 0) frame = requestAnimationFrame(flush);
      if (event.type === "completed" || event.type === "failed" || event.type === "cancelled") {
        // Terminal events must reach the UI promptly even mid-frame; flush now so the stop
        // indicator and onTerminal callback aren't delayed by up to ~16ms.
        if (frame !== 0) {
          cancelAnimationFrame(frame);
          frame = 0;
        }
        flush();
        onTerminalRef.current?.();
      }
    }).then((unsubscribe) => {
      if (active) cleanup = unsubscribe;
      else unsubscribe();
    });
    return () => {
      active = false;
      if (frame !== 0) cancelAnimationFrame(frame);
      cleanup?.();
    };
  }, [queryClient, queryKey, sessionId]);
}
