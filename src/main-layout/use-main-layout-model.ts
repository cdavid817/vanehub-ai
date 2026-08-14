import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useChatConfig } from "../components/chat/hooks/useChatConfig";
import { createChatOperationFailureEvent } from "./chat-operation-failure";
import { useNotifications } from "../notifications/notification-provider";
import { normalizeDisplayPath } from "../lib/session-path";
import { useDebouncedValue } from "../hooks/use-debounced-value";
import type { TurnStatus } from "../components/chat/TurnStatusBar";
import { turnStatusFromEvent, waitedMinutes } from "../services/turn-status";
import { applyChatEvents } from "../services/chat-events";
import { agentService } from "../services/runtime-agent-client";
import { permissionsService } from "../services/runtime-permissions-client";
import { settingsService } from "../services/runtime-settings-client";
import type { Session, SessionCategory, SessionExportFormat } from "../types/agent";
import type { FileSearchMatch } from "../types/session-workspace";
import { useMentionCandidates } from "./use-mention-candidates";
import type { ChatFileReference, ChatMessage, ChatStreamEvent } from "../types/chat";
import { appendMessageIfMissing, createOptimisticUserMessage, removeMessageById, type SendMessageMutationInput } from "./optimistic-message";
import { canSendToSession, hasLiveSessionGeneration } from "../services/session-admission";
import { useSessionRecoverySync } from "./use-session-recovery-sync";
import { useSessionSwitch } from "./use-session-switch";
export function useMainLayoutModel() {
  const { t } = useTranslation();
  const { notify } = useNotifications();
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState("");
  const [fileReferences, setFileReferences] = useState<ChatFileReference[]>([]);
  const [messageLimit, setMessageLimit] = useState(50);
  const [turnStatus, setTurnStatus] = useState<TurnStatus | null>(null);
  const waitingSince = useRef<string | null>(null);
  const optimisticMessageSequence = useRef(0);
  const activeSessionIdRef = useRef<string | null>(null);
  const [sessionSearchQuery, setSessionSearchQuery] = useState("");
  const normalizedSessionSearchQuery = sessionSearchQuery.trim();
  const debouncedSessionSearchQuery = useDebouncedValue(normalizedSessionSearchQuery, 250);
  const agentsQuery = useQuery({ queryKey: ["agents"], queryFn: () => agentService.listAgents() });
  const sessionsQuery = useQuery({ queryKey: ["sessions"], queryFn: () => agentService.listSessions() });
  const archivedQuery = useQuery({ queryKey: ["sessions", "archived"], queryFn: () => agentService.listArchivedSessions() });
  const categoriesQuery = useQuery({ queryKey: ["session-categories"], queryFn: () => agentService.listSessionCategories() });
  const sessionSearch = useQuery({
    enabled: debouncedSessionSearchQuery.length >= 2,
    queryKey: ["sessions", "search", debouncedSessionSearchQuery],
    queryFn: () => agentService.searchSessions({ query: debouncedSessionSearchQuery, limit: 50 }),
  });
  const activeQuery = useQuery({ queryKey: ["sessions", "active"], queryFn: () => agentService.getActiveSession() });
  const agents = agentsQuery.data ?? [];
  const activeSession = activeQuery.data ?? null;
  const activeSessionId = activeSession?.id ?? null;
  activeSessionIdRef.current = activeSessionId;
  const messagesKey = useMemo(
    () => ["messages", activeSessionId, messageLimit] as const,
    [activeSessionId, messageLimit],
  );
  const messagesQuery = useQuery({
    enabled: Boolean(activeSessionId), queryKey: messagesKey,
    queryFn: () => activeSessionId ? agentService.listMessages({ sessionId: activeSessionId, limit: messageLimit }) : Promise.resolve([]),
  });
  const messages = messagesQuery.data ?? [];
  const fileReferenceCandidates = useMentionCandidates(activeSessionId, draft);
  const isStreaming = hasLiveSessionGeneration(activeSession, messages);
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
  const reportRecoveryFailure = useCallback(
    (reason: unknown, sessionId: string) => reportChatFailure(
      "MainLayout.acknowledgeSessionRecovery",
      reason,
      sessionId,
    ),
    [reportChatFailure],
  );
  const recoverySync = useSessionRecoverySync({ activeSession, onAcknowledgementError: reportRecoveryFailure });
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
  const switchSession = useSessionSwitch({ activeSessionId, onError: (reason, sessionId) => reportChatFailure("MainLayout.switchSession", reason, sessionId) });
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
        notify({ type: "success", title: t("notifications.sessionExported.title"), message: t("notifications.sessionExported.message", { title: input.session.title, path: result.path ? normalizeDisplayPath(result.path) : "" }), scope: { kind: "session", sessionId: input.session.id } });
        return;
      }
      notify({ type: "warning", title: t("notifications.sessionExportCancelled.title"), message: t("notifications.sessionExportCancelled.message"), scope: { kind: "session", sessionId: input.session.id } });
    },
    onError: (reason, input) => reportChatFailure("MainLayout.exportSession", reason, input.session.id),
  });
  const sendMessage = useMutation({
    mutationFn: (input: SendMessageMutationInput) => agentService.sendMessage(input),
    onMutate: async (input) => {
      await queryClient.cancelQueries({ queryKey: ["messages", input.sessionId] });
      optimisticMessageSequence.current += 1;
      const optimisticId = `optimistic:${input.sessionId}:${optimisticMessageSequence.current}`;
      const optimisticMessage = createOptimisticUserMessage({
        content: input.content,
        fileReferences: input.fileReferences,
        id: optimisticId,
        sessionId: input.sessionId,
      });
      queryClient.setQueriesData<ChatMessage[]>({ queryKey: ["messages", input.sessionId] }, (current) =>
        appendMessageIfMissing(current, optimisticMessage));
      return { optimisticId };
    },
    onSuccess: (assistant, input) => {
      queryClient.setQueriesData<ChatMessage[]>({ queryKey: ["messages", input.sessionId] }, (current) =>
        appendMessageIfMissing(current, assistant));
      invalidateRuntime();
    },
    onError: (reason, input, context) => {
      if (context?.optimisticId) {
        queryClient.setQueriesData<ChatMessage[]>({ queryKey: ["messages", input.sessionId] }, (current) =>
          removeMessageById(current, context.optimisticId));
      }
      if (activeSessionIdRef.current === input.sessionId) {
        setDraft(input.content);
        setFileReferences(input.fileReferences);
      }
      reportChatFailure("MainLayout.sendMessage", reason, input.sessionId);
    },
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
    // Token events arrive in the thousands per turn; buffer them and flush on an animation
    // frame so the message array is rebuilt once per frame instead of once per token.
    let pending: ChatStreamEvent[] = [];
    let frame = 0;
    const flush = () => {
      frame = 0;
      if (pending.length === 0) return;
      const batch = pending;
      pending = [];
      queryClient.setQueryData<ChatMessage[]>(messagesKey, (current) =>
        applyChatEvents(current ?? [], batch),
      );
    };
    void agentService.subscribeMessageEvents(activeSessionId, (event) => {
      if (event.type === "turn_status") {
        // turn_status is session-scoped, not message-scoped, and drives the waiting bar; it
        // must update immediately rather than ride a frame delay.
        waitingSince.current = event.status.kind === "waiting_human" ? event.status.since : null;
        setTurnStatus(turnStatusFromEvent(event.status));
        return;
      }
      pending.push(event);
      if (event.type === "completed" && event.tokenUsage) {
        void queryClient.invalidateQueries({ queryKey: ["session-usage-summary", event.sessionId] });
        void queryClient.invalidateQueries({ queryKey: ["usage-statistics"] });
      }
      // A round that ends leaves nobody holding the turn, so the bar has to go rather than freeze.
      if (["completed", "failed", "cancelled"].includes(event.type)) {
        invalidateSessions();
        // Flush the buffered tokens now so the terminal message lands with the status change.
        if (frame !== 0) { cancelAnimationFrame(frame); frame = 0; }
        flush();
      } else if (frame === 0) {
        frame = requestAnimationFrame(flush);
      }
    }).then((unsubscribe) => { if (cancelled) unsubscribe(); else cleanup = unsubscribe; });
    return () => {
      cancelled = true;
      if (frame !== 0) cancelAnimationFrame(frame);
      cleanup?.();
    };
  }, [activeSessionId, invalidateSessions, messagesKey, queryClient]);
  useEffect(() => { setMessageLimit(50); setDraft(""); setFileReferences([]); setTurnStatus(null); waitingSince.current = null; }, [activeSessionId]);
  // Keep a human-wait duration moving without backend minute-by-minute events.
  useEffect(() => {
    if (turnStatus?.kind !== "waiting-human") return;
    const timer = setInterval(() => {
      const since = waitingSince.current;
      if (!since) return;
      setTurnStatus((current) =>
        current?.kind === "waiting-human"
          ? { ...current, waitedMinutes: waitedMinutes(since, new Date()) }
          : current,
      );
    }, 30_000);
    return () => clearInterval(timer);
  }, [turnStatus?.kind]);
  useEffect(() => {
    // Global so pending approvals remain visible without opening their owning session.
    let cleanup: (() => void) | null = null;
    let cancelled = false;
    void permissionsService.subscribePendingApprovalEvents((event) => {
      notify({
        type: "warning",
        title: t("notifications.pendingApproval.title"),
        message: t("notifications.pendingApproval.message", { agentId: event.agentId, action: event.action }),
        scope: { kind: "global" },
      });
    }).then((unsubscribe) => { if (cancelled) unsubscribe(); else cleanup = unsubscribe; });
    return () => { cancelled = true; cleanup?.(); };
  }, [notify, t]);
  function submit() {
    if (!canSendToSession(activeSession) || !activeSession || !draft.trim() || isStreaming) return;
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
    activeSession, activeSessionId, agents, agentsAvailable: Boolean(agentsQuery.data?.length), archivedSessions: archivedQuery.data ?? [], turnStatus,
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
    recoverySummary: recoverySync.recoverySummary,
    acknowledgeRecovery: recoverySync.acknowledgeRecovery,
    acknowledgingRecovery: recoverySync.acknowledgingRecovery,
    renameSession: (session: Session, title: string) => renameSession.mutate({ sessionId: session.id, title }),
    sessionCreated,
    sessionSearchQuery,
    sessionSearchResults: normalizedSessionSearchQuery === debouncedSessionSearchQuery
      ? sessionSearch.data ?? []
      : [],
    sessions: sessionsQuery.data ?? [],
    setDraft,
    addFileReference: (candidate: FileSearchMatch) => setFileReferences((current) => current.some((reference) => reference.path === candidate.path) ? current : [...current, { id: candidate.path, path: candidate.path, name: candidate.name }]),
    removeFileReference: (path: string) => setFileReferences((current) => current.filter((reference) => reference.path !== path)),
    setSessionSearchQuery,
    stop, submit,
    switchSession,
  };
}

export type MainLayoutModel = ReturnType<typeof useMainLayoutModel>;
