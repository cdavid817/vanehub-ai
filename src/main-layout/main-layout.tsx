import { useEffect, useMemo, useState, type MouseEvent, type ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Archive, Bot, BrainCircuit, CheckCircle2, ChevronDown, ChevronRight, CircleDot, Clock3, Code2, FileText, Folder, GitBranch, HelpCircle, PanelRightClose, PanelRightOpen, Pin, Plus, RotateCcw, Settings, Sparkles, TerminalSquare, type LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ChatInputBox } from "../components/chat/ChatInputBox";
import { useChatConfig } from "../components/chat/hooks/useChatConfig";
import { MessageList } from "../components/chat/MessageList";
import { Button } from "../components/ui/button";
import { cn } from "../lib/utils";
import { agentService } from "../services/runtime-agent-client";
import type { Session } from "../types/agent";
import type { ChatConfig, ChatMessage as ChatMessageModel, ChatStreamEvent } from "../types/chat";
import { CreateSessionDialog } from "./create-session-dialog";
import { StatusBar } from "./status-bar";
import { TopBar } from "./top-bar";

type SidebarMode = "activity" | "group" | "archived";
type ActivityKey = "needs-input" | "pending-verification" | "in-progress" | "inactive";
type AgentKey = "codex" | "claude-code" | "opencode" | "gemini" | "unknown";
type InfoTab = "agent" | "files" | "changes";
type ContextPanelMode = "menu" | "rename" | "delete";
type ContextPanelState = {
  session: Session;
  mode: ContextPanelMode;
  draftTitle: string;
  x: number;
  y: number;
};

const activityGroups: Array<{ key: ActivityKey; labelKey: string }> = [
  { key: "needs-input", labelKey: "layout.needsInput" },
  { key: "pending-verification", labelKey: "layout.pendingVerification" },
  { key: "in-progress", labelKey: "layout.running" },
  { key: "inactive", labelKey: "layout.inactive" },
];

const agentMeta: Record<AgentKey, { label: string; Icon: LucideIcon; tone: string }> = {
  codex: { label: "Codex", Icon: Code2, tone: "ucd-agent-codex" },
  "claude-code": { label: "Claude Code", Icon: Sparkles, tone: "ucd-agent-claude" },
  opencode: { label: "OpenCode", Icon: TerminalSquare, tone: "ucd-agent-opencode" },
  gemini: { label: "Gemini", Icon: BrainCircuit, tone: "ucd-agent-gemini" },
  unknown: { label: "Agent", Icon: Bot, tone: "border-border bg-muted text-muted-foreground" },
};

const infoTabs: Array<{ key: InfoTab; labelKey: string }> = [
  { key: "agent", labelKey: "layout.infoTab.agent" },
  { key: "files", labelKey: "layout.infoTab.files" },
  { key: "changes", labelKey: "layout.infoTab.changes" },
];

function getActivityKeyForSession(session: Session): ActivityKey {
  if (session.archived || session.lifecycleState === "idle" || session.lifecycleState === "stopped") return "inactive";
  if (session.lifecycleState === "failed") return "needs-input";
  if (session.lifecycleState === "starting") return "pending-verification";
  return "in-progress";
}

function getSessionFolder(session: Session, fallback: string) {
  return session.folder ?? fallback;
}

function getAgentKeyForSession(session: Session): AgentKey {
  if (session.agentId.includes("codex")) return "codex";
  if (session.agentId.includes("claude")) return "claude-code";
  if (session.agentId.includes("opencode")) return "opencode";
  if (session.agentId.includes("gemini")) return "gemini";
  return "unknown";
}

function formatSessionDate(session: Session, language: string) {
  return new Intl.DateTimeFormat(language, { month: "2-digit", day: "2-digit" }).format(new Date(session.updatedAt));
}

function formatLifecycle(session: Session, t: (key: string) => string) {
  const labels: Record<Session["lifecycleState"], string> = {
    failed: t("layout.needsInput"),
    idle: t("layout.idle"),
    running: t("layout.running"),
    starting: t("layout.pendingVerification"),
    stopped: t("layout.stopped"),
  };
  return session.archived ? t("layout.archived") : labels[session.lifecycleState];
}

export function ConversationCard({
  active,
  lifecycleLabel,
  language,
  onContextMenu,
  onSelect,
  session,
  sourceLabel,
}: {
  active: boolean;
  lifecycleLabel: string;
  language: string;
  onContextMenu: (event: MouseEvent<HTMLButtonElement>) => void;
  onSelect: () => void;
  session: Session;
  sourceLabel?: string;
}) {
  const meta = agentMeta[getAgentKeyForSession(session)];
  const Icon = meta.Icon;

  return (
    <button className={cn("ucd-list-row relative w-full rounded-lg p-2.5 text-left", active && "border-primary bg-[hsl(var(--nav-active-soft))]")} onClick={onSelect} onContextMenu={onContextMenu} type="button">
      {active ? <span className="absolute left-0 top-2 h-10 w-0.5 rounded bg-primary" /> : null}
      <div className="flex min-w-0 items-center gap-2">
        <span className={cn("flex h-6 w-6 shrink-0 items-center justify-center rounded border", meta.tone)} title={meta.label}>
          <Icon className="h-3.5 w-3.5" aria-hidden="true" />
        </span>
        <span className={cn("truncate text-sm font-medium", session.archived && "text-muted-foreground")}>{session.title}</span>
        {session.pinned ? <Pin className="ml-auto h-3.5 w-3.5 text-primary" aria-hidden="true" /> : null}
      </div>
      <div className="mt-2 flex min-w-0 items-center gap-2 overflow-hidden text-xs text-muted-foreground">
        <span className={cn("h-2 w-2 rounded-full", session.archived ? "bg-muted-foreground" : "bg-[hsl(var(--success))]")} />
        <span>{lifecycleLabel}</span>
        <span className="shrink-0 font-mono">{meta.label}</span>
        {sourceLabel ? <span className="h-5 shrink-0 rounded-sm border border-border bg-muted px-1.5 leading-[18px] text-foreground">{sourceLabel}</span> : null}
        <span className="ml-auto font-mono">{formatSessionDate(session, language)}</span>
      </div>
    </button>
  );
}

function KeepAlivePane({ active, children }: { active: boolean; children: ReactNode }) {
  return <div className={cn("h-full", active ? "block" : "hidden")}>{children}</div>;
}

function applyChatEvent(messages: ChatMessageModel[], event: ChatStreamEvent) {
  return messages.map((message) => {
    if (message.id !== event.messageId) return message;
    const timestamp = new Date().toISOString();
    if (event.type === "token") {
      return { ...message, content: `${message.content}${event.contentDelta}`, updatedAt: timestamp };
    }
    if (event.type === "thinking") {
      return {
        ...message,
        thinkingContent: `${message.thinkingContent ?? ""}${event.contentDelta}`,
        updatedAt: timestamp,
      };
    }
    if (event.type === "tool_use") {
      return { ...message, toolUse: [...(message.toolUse ?? []), event.toolUse], updatedAt: timestamp };
    }
    if (event.type === "completed") {
      return { ...message, status: "completed" as const, tokenUsage: event.tokenUsage, updatedAt: timestamp };
    }
    if (event.type === "failed") {
      return { ...message, status: "failed" as const, error: event.error, updatedAt: timestamp };
    }
    if (event.type === "cancelled") {
      return { ...message, status: "cancelled" as const, updatedAt: timestamp };
    }
    return message;
  });
}

export function MainLayout({ onOpenSettings }: { onOpenSettings: () => void }) {
  const { i18n, t } = useTranslation();
  const [sidebarMode, setSidebarMode] = useState<SidebarMode>("activity");
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(() => new Set(["Current Workspace", "Engineering", "Content"]));
  const [activeInfoTab, setActiveInfoTab] = useState<InfoTab>("agent");
  const [infoPanelCollapsed, setInfoPanelCollapsed] = useState(false);
  const [contextPanel, setContextPanel] = useState<ContextPanelState | null>(null);
  const [createSessionOpen, setCreateSessionOpen] = useState(false);
  const [chatDraft, setChatDraft] = useState("");
  const [messageLimit, setMessageLimit] = useState(50);
  const queryClient = useQueryClient();

  const agentsQuery = useQuery({
    queryKey: ["agents"],
    queryFn: () => agentService.listAgents(),
  });
  const sessionsQuery = useQuery({
    queryKey: ["sessions"],
    queryFn: () => agentService.listSessions(),
  });
  const archivedSessionsQuery = useQuery({
    queryKey: ["sessions", "archived"],
    queryFn: () => agentService.listArchivedSessions(),
  });
  const activeSessionQuery = useQuery({
    queryKey: ["sessions", "active"],
    queryFn: () => agentService.getActiveSession(),
  });
  const sessions = sessionsQuery.data ?? [];
  const agents = agentsQuery.data ?? [];
  const archivedSessions = archivedSessionsQuery.data ?? [];
  const activeSession = activeSessionQuery.data ?? null;
  const activeSessionId = activeSession?.id ?? null;
  const messagesQueryKey = ["messages", activeSessionId, messageLimit] as const;
  const messagesQuery = useQuery({
    enabled: Boolean(activeSessionId),
    queryKey: messagesQueryKey,
    queryFn: () => {
      if (!activeSessionId) return Promise.resolve([]);
      return agentService.listMessages({ sessionId: activeSessionId, limit: messageLimit });
    },
  });
  const chatMessages = messagesQuery.data ?? [];
  const isStreaming = chatMessages.some((message) => message.status === "streaming");
  const chatConfig = useChatConfig({ activeSession, agents });

  function invalidateSessions() {
    void queryClient.invalidateQueries({ queryKey: ["sessions"] });
    void queryClient.invalidateQueries({ queryKey: ["workflow"] });
  }

  function invalidateSessionRuntime() {
    invalidateSessions();
    invalidateActiveMessages();
  }

  function invalidateActiveMessages() {
    if (!activeSessionId) return;
    void queryClient.invalidateQueries({ queryKey: ["messages", activeSessionId] });
  }

  const switchSessionMutation = useMutation({
    mutationFn: (sessionId: string) => agentService.switchSession(sessionId),
    onSuccess: invalidateSessions,
  });
  const renameSessionMutation = useMutation({
    mutationFn: ({ sessionId, title }: { sessionId: string; title: string }) => agentService.renameSession(sessionId, title),
    onSuccess: invalidateSessions,
  });
  const pinSessionMutation = useMutation({
    mutationFn: (session: Session) => (session.pinned ? agentService.unpinSession(session.id) : agentService.pinSession(session.id)),
    onSuccess: invalidateSessions,
  });
  const archiveSessionMutation = useMutation({
    mutationFn: (session: Session) => (session.archived ? agentService.unarchiveSession(session.id) : agentService.archiveSession(session.id)),
    onSuccess: invalidateSessions,
  });
  const deleteSessionMutation = useMutation({
    mutationFn: (sessionId: string) => agentService.deleteSession(sessionId),
    onSuccess: invalidateSessions,
  });
  const sendMessageMutation = useMutation({
    mutationFn: (input: { content: string; config: ChatConfig; sessionId: string }) =>
      agentService.sendMessage(input),
    onSuccess: invalidateSessionRuntime,
  });
  const stopGenerationMutation = useMutation({
    mutationFn: (sessionId: string) => agentService.stopGeneration(sessionId),
    onSuccess: invalidateSessionRuntime,
  });

  const activityBuckets = useMemo(
    () =>
      activityGroups.map((group) => ({
        ...group,
        sessions: sessions.filter((session) => !session.pinned && getActivityKeyForSession(session) === group.key),
      })),
    [sessions],
  );
  const folderBuckets = useMemo(() => {
    const groups = new Map<string, Session[]>();
    sessions.filter((session) => !session.pinned).forEach((session) => {
      const folder = getSessionFolder(session, t("layout.currentWorkspace"));
      groups.set(folder, [...(groups.get(folder) ?? []), session]);
    });
    return Array.from(groups.entries()).map(([folder, groupedSessions]) => ({ folder, sessions: groupedSessions }));
  }, [sessions, t]);
  const pinnedSessions = useMemo(() => sessions.filter((session) => session.pinned), [sessions]);
  const progressStats = { complete: 6, running: 3, pending: 4 };
  const infoFiles = ["src/main-layout/main-layout.tsx", "openspec/changes/improve-main-layout-ui/tasks.md", "openspec/changes/improve-main-layout-ui/design.md"];
  const changeItems = ["Sidebar tool entry migration", "Info panel collapse and keep-alive", "Flexible main content layout"];
  const progressTotal = progressStats.complete + progressStats.running + progressStats.pending;
  const progressPercent = Math.round((progressStats.complete / progressTotal) * 100);

  useEffect(() => {
    if (!activeSessionId) return;
    let cleanup: (() => void) | null = null;
    let cancelled = false;
    void agentService.subscribeMessageEvents(activeSessionId, (event) => {
      queryClient.setQueryData<ChatMessageModel[]>(["messages", activeSessionId, messageLimit], (currentMessages) =>
        applyChatEvent(currentMessages ?? [], event),
      );
      if (event.type === "completed" || event.type === "failed" || event.type === "cancelled") {
        invalidateSessions();
      }
    }).then((unsubscribe) => {
      if (cancelled) unsubscribe();
      else cleanup = unsubscribe;
    });
    return () => {
      cancelled = true;
      cleanup?.();
    };
  }, [activeSessionId, messageLimit, queryClient]);

  useEffect(() => {
    setMessageLimit(50);
  }, [activeSessionId]);

  function toggleFolder(folder: string) {
    setExpandedFolders((current) => {
      const next = new Set(current);
      if (next.has(folder)) next.delete(folder);
      else next.add(folder);
      return next;
    });
  }

  function openContextMenu(event: MouseEvent<HTMLButtonElement>, session: Session) {
    event.preventDefault();
    const menuWidth = 160;
    const menuHeight = 148;
    const gap = 8;
    const x = Math.min(event.clientX + gap, window.innerWidth - menuWidth - gap);
    const y = Math.min(event.clientY + gap, window.innerHeight - menuHeight - gap);
    setContextPanel({
      session,
      mode: "menu",
      draftTitle: session.title,
      x: Math.max(gap, x),
      y: Math.max(gap, y),
    });
  }

  function submitRename() {
    if (!contextPanel) return;
    const nextTitle = contextPanel.draftTitle.trim();
    if (!nextTitle || nextTitle === contextPanel.session.title) {
      setContextPanel(null);
      return;
    }
    renameSessionMutation.mutate({ sessionId: contextPanel.session.id, title: nextTitle });
    setContextPanel(null);
  }

  function confirmDelete() {
    if (!contextPanel) return;
    deleteSessionMutation.mutate(contextPanel.session.id);
    setContextPanel(null);
  }

  function submitChatMessage() {
    if (!activeSession || !chatDraft.trim() || isStreaming) return;
    const content = chatDraft.trim();
    setChatDraft("");
    sendMessageMutation.mutate({
      sessionId: activeSession.id,
      content,
      config: {
        ...chatConfig.config,
        agentId: chatConfig.config.agentId || activeSession.agentId,
        interactionMode: activeSession.interactionMode,
      },
    });
  }

  function stopChatGeneration() {
    if (!activeSessionId || !isStreaming) return;
    stopGenerationMutation.mutate(activeSessionId);
  }

  function handleSessionCreated(session: Session) {
    setCreateSessionOpen(false);
    queryClient.setQueryData(["sessions", "active"], session);
    invalidateSessions();
  }

  function renderContextMenu() {
    if (!contextPanel || contextPanel.mode !== "menu") return null;
    const session = contextPanel.session;
    return (
      <div className="ucd-panel fixed z-50 grid w-40 gap-1 rounded-md p-1 text-sm shadow-lg" style={{ left: contextPanel.x, top: contextPanel.y }}>
        <button className="rounded px-2 py-1.5 text-left hover:bg-muted" onClick={() => setContextPanel({ ...contextPanel, mode: "rename" })} type="button">{t("layout.rename")}</button>
        <button className="rounded px-2 py-1.5 text-left hover:bg-muted" onClick={() => { pinSessionMutation.mutate(session); setContextPanel(null); }} type="button">
          {session.pinned ? t("layout.unpin") : t("layout.pinned")}
        </button>
        <button className="rounded px-2 py-1.5 text-left hover:bg-muted" onClick={() => { archiveSessionMutation.mutate(session); setContextPanel(null); }} type="button">
          {session.archived ? <><RotateCcw className="mr-1 inline h-3.5 w-3.5" />{t("layout.restore")}</> : t("layout.archive")}
        </button>
        <button className="rounded px-2 py-1.5 text-left text-destructive hover:bg-muted" onClick={() => setContextPanel({ ...contextPanel, mode: "delete" })} type="button">{t("layout.delete")}</button>
      </div>
    );
  }

  function renderCenteredDialog() {
    if (!contextPanel || contextPanel.mode === "menu") return null;
    if (contextPanel.mode === "rename") {
      return (
        <div className="fixed inset-0 z-50 grid place-items-center bg-background/60 p-4">
          <form className="ucd-panel grid w-full max-w-sm gap-3 rounded-lg p-4 text-sm shadow-xl" onSubmit={(event) => { event.preventDefault(); submitRename(); }}>
            <div>
              <h3 className="text-sm font-semibold">{t("layout.renameSession")}</h3>
              <p className="mt-1 text-xs text-muted-foreground">{t("layout.renameDescription")}</p>
            </div>
            <label className="grid gap-1">
              <span className="text-xs text-muted-foreground">{t("layout.sessionName")}</span>
              <input
                autoFocus
                className="ucd-input h-9 rounded px-2 outline-none focus-visible:ring-2 focus-visible:ring-ring"
                onChange={(event) => setContextPanel({ ...contextPanel, draftTitle: event.target.value })}
                value={contextPanel.draftTitle}
              />
            </label>
            <div className="grid grid-cols-2 gap-2">
              <button className="h-8 rounded border border-border text-xs hover:bg-muted" onClick={() => setContextPanel(null)} type="button">{t("layout.cancel")}</button>
              <button className="h-8 rounded bg-primary text-xs text-primary-foreground disabled:opacity-50" disabled={!contextPanel.draftTitle.trim()} type="submit">{t("layout.confirm")}</button>
            </div>
          </form>
        </div>
      );
    }
    return (
      <div className="fixed inset-0 z-50 grid place-items-center bg-background/60 p-4">
        <div className="ucd-panel grid w-full max-w-sm gap-3 rounded-lg p-4 text-sm shadow-xl">
          <div>
            <h3 className="text-sm font-semibold">{t("layout.deleteSession")}</h3>
            <p className="mt-1 break-words text-xs text-muted-foreground">"{contextPanel.session.title}" {t("layout.deleteDescription")}</p>
          </div>
          <div className="grid grid-cols-2 gap-2">
            <button className="h-8 rounded border border-border text-xs hover:bg-muted" onClick={() => setContextPanel(null)} type="button">{t("layout.cancel")}</button>
            <button className="h-8 rounded bg-destructive text-xs text-destructive-foreground" onClick={confirmDelete} type="button">{t("layout.delete")}</button>
          </div>
        </div>
      </div>
    );
  }

  function renderSessionCard(session: Session) {
    return (
      <ConversationCard
        active={activeSessionId === session.id}
        key={session.id}
        language={i18n.language}
        lifecycleLabel={formatLifecycle(session, t)}
        onContextMenu={(event) => openContextMenu(event, session)}
        onSelect={() => {
          setContextPanel(null);
          if (!session.archived) switchSessionMutation.mutate(session.id);
        }}
        session={session}
        sourceLabel={session.source?.kind === "im" && session.source.connector ? t(`layout.imSource.${session.source.connector}`) : undefined}
      />
    );
  }

  return (
    <main className="min-h-screen bg-background text-foreground">
      <div className="pointer-events-none fixed inset-0 opacity-[0.035] [background-image:linear-gradient(hsl(var(--primary))_1px,transparent_1px),linear-gradient(90deg,hsl(var(--primary))_1px,transparent_1px)] [background-size:100px_100px]" />
      <div className="relative flex h-screen min-h-0 flex-col overflow-hidden">
        <TopBar />
        <div
          className="ucd-workspace-grid relative grid min-h-0 flex-1 gap-4 p-2 transition-[grid-template-columns] duration-200 max-[900px]:gap-2"
          data-info-collapsed={infoPanelCollapsed ? "true" : "false"}
        >
          <aside className="ucd-panel flex min-h-0 flex-col rounded-lg p-3 max-[640px]:max-h-64" onContextMenu={(event) => event.preventDefault()}>
            <div className="mb-3 flex items-center justify-between gap-2">
              <h2 className="text-sm font-semibold">{t("layout.sessions")}</h2>
              <Button className="h-7 px-2 text-xs" disabled={!agentsQuery.data?.length} onClick={() => setCreateSessionOpen(true)}>
                <Plus className="h-3.5 w-3.5" aria-hidden="true" />{t("layout.new")}
              </Button>
            </div>
            <div className="ucd-segmented mb-3 grid grid-cols-3 gap-1 rounded-md p-1">
                {[["activity", t("layout.activity")], ["group", t("layout.group")], ["archived", `${t("layout.archive")} ${archivedSessions.length}`]].map(([key, label]) => (
                <button className={cn("h-7 rounded text-xs", sidebarMode === key ? "bg-background font-semibold text-primary" : "text-muted-foreground hover:bg-muted")} key={key} onClick={() => setSidebarMode(key as SidebarMode)} type="button">
                  {label}
                </button>
              ))}
            </div>
            <div className="min-h-0 flex-1 overflow-y-auto pr-1">
              {sidebarMode !== "archived" && pinnedSessions.length > 0 ? (
                <section className="mb-3 grid gap-2 border-b border-border pb-3">
                  <div className="flex items-center justify-between text-xs text-muted-foreground">
                    <span className="inline-flex items-center gap-1"><Pin className="h-3.5 w-3.5" aria-hidden="true" />{t("layout.pinned")}</span>
                    <span className="rounded-full border border-border px-1.5 font-mono">{pinnedSessions.length}</span>
                  </div>
                  {pinnedSessions.map(renderSessionCard)}
                </section>
              ) : null}
              {sidebarMode === "activity" ? (
                <div className="grid gap-3">
                  {activityBuckets.map((group) => (
                    <section className="grid gap-2" key={group.key}>
                      <div className="flex items-center justify-between text-xs text-muted-foreground">
                        <span>{t(group.labelKey)}</span>
                        <span className="rounded-full border border-border px-1.5 font-mono">{group.sessions.length}</span>
                      </div>
                      {group.sessions.map(renderSessionCard)}
                    </section>
                  ))}
                </div>
              ) : sidebarMode === "group" ? (
                <div className="grid gap-2">
                  {folderBuckets.map((group) => {
                    const expanded = expandedFolders.has(group.folder);
                    return (
                      <section className="grid gap-2" key={group.folder}>
                        <button className="ucd-list-row flex h-8 items-center gap-2 rounded-md px-2 text-left text-xs" onClick={() => toggleFolder(group.folder)} type="button">
                          {expanded ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}
                          <Folder className="h-3.5 w-3.5 text-primary" />
                          <span className="truncate">{group.folder}</span>
                          <span className="ml-auto font-mono text-muted-foreground">{group.sessions.length}</span>
                        </button>
                        {expanded ? group.sessions.map(renderSessionCard) : null}
                      </section>
                    );
                  })}
                </div>
              ) : (
                <div className="grid gap-2">
                  <div className="flex items-center justify-between text-xs text-muted-foreground">
                    <span className="inline-flex items-center gap-1"><Archive className="h-3.5 w-3.5" aria-hidden="true" />{t("layout.archived")}</span>
                    <span className="rounded-full border border-border px-1.5 font-mono">{archivedSessions.length}</span>
                  </div>
                  {archivedSessions.map(renderSessionCard)}
                  {archivedSessions.length === 0 ? <p className="ucd-muted-panel rounded-md p-3 text-xs text-muted-foreground">{t("layout.noArchived")}</p> : null}
                </div>
              )}
            </div>
            <div className="mt-3 grid grid-cols-2 gap-1.5 border-t border-border pt-3">
              <button className="ucd-list-row h-7 rounded-md text-xs" onClick={onOpenSettings} type="button">
                <Settings className="mr-1 inline h-3.5 w-3.5" aria-hidden="true" />{t("layout.settings")}
              </button>
              <button className="ucd-list-row h-7 rounded-md text-xs" type="button">
                <HelpCircle className="mr-1 inline h-3.5 w-3.5" aria-hidden="true" />{t("layout.help")}
              </button>
            </div>
          </aside>

          <section className="ucd-panel flex min-h-0 min-w-0 flex-col rounded-lg p-3">
            <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
              <h2 className="text-sm font-semibold">{t("layout.chatMode")}</h2>
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <span>{t("layout.status")}: {isStreaming ? t("layout.generating") : t("layout.idle")}</span>
                <span>{t("layout.messages")}: {chatMessages.length}</span>
                <span>{activeSession ? formatLifecycle(activeSession, t) : t("layout.noSession")}</span>
              </div>
            </div>
            <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border border-border bg-[hsl(var(--panel-muted))] shadow-sm">
              <div className="flex items-center justify-between gap-3 border-b border-border p-4">
                <div>
                  <h3 className="text-sm font-semibold">{activeSession?.title ?? t("layout.noSession")}</h3>
                  <p className="mt-1 text-xs text-muted-foreground">
                    {activeSession ? `${activeSession.agentId} · ${activeSession.interactionMode}` : t("layout.startChat")}
                  </p>
                </div>
                <span className="rounded-full bg-[hsl(var(--success-soft))] px-2 py-1 text-xs text-[hsl(var(--success))]">
                  {isStreaming ? t("layout.generating") : t("layout.ready")}
                </span>
              </div>
              <MessageList
                hasActiveSession={Boolean(activeSession)}
                hasMore={chatMessages.length >= messageLimit}
                messages={chatMessages}
                onLoadEarlier={() => setMessageLimit((current) => current + 50)}
              />
            </div>
            <div className="mt-3">
              <ChatInputBox
                agents={chatConfig.availableAgents.length > 0 ? chatConfig.availableAgents : agents}
                availableModes={chatConfig.availableModes}
                availableModels={chatConfig.availableModels}
                availableReasoning={chatConfig.availableReasoning}
                config={chatConfig.config}
                disabled={!activeSession || sendMessageMutation.isPending}
                isStreaming={isStreaming}
                onChange={setChatDraft}
                onClear={() => setChatDraft("")}
                onConfigAgentChange={chatConfig.changeAgent}
                onConfigLongContextChange={chatConfig.setLongContext}
                onConfigModeChange={chatConfig.setPermissionMode}
                onConfigModelChange={chatConfig.changeModel}
                onConfigProviderChange={chatConfig.changeProvider}
                onConfigReasoningChange={chatConfig.setReasoningDepth}
                onConfigStreamingChange={chatConfig.setStreaming}
                onConfigThinkingChange={chatConfig.setThinking}
                onStop={stopChatGeneration}
                onSubmit={submitChatMessage}
                value={chatDraft}
              />
            </div>
          </section>

          <aside className={cn("ucd-panel min-w-0 overflow-hidden rounded-lg transition-[opacity,transform] duration-200 max-[900px]:hidden", infoPanelCollapsed ? "pointer-events-none translate-x-2 opacity-0" : "opacity-100")}>
            <div className="flex h-full min-h-0 flex-col p-3">
              <div className="mb-3 flex items-center justify-between gap-2">
                <h2 className="text-sm font-semibold">{t("layout.infoPanel")}</h2>
                <Button className="h-7 px-2 text-xs" onClick={() => setInfoPanelCollapsed(true)} variant="outline">
                  <PanelRightClose className="h-3.5 w-3.5" aria-hidden="true" />
                  {t("layout.collapse")}
                </Button>
              </div>
              <div className="ucd-segmented mb-3 grid grid-cols-3 gap-1 rounded-md p-1">
                {infoTabs.map((tab) => (
                  <button className={cn("h-8 rounded-md text-xs", activeInfoTab === tab.key ? "bg-background font-semibold text-primary shadow-sm" : "text-muted-foreground hover:bg-muted")} key={tab.key} onClick={() => setActiveInfoTab(tab.key)} type="button">
                    {t(tab.labelKey)}
                  </button>
                ))}
              </div>
              <div className="min-h-0 flex-1 overflow-y-auto pr-1">
                <KeepAlivePane active={activeInfoTab === "agent"}>
                  <div className="grid gap-4">
                    <section className="ucd-muted-panel rounded-lg p-3">
                      <div className="mb-3 flex items-center justify-between">
                        <h3 className="text-sm font-semibold">{t("layout.taskProgress")}</h3>
                        <strong className="text-sm text-primary">{progressPercent}%</strong>
                      </div>
                      <div className="h-2 rounded bg-muted"><div className="h-2 w-[46%] rounded bg-primary" /></div>
                      <div className="mt-3 grid grid-cols-3 gap-2 text-center text-xs">
                        <div className="rounded border border-border p-2"><CheckCircle2 className="mx-auto mb-1 h-4 w-4 text-[hsl(var(--success))]" />{progressStats.complete}<br />{t("layout.completed")}</div>
                        <div className="rounded border border-border p-2"><CircleDot className="mx-auto mb-1 h-4 w-4 text-primary" />{progressStats.running}<br />{t("layout.running")}</div>
                        <div className="rounded border border-border p-2"><Clock3 className="mx-auto mb-1 h-4 w-4 text-muted-foreground" />{progressStats.pending}<br />{t("layout.pending")}</div>
                      </div>
                    </section>
                    <section className="ucd-muted-panel rounded-lg p-3">
                      <h3 className="mb-3 text-sm font-semibold">{t("layout.sessionConfig")}</h3>
                      <div className="grid gap-3 text-sm">
                        <div className="grid gap-1">
                          <span className="text-muted-foreground">{t("layout.sessionName")}</span>
                          <span className="min-h-8 truncate rounded border border-border bg-background px-2 py-1.5">{activeSession?.title ?? t("layout.noSession")}</span>
                        </div>
                        <div className="grid gap-1">
                          <span className="text-muted-foreground">{t("layout.description")}</span>
                          <span className="min-h-8 truncate rounded border border-border bg-background px-2 py-1.5">
                            {activeSession ? `${activeSession.agentId} · ${activeSession.interactionMode}` : t("layout.startChat")}
                          </span>
                        </div>
                        <div className="flex justify-between gap-3"><span className="text-muted-foreground">{t("layout.autoSave")}</span><span className="rounded-full bg-[hsl(var(--success-soft))] px-2 py-1 text-xs text-[hsl(var(--success))]">{t("layout.enabled")}</span></div>
                      </div>
                    </section>
                  </div>
                </KeepAlivePane>
                <KeepAlivePane active={activeInfoTab === "files"}>
                  <div className="grid gap-2">
                    {infoFiles.map((file) => <div className="flex items-center gap-2 rounded border border-border p-2 text-sm" key={file}><FileText className="h-4 w-4 text-primary" aria-hidden="true" /><span className="truncate">{file}</span></div>)}
                  </div>
                </KeepAlivePane>
                <KeepAlivePane active={activeInfoTab === "changes"}>
                  <div className="grid gap-2">
                    {changeItems.map((change) => <div className="flex items-center gap-2 rounded border border-border p-2 text-sm" key={change}><GitBranch className="h-4 w-4 text-primary" aria-hidden="true" /><span>{change}</span></div>)}
                  </div>
                </KeepAlivePane>
              </div>
            </div>
          </aside>
          {infoPanelCollapsed ? (
            <Button className="absolute right-2 top-1/2 h-9 w-9 -translate-y-1/2 px-0" onClick={() => setInfoPanelCollapsed(false)} size="icon" title={t("layout.expandInfo")} variant="outline">
              <PanelRightOpen className="h-4 w-4" aria-hidden="true" />
            </Button>
          ) : null}
        </div>
        <StatusBar />
      </div>
      {renderContextMenu()}
      {renderCenteredDialog()}
      <CreateSessionDialog
        agents={agents}
        onClose={() => setCreateSessionOpen(false)}
        onCreated={handleSessionCreated}
        open={createSessionOpen}
      />
    </main>
  );
}
