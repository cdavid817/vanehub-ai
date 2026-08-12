import { useCallback, useRef } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { agentService } from "../services/runtime-agent-client";
import type { Session } from "../types/agent";

interface SessionSwitchOptions {
  activeSessionId: string | null;
  onError: (reason: unknown, sessionId: string) => void;
}

export function useSessionSwitch({ activeSessionId, onError }: SessionSwitchOptions) {
  const queryClient = useQueryClient();
  const requestSequence = useRef(0);
  const selectedSessionId = useRef(activeSessionId);
  selectedSessionId.current = activeSessionId;
  const mutation = useMutation({
    mutationFn: (session: Session) => agentService.switchSession(session.id),
    onMutate: async (session) => {
      requestSequence.current += 1;
      const requestId = requestSequence.current;
      const previous = queryClient.getQueryData<Session | null>(["sessions", "active"]) ?? null;
      const cancellation = queryClient.cancelQueries({ exact: true, queryKey: ["sessions", "active"] });
      queryClient.setQueryData(["sessions", "active"], session);
      await cancellation;
      return { previous, requestId };
    },
    onSuccess: (session, _input, context) => {
      if (context?.requestId !== requestSequence.current) return;
      selectedSessionId.current = session.id;
      queryClient.setQueryData(["sessions", "active"], session);
    },
    onError: (reason, input, context) => {
      if (context?.requestId !== requestSequence.current) return;
      selectedSessionId.current = context.previous?.id ?? null;
      queryClient.setQueryData(["sessions", "active"], context.previous);
      onError(reason, input.id);
    },
  });
  const mutate = mutation.mutate;

  return useCallback((session: Session) => {
    if (session.archived || session.id === selectedSessionId.current) return;
    selectedSessionId.current = session.id;
    mutate(session);
  }, [mutate]);
}
