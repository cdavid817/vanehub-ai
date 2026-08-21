import { useCallback, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useNotifications } from "../notifications/notification-provider";
import { agentService } from "../services/runtime-agent-client";
import type { Session } from "../types/agent";

/**
 * One recovery action behind two entry points — the workspace failure banner and the session list
 * context menu — so a session can be unstuck without first being made active.
 */
export function useSessionRuntimeRecovery() {
  const queryClient = useQueryClient();
  const { notify } = useNotifications();
  const { t } = useTranslation();
  const [recoveringSessionId, setRecoveringSessionId] = useState<string | null>(null);

  const recoverSession = useCallback(async (session: Session) => {
    setRecoveringSessionId(session.id);
    try {
      const result = await agentService.recoverSession(session.id);
      // Invalidated rather than patched: the lifecycle the backend settled on is the one to show,
      // and the session list, the active session and the transcript all read it independently.
      await Promise.all([
        queryClient.invalidateQueries({ exact: true, queryKey: ["sessions"] }),
        queryClient.invalidateQueries({ queryKey: ["sessions", "active"] }),
        queryClient.invalidateQueries({ queryKey: ["messages", session.id] }),
      ]);
      notify({
        type: "success",
        title: t("sessionRuntime.recover.successTitle"),
        message: result.cancelledMessageIds.length
          ? t("sessionRuntime.recover.successCancelled", { count: result.cancelledMessageIds.length })
          : t("sessionRuntime.recover.successNothing"),
        scope: { kind: "session", sessionId: session.id },
      });
    } catch (reason: unknown) {
      notify({
        type: "error",
        title: t("sessionRuntime.recover.errorTitle"),
        message: reason instanceof Error ? reason.message : String(reason),
        scope: { kind: "session", sessionId: session.id },
      });
    } finally {
      setRecoveringSessionId(null);
    }
  }, [notify, queryClient, t]);

  return { recoverSession, recoveringSessionId };
}
