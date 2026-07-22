import { useCallback, useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useChatConfig } from "../components/chat/hooks/useChatConfig";
import { createChatOperationFailureEvent } from "./chat-operation-failure";
import { useNotifications } from "../notifications/notification-provider";
import { applyChatEvent } from "../services/chat-events";
import { agentService } from "../services/runtime-agent-client";
import { settingsService } from "../services/runtime-settings-client";
import type { Session, SessionCategory, SessionExportFormat } from "../types/agent";
import type { SessionDocument } from "../types/session-workspace";
import type { ChatConfig, ChatFileReference, ChatMessage } from "../types/chat";

export function useMainLayoutModel() {
  const { t } = useTranslation();
  const { notify } = useNotifications();
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState("");
  const [fileReferences, setFileReferences] = useState<ChatFileReference[]>([]);
  const [messageLimit, setMessageLimit] = useState(50);
  const [sessionSearchQuery, setSessionSearchQuery] = useState("");
  const agentsQuery = useQuery({ queryKey: ["agents"], queryFn: () => agentService.listAgents() });
  const sessionsQuery = useQuery({ queryKey: ["sessions"], queryFn: () => agentService.listSessions() });
  const archivedQuery = useQuery({ queryKey: ["sessions", "archived"], queryFn: () => agentService.listArchivedSessions() });
  const categoriesQuery = useQuery({ queryKey: ["session-categories"], queryFn: () => agentService.listSessionCategories() });
  const sessionSearch = useQuery({
    enabled: sessionSearchQuery.trim().length > 0,
    queryKey: ["sessions", "search", sessionSearchQuery],
    queryFn: () => agentService.searchSessions({ query: sessionSearchQuery, limit: 50 }),
  });
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
  const documentsQuery = useQuery({
    enabled: Boolean(activeSessionId),
    queryKey: ["session-documents", activeSessionId],
    queryFn: () => activeSessionId ? agentService.listSessionDocuments(activeSessionId) : Promise.resolve({ context: { availability: "unavailable" as const, rootName: null, reason: null }, items: [], truncated: false, nextCursor: null }),
  });
  const fileReferenceCandidates = documentsQuery.data?.items ?? [];
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
    void queryClient.invalidateQueries({ queryKey: ["session-categories"] });
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
  const deleteSessions = useMutation({
    mutationFn: async (targets: Session[]) => {
      const results = await Promise.allSettled(targets.map((session) => agentService.deleteSession(session.id)));
      return {
        deleted: results.filter((result) => result.status === "fulfilled").length,
        failed: results.filter((result) => result.status === "rejected").length,
      };
    },
    onSuccess: (result) => {
      if (result.deleted > 0) notify({ type: "success", title: t("layout.batchDeleteSuccessTitle"), message: t("layout.batchDeleteSuccessMessage", { count: result.deleted }), scope: { kind: "global" } });
      if (result.failed > 0) notify({ type: "error", title: t("layout.batchDeleteFailedTitle"), message: t("layout.batchDeleteFailedMessage", { count: result.failed }), scope: { kind: "global" } });
    },
    onSettled: invalidateSessions,
  });
  const createCategory = useMutation({ mutationFn: (name: string) => agentService.createSessionCategory({ name }), onSuccess: invalidateSessions });
  const assignCategory = useMutation({ mutationFn: ({ session, categoryId }: { session: Session; categoryId: string | null }) => agentService.assignSessionCategory({ sessionId: session.id, categoryId }), onSuccess: invalidateSessions });
  const exportSession = useMutation({
    mutationFn: ({ session, format }: { session: Session; format: SessionExportFormat }) => agentService.exportSession({ sessionId: session.id, format }),
    onSuccess: (result, input) => {
      if (result.status === "exported") {
        notify({ type: "success", title: t("notifications.sessionExported.title"), message: t("notifications.sessionExported.message", { title: input.session.title, path: result.path ?? "" }), scope: { kind: "session", sessionId: input.session.id } });
        return;
      }
      notify({ type: "warning", title: t("notifications.sessionExportCancelled.title"), message: t("notifications.sessionExportCancelled.message"), scope: { kind: "session", sessionId: input.session.id } });
    },
    onError: (reason, input) => reportChatFailure("MainLayout.exportSession", reason, input.session.id),
  });
  const sendMessage = useMutation({
    mutationFn: (input: { content: string; config: ChatConfig; fileReferences: ChatFileReference[]; sessionId: string }) => agentService.sendMessage(input),
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
      if (event.type === "completed" && event.tokenUsage) {
        void queryClient.invalidateQueries({ queryKey: ["session-usage-summary", event.sessionId] });
        void queryClient.invalidateQueries({ queryKey: ["usage-statistics"] });
      }
      if (["completed", "failed", "cancelled"].includes(event.type)) invalidateSessions();
    }).then((unsubscribe) => { if (cancelled) unsubscribe(); else cleanup = unsubscribe; });
    return () => { cancelled = true; cleanup?.(); };
  }, [activeSessionId, invalidateSessions, messageLimit, queryClient]);

  useEffect(() => { setMessageLimit(50); setDraft(""); setFileReferences([]); }, [activeSessionId]);

  function submit() {
    if (!activeSession || !draft.trim() || isStreaming) return;
    const content = draft.trim();
    const references = fileReferences;
    setDraft("");
    setFileReferences([]);
    sendMessage.mutate({ sessionId: activeSession.id, content, fileReferences: references, config: { ...chatConfig.config, agentId: chatConfig.config.agentId || activeSession.agentId, interactionMode: activeSession.interactionMode } });
  }
  function stop() { if (activeSessionId && isStreaming) stopGeneration.mutate(activeSessionId); }
  function sessionCreated(session: Session) {
    queryClient.setQueryData(["sessions", "active"], session);
    invalidateSessions();
    notify({ type: "success", title: t("notifications.sessionCreated.title"), message: t("notifications.sessionCreated.message", { title: session.title }), scope: { kind: "session", sessionId: session.id } });
  }
  return {
    activeSession, activeSessionId, agents, agentsAvailable: Boolean(agentsQuery.data?.length), archivedSessions: archivedQuery.data ?? [],
    assignCategory: (session: Session, categoryId: string | null) => assignCategory.mutate({ session, categoryId }),
    categories: categoriesQuery.data ?? [],
    chatConfig,
    createCategory: async (name: string): Promise<SessionCategory> => createCategory.mutateAsync(name),
    deleteSession: (session: Session) => deleteSession.mutate(session.id),
    deleteSessions: (selectedSessions: Session[]) => deleteSessions.mutate(selectedSessions),
    deletingSessions: deleteSessions.isPending,
    draft,
    exportSession: (session: Session, format: SessionExportFormat) => exportSession.mutate({ session, format }),
    fileReferenceCandidates,
    fileReferences,
    isSending: sendMessage.isPending, isStreaming,
    loadEarlier: () => setMessageLimit((value) => value + 50), messages, messagesPartial: messages.length >= messageLimit,
    pinSession: (session: Session) => pinSession.mutate(session), archiveSession: (session: Session) => archiveSession.mutate(session),
    renameSession: (session: Session, title: string) => renameSession.mutate({ sessionId: session.id, title }),
    sessionCreated,
    sessionSearchQuery,
    sessionSearchResults: sessionSearch.data ?? [],
    sessions: sessionsQuery.data ?? [],
    setDraft,
    addFileReference: (document: SessionDocument) => setFileReferences((current) => current.some((reference) => reference.path === document.path) ? current : [...current, { id: document.path, path: document.path, name: document.name }]),
    removeFileReference: (path: string) => setFileReferences((current) => current.filter((reference) => reference.path !== path)),
    setSessionSearchQuery,
    stop, submit,
    switchSession: (session: Session) => { if (!session.archived) switchSession.mutate(session.id); },
  };
}
