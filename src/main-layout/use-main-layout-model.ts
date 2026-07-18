import { useCallback, useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useChatConfig } from "../components/chat/hooks/useChatConfig";
import { createChatOperationFailureEvent } from "./chat-operation-failure";
import { useNotifications } from "../notifications/notification-provider";
import { applyChatEvent } from "../services/chat-events";
import { agentService } from "../services/runtime-agent-client";
import { settingsService } from "../services/runtime-settings-client";
import type { Session } from "../types/agent";
import type { ChatConfig, ChatMessage } from "../types/chat";

export function useMainLayoutModel() {
  const { t } = useTranslation();
  const { notify } = useNotifications();
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState("");
  const [messageLimit, setMessageLimit] = useState(50);
  const agentsQuery = useQuery({ queryKey: ["agents"], queryFn: () => agentService.listAgents() });
  const sessionsQuery = useQuery({ queryKey: ["sessions"], queryFn: () => agentService.listSessions() });
  const archivedQuery = useQuery({ queryKey: ["sessions", "archived"], queryFn: () => agentService.listArchivedSessions() });
  const activeQuery = useQuery({ queryKey: ["sessions", "active"], queryFn: () => agentService.getActiveSession() });
  const agents = agentsQuery.data ?? [];
  const activeSession = activeQuery.data ?? null;
  const activeSessionId = activeSession?.id ?? null;
  const messagesKey = ["messages", activeSessionId, messageLimit] as const;
  const messagesQuery = useQuery({
    enabled: Boolean(activeSessionId), queryKey: messagesKey,
    queryFn: () => activeSessionId ? agentService.listMessages({ sessionId: activeSessionId, limit: messageLimit }) : Promise.resolve([]),
  });
  const messages = messagesQuery.data ?? [];
  const isStreaming = messages.some((message) => message.status === "streaming");
  const reportChatFailure = useCallback((source: string, reason: unknown, sessionId: string | null, restoreDraft?: string) => {
    const event = createChatOperationFailureEvent(source, reason);
    if (restoreDraft !== undefined) setDraft(restoreDraft);
    notify({
      type: "error",
      title: t("app.error.title"),
      message: event.message,
      scope: sessionId ? { kind: "session", sessionId } : { kind: "global" },
    });
    void settingsService.reportClientLogEvent(event).catch(() => undefined);
  }, [notify, t]);
  const reportConfigPersistFailure = useCallback(
    (reason: unknown) => reportChatFailure("MainLayout.saveSessionChatConfig", reason, activeSessionId),
    [activeSessionId, reportChatFailure],
  );
  const chatConfig = useChatConfig({
    activeSession,
    agents,
    onPersistError: reportConfigPersistFailure,
  });
  const invalidateSessions = useCallback(() => {
    void queryClient.invalidateQueries({ queryKey: ["sessions"] });
    void queryClient.invalidateQueries({ queryKey: ["workflow"] });
  }, [queryClient]);
  const invalidateRuntime = useCallback(() => {
    invalidateSessions();
    if (activeSessionId) void queryClient.invalidateQueries({ queryKey: ["messages", activeSessionId] });
  }, [activeSessionId, invalidateSessions, queryClient]);

  const switchSession = useMutation({ mutationFn: (sessionId: string) => agentService.switchSession(sessionId), onSuccess: invalidateSessions });
  const renameSession = useMutation({ mutationFn: ({ sessionId, title }: { sessionId: string; title: string }) => agentService.renameSession(sessionId, title), onSuccess: invalidateSessions });
  const pinSession = useMutation({ mutationFn: (session: Session) => session.pinned ? agentService.unpinSession(session.id) : agentService.pinSession(session.id), onSuccess: invalidateSessions });
  const archiveSession = useMutation({ mutationFn: (session: Session) => session.archived ? agentService.unarchiveSession(session.id) : agentService.archiveSession(session.id), onSuccess: invalidateSessions });
  const deleteSession = useMutation({ mutationFn: (sessionId: string) => agentService.deleteSession(sessionId), onSuccess: invalidateSessions });
  const sendMessage = useMutation({
    mutationFn: (input: { content: string; config: ChatConfig; sessionId: string }) => agentService.sendMessage(input),
    onSuccess: invalidateRuntime,
    onError: (reason, input) => reportChatFailure("MainLayout.sendMessage", reason, input.sessionId, input.content),
  });
  const stopGeneration = useMutation({
    mutationFn: (sessionId: string) => agentService.stopGeneration(sessionId),
    onSuccess: invalidateRuntime,
    onError: (reason, sessionId) => reportChatFailure("MainLayout.stopGeneration", reason, sessionId),
  });

  useEffect(() => {
    if (!activeSessionId) return;
    let cleanup: (() => void) | null = null;
    let cancelled = false;
    void agentService.subscribeMessageEvents(activeSessionId, (event) => {
      queryClient.setQueryData<ChatMessage[]>(messagesKey, (current) => applyChatEvent(current ?? [], event));
      if (["completed", "failed", "cancelled"].includes(event.type)) invalidateSessions();
    }).then((unsubscribe) => { if (cancelled) unsubscribe(); else cleanup = unsubscribe; });
    return () => { cancelled = true; cleanup?.(); };
  }, [activeSessionId, invalidateSessions, messageLimit, queryClient]);

  useEffect(() => { setMessageLimit(50); setDraft(""); }, [activeSessionId]);

  function submit() {
    if (!activeSession || !draft.trim() || isStreaming) return;
    const content = draft.trim();
    setDraft("");
    sendMessage.mutate({ sessionId: activeSession.id, content, config: { ...chatConfig.config, agentId: chatConfig.config.agentId || activeSession.agentId, interactionMode: activeSession.interactionMode } });
  }
  function stop() { if (activeSessionId && isStreaming) stopGeneration.mutate(activeSessionId); }
  function sessionCreated(session: Session) {
    queryClient.setQueryData(["sessions", "active"], session);
    invalidateSessions();
    notify({ type: "success", title: t("notifications.sessionCreated.title"), message: t("notifications.sessionCreated.message", { title: session.title }), scope: { kind: "session", sessionId: session.id } });
  }
  return {
    activeSession, activeSessionId, agents, agentsAvailable: Boolean(agentsQuery.data?.length), archivedSessions: archivedQuery.data ?? [],
    chatConfig, deleteSession: (session: Session) => deleteSession.mutate(session.id), draft, isSending: sendMessage.isPending, isStreaming,
    loadEarlier: () => setMessageLimit((value) => value + 50), messages, messagesPartial: messages.length >= messageLimit,
    pinSession: (session: Session) => pinSession.mutate(session), archiveSession: (session: Session) => archiveSession.mutate(session),
    renameSession: (session: Session, title: string) => renameSession.mutate({ sessionId: session.id, title }),
    sessionCreated, sessions: sessionsQuery.data ?? [], setDraft, stop, submit,
    switchSession: (session: Session) => { if (!session.archived) switchSession.mutate(session.id); },
  };
}
