import { useCallback, useEffect } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { agentService } from "../services/runtime-agent-client";
import type { Session } from "../types/agent";

interface UseSessionRecoverySyncOptions {
  activeSession: Session | null;
  onAcknowledgementError: (reason: unknown, sessionId: string) => void;
}

export function useSessionRecoverySync({
  activeSession,
  onAcknowledgementError,
}: UseSessionRecoverySyncOptions) {
  const queryClient = useQueryClient();
  const activeSessionId = activeSession?.id ?? null;
  const recoverySummaryQuery = useQuery({
    enabled: Boolean(activeSessionId && activeSession?.recoveryStatus !== "clean"),
    queryKey: ["sessions", activeSessionId, "recovery"],
    queryFn: () => activeSessionId
      ? agentService.getSessionRecoverySummary(activeSessionId)
      : Promise.reject(new Error("An active session is required.")),
  });
  const invalidateRecovery = useCallback((sessionId: string) => {
    void queryClient.invalidateQueries({ exact: true, queryKey: ["sessions"] });
    void queryClient.invalidateQueries({ queryKey: ["sessions", "active"] });
    void queryClient.invalidateQueries({ queryKey: ["sessions", sessionId, "recovery"] });
    void queryClient.invalidateQueries({ queryKey: ["messages", sessionId] });
  }, [queryClient]);
  const pollRecovery = useCallback((sessionId: string) => {
    void queryClient.invalidateQueries({ queryKey: ["sessions", "active"] });
    void queryClient.invalidateQueries({ queryKey: ["sessions", sessionId, "recovery"] });
  }, [queryClient]);
  const acknowledgement = useMutation({
    mutationFn: ({ sessionId, recoveryRevision }: { sessionId: string; recoveryRevision: number }) =>
      agentService.acknowledgeSessionRecovery(sessionId, recoveryRevision),
    onError: (reason, input) => onAcknowledgementError(reason, input.sessionId),
    onSettled: (_result, _reason, input) => invalidateRecovery(input.sessionId),
  });

  useEffect(() => {
    let cleanup: (() => void) | null = null;
    let cancelled = false;
    void agentService.subscribeSessionEvents((event) => {
      if ("recoveryRevision" in event) invalidateRecovery(event.sessionId);
    }).then((unsubscribe) => { if (cancelled) unsubscribe(); else cleanup = unsubscribe; });
    return () => { cancelled = true; cleanup?.(); };
  }, [invalidateRecovery]);

  useEffect(() => {
    if (!activeSession || activeSession.recoveryStatus !== "reconciling") return;
    const timer = setInterval(() => pollRecovery(activeSession.id), 5_000);
    return () => clearInterval(timer);
  }, [activeSession, pollRecovery]);

  return {
    acknowledgeRecovery: async () => {
      if (!activeSession || activeSession.recoveryStatus !== "action_required") return;
      await acknowledgement.mutateAsync({
        sessionId: activeSession.id,
        recoveryRevision: activeSession.recoveryRevision,
      });
    },
    acknowledgingRecovery: acknowledgement.isPending,
    recoverySummary: activeSession?.recoveryStatus === "clean"
      ? null
      : recoverySummaryQuery.data ?? null,
  };
}
