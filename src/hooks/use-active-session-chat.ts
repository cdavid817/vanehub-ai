import { useEffect, useRef } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { applyChatEvent } from "../services/chat-events";
import { agentService } from "../services/runtime-agent-client";
import type { ChatMessage } from "../types/chat";

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
    void agentService.subscribeMessageEvents(sessionId, (event) => {
      queryClient.setQueryData<ChatMessage[]>(queryKey, (current) => applyChatEvent(current ?? [], event));
      if (event.type === "completed" || event.type === "failed" || event.type === "cancelled") {
        onTerminalRef.current?.();
      }
    }).then((unsubscribe) => {
      if (active) cleanup = unsubscribe;
      else unsubscribe();
    });
    return () => {
      active = false;
      cleanup?.();
    };
  }, [queryClient, queryKey, sessionId]);
}
