import { useEffect, useMemo, useState, type MouseEvent, type ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Archive, Bot, BrainCircuit, CheckCircle2, ChevronDown, ChevronRight, CircleDot, Clock3, Code2, FileText, Folder, GitBranch, HelpCircle, PanelRightClose, PanelRightOpen, Pin, Plus, RotateCcw, Settings, Sparkles, TerminalSquare, type LucideIcon } from "lucide-react";
import { ChatInputBox } from "../components/chat/ChatInputBox";
import { useChatConfig } from "../components/chat/hooks/useChatConfig";
import { MessageList } from "../components/chat/MessageList";
import { Button } from "../components/ui/button";
import { cn } from "../lib/utils";
import { agentService } from "../services/runtime-agent-client";
import { getNextThemeId, getThemeDefinition } from "../theme/theme-registry";
import { useTheme } from "../theme/theme-provider";
import type { Session } from "../types/agent";
import type { ChatConfig, ChatMessage as ChatMessageModel, ChatStreamEvent } from "../types/chat";
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

const activityGroups: Array<{ key: ActivityKey; label: string }> = [
  { key: "needs-input", label: "需要输入" }, { key: "pending-verification", label: "待验证" }, { key: "in-progress", label: "进行中" }, { key: "inactive", label: "非活跃" },
];

const agentMeta: Record<AgentKey, { label: string; Icon: LucideIcon; tone: string }> = {
  codex: { label: "Codex", Icon: Code2, tone: "border-sky-400/40 bg-sky-400/10 text-sky-500" },
  "claude-code": { label: "Claude Code", Icon: Sparkles, tone: "border-amber-400/40 bg-amber-400/10 text-amber-500" },
  opencode: { label: "OpenCode", Icon: TerminalSquare, tone: "border-emerald-400/40 bg-emerald-400/10 text-emerald-500" },
  gemini: { label: "Gemini", Icon: BrainCircuit, tone: "border-violet-400/40 bg-violet-400/10 text-violet-500" },
  unknown: { label: "Agent", Icon: Bot, tone: "border-border bg-muted text-muted-foreground" },
};

const infoTabs: Array<{ key: InfoTab; label: string }> = [{ key: "agent", label: "Agent Info" }, { key: "files", label: "Files" }, { key: "changes", label: "Changes" }];

function getActivityKeyForSession(session: Session): ActivityKey {
  if (session.archived || session.lifecycleState === "idle" || session.lifecycleState === "stopped") return "inactive";
  if (session.lifecycleState === "failed") return "needs-input";
  if (session.lifecycleState === "starting") return "pending-verification";
  return "in-progress";
}

function getSessionFolder(session: Session) {
  return session.folder ?? "当前工作区";
}

function getAgentKeyForSession(session: Session): AgentKey {
  if (session.agentId.includes("codex")) return "codex";
  if (session.agentId.includes("claude")) return "claude-code";
  if (session.agentId.includes("opencode")) return "opencode";
  if (session.agentId.includes("gemini")) return "gemini";
  return "unknown";
}

function formatSessionDate(session: Session) {
  return new Intl.DateTimeFormat("zh-CN", { month: "2-digit", day: "2-digit" }).format(new Date(session.updatedAt));
}

function formatLifecycle(session: Session) {
  const labels: Record<Session["lifecycleState"], string> = {
    failed: "需要输入",
    idle: "空闲",
    running: "进行中",
    starting: "启动中",
    stopped: "已停止",
  };
  return session.archived ? "已归档" : labels[session.lifecycleState];
}

function ConversationCard({
  active,
  onContextMenu,
  onSelect,
  session,
}: {
  active: boolean;
  onContextMenu: (event: MouseEvent<HTMLButtonElement>) => void;
  onSelect: () => void;
  session: Session;
}) {
  const meta = agentMeta[getAgentKeyForSession(session)];
  const Icon = meta.Icon;

  return (
    <button className={cn("relative w-full rounded-lg border border-border p-2.5 text-left transition-colors hover:bg-muted", active && "bg-[hsl(var(--nav-active-soft))]")} onClick={onSelect} onContextMenu={onContextMenu} type="button">
      {active ? <span className="absolute left-0 top-2 h-10 w-0.5 rounded bg-primary" /> : null}
      <div className="flex min-w-0 items-center gap-2">
        <span className={cn("flex h-6 w-6 shrink-0 items-center justify-center rounded border", meta.tone)} title={meta.label}>
          <Icon className="h-3.5 w-3.5" aria-hidden="true" />
        </span>
        <span className={cn("truncate text-sm font-medium", session.archived && "text-muted-foreground")}>{session.title}</span>
        {session.pinned ? <Pin className="ml-auto h-3.5 w-3.5 text-primary" aria-hidden="true" /> : null}
      </div>
      <div className="mt-2 flex items-center gap-2 text-xs text-muted-foreground">
        <span className={cn("h-2 w-2 rounded-full", session.archived ? "bg-muted-foreground" : "bg-[hsl(var(--success))]")} />
        <span>{formatLifecycle(session)}</span>
        <span className="font-mono">{meta.label}</span>
        <span className="ml-auto font-mono">{formatSessionDate(session)}</span>
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
  const [sidebarMode, setSidebarMode] = useState<SidebarMode>("activity");
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(() => new Set(["当前工作区", "工程项目", "内容项目"]));
  const [activeInfoTab, setActiveInfoTab] = useState<InfoTab>("agent");
  const [infoPanelCollapsed, setInfoPanelCollapsed] = useState(false);
  const [contextPanel, setContextPanel] = useState<ContextPanelState | null>(null);
  const [chatDraft, setChatDraft] = useState("");
  const [messageLimit, setMessageLimit] = useState(50);
  const { theme, setTheme } = useTheme();
  const queryClient = useQueryClient();
  const nextTheme = getNextThemeId(theme);
  const nextThemeDefinition = getThemeDefinition(nextTheme);

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

  function invalidateActiveMessages() {
    if (!activeSessionId) return;
    void queryClient.invalidateQueries({ queryKey: ["messages", activeSessionId] });
  }

  const createSessionMutation = useMutation({
    mutationFn: async () => {
      const activeAgent = agentsQuery.data?.[0];
      if (!activeAgent) throw new Error("No agents are available.");
      return agentService.createSession({
        agentId: activeAgent.id,
        interactionMode: activeAgent.supportedInteractionModes[0] ?? "cli",
      });
    },
    onSuccess: invalidateSessions,
  });
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
    onSuccess: invalidateActiveMessages,
  });
  const stopGenerationMutation = useMutation({
    mutationFn: (sessionId: string) => agentService.stopGeneration(sessionId),
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
      const folder = getSessionFolder(session);
      groups.set(folder, [...(groups.get(folder) ?? []), session]);
    });
    return Array.from(groups.entries()).map(([folder, groupedSessions]) => ({ folder, sessions: groupedSessions }));
  }, [sessions]);
  const pinnedSessions = useMemo(() => sessions.filter((session) => session.pinned), [sessions]);
  const progressStats = { complete: 6, running: 3, pending: 4 };
  const infoFiles = ["src/main-layout/main-layout.tsx", "openspec/changes/improve-main-layout-ui/tasks.md", "openspec/changes/improve-main-layout-ui/design.md"];
  const changeItems = ["侧边栏工具入口迁移", "信息面板折叠与 keep-alive", "主内容弹性布局"];
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

  function renderContextMenu() {
    if (!contextPanel || contextPanel.mode !== "menu") return null;
    const session = contextPanel.session;
    return (
      <div className="fixed z-50 grid w-40 gap-1 rounded-md border border-border bg-background p-1 text-sm shadow-lg" style={{ left: contextPanel.x, top: contextPanel.y }}>
        <button className="rounded px-2 py-1.5 text-left hover:bg-muted" onClick={() => setContextPanel({ ...contextPanel, mode: "rename" })} type="button">重命名</button>
        <button className="rounded px-2 py-1.5 text-left hover:bg-muted" onClick={() => { pinSessionMutation.mutate(session); setContextPanel(null); }} type="button">
          {session.pinned ? "取消置顶" : "置顶"}
        </button>
        <button className="rounded px-2 py-1.5 text-left hover:bg-muted" onClick={() => { archiveSessionMutation.mutate(session); setContextPanel(null); }} type="button">
          {session.archived ? <><RotateCcw className="mr-1 inline h-3.5 w-3.5" />恢复</> : "归档"}
        </button>
        <button className="rounded px-2 py-1.5 text-left text-destructive hover:bg-muted" onClick={() => setContextPanel({ ...contextPanel, mode: "delete" })} type="button">删除</button>
      </div>
    );
  }

  function renderCenteredDialog() {
    if (!contextPanel || contextPanel.mode === "menu") return null;
    if (contextPanel.mode === "rename") {
      return (
        <div className="fixed inset-0 z-50 grid place-items-center bg-background/60 p-4">
          <form className="grid w-full max-w-sm gap-3 rounded-lg border border-border bg-background p-4 text-sm shadow-xl" onSubmit={(event) => { event.preventDefault(); submitRename(); }}>
            <div>
              <h3 className="text-sm font-semibold">重命名会话</h3>
              <p className="mt-1 text-xs text-muted-foreground">修改当前会话在侧边栏中的显示名称。</p>
            </div>
            <label className="grid gap-1">
              <span className="text-xs text-muted-foreground">会话名称</span>
              <input
                autoFocus
                className="ucd-input h-9 rounded px-2 outline-none focus-visible:ring-2 focus-visible:ring-ring"
                onChange={(event) => setContextPanel({ ...contextPanel, draftTitle: event.target.value })}
                value={contextPanel.draftTitle}
              />
            </label>
            <div className="grid grid-cols-2 gap-2">
              <button className="h-8 rounded border border-border text-xs hover:bg-muted" onClick={() => setContextPanel(null)} type="button">取消</button>
              <button className="h-8 rounded bg-primary text-xs text-primary-foreground disabled:opacity-50" disabled={!contextPanel.draftTitle.trim()} type="submit">确认</button>
            </div>
          </form>
        </div>
      );
    }
    return (
      <div className="fixed inset-0 z-50 grid place-items-center bg-background/60 p-4">
        <div className="grid w-full max-w-sm gap-3 rounded-lg border border-border bg-background p-4 text-sm shadow-xl">
          <div>
            <h3 className="text-sm font-semibold">删除会话</h3>
            <p className="mt-1 break-words text-xs text-muted-foreground">“{contextPanel.session.title}” 删除后不可恢复。</p>
          </div>
          <div className="grid grid-cols-2 gap-2">
            <button className="h-8 rounded border border-border text-xs hover:bg-muted" onClick={() => setContextPanel(null)} type="button">取消</button>
            <button className="h-8 rounded bg-destructive text-xs text-destructive-foreground" onClick={confirmDelete} type="button">删除</button>
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
        onContextMenu={(event) => openContextMenu(event, session)}
        onSelect={() => {
          setContextPanel(null);
          if (!session.archived) switchSessionMutation.mutate(session.id);
        }}
        session={session}
      />
    );
  }

  return (
    <main className="min-h-screen bg-background text-foreground">
      <div className="pointer-events-none fixed inset-0 opacity-[0.035] [background-image:linear-gradient(hsl(var(--primary))_1px,transparent_1px),linear-gradient(90deg,hsl(var(--primary))_1px,transparent_1px)] [background-size:100px_100px]" />
      <div className="relative flex h-screen min-h-0 flex-col overflow-hidden">
        <TopBar />
        <div
          className={cn(
            "relative grid min-h-0 flex-1 gap-4 p-2 transition-[grid-template-columns] duration-200",
            infoPanelCollapsed ? "grid-cols-[220px_minmax(0,1fr)_0px]" : "grid-cols-[220px_minmax(0,1fr)_300px]",
          )}
        >
          <aside className="ucd-panel flex min-h-0 flex-col rounded-xl p-3" onContextMenu={(event) => event.preventDefault()}>
            <div className="mb-3 flex items-center justify-between gap-2">
              <h2 className="text-sm font-semibold">会话列表</h2>
              <Button className="h-7 px-2 text-xs" disabled={createSessionMutation.isPending || !agentsQuery.data?.length} onClick={() => createSessionMutation.mutate()}>
                <Plus className="h-3.5 w-3.5" aria-hidden="true" />新建
              </Button>
            </div>
            <div className="mb-3 grid grid-cols-3 gap-1 rounded border border-border bg-[hsl(var(--panel-muted))] p-1">
                {[["activity", "活动"], ["group", "分组"], ["archived", `归档 ${archivedSessions.length}`]].map(([key, label]) => (
                <button className={cn("h-7 rounded text-xs", sidebarMode === key ? "bg-background font-semibold text-primary" : "text-muted-foreground hover:bg-muted")} key={key} onClick={() => setSidebarMode(key as SidebarMode)} type="button">
                  {label}
                </button>
              ))}
            </div>
            <div className="min-h-0 flex-1 overflow-y-auto pr-1">
              {sidebarMode !== "archived" && pinnedSessions.length > 0 ? (
                <section className="mb-3 grid gap-2 border-b border-border pb-3">
                  <div className="flex items-center justify-between text-xs text-muted-foreground">
                    <span className="inline-flex items-center gap-1"><Pin className="h-3.5 w-3.5" aria-hidden="true" />置顶</span>
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
                        <span>{group.label}</span>
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
                        <button className="flex h-8 items-center gap-2 rounded border border-border px-2 text-left text-xs hover:bg-muted" onClick={() => toggleFolder(group.folder)} type="button">
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
                    <span className="inline-flex items-center gap-1"><Archive className="h-3.5 w-3.5" aria-hidden="true" />已归档</span>
                    <span className="rounded-full border border-border px-1.5 font-mono">{archivedSessions.length}</span>
                  </div>
                  {archivedSessions.map(renderSessionCard)}
                  {archivedSessions.length === 0 ? <p className="rounded border border-border p-3 text-xs text-muted-foreground">暂无归档会话</p> : null}
                </div>
              )}
            </div>
            <div className="mt-3 grid grid-cols-3 gap-1.5 border-t border-border pt-3">
              <button className="h-7 rounded border border-border text-xs hover:bg-muted" onClick={onOpenSettings} type="button">
                <Settings className="mr-1 inline h-3.5 w-3.5" aria-hidden="true" />设置
              </button>
              <button className="h-7 rounded border border-border text-xs hover:bg-muted" onClick={() => setTheme(nextTheme)} type="button">
                {nextThemeDefinition.displayName}
              </button>
              <button className="h-7 rounded border border-border text-xs hover:bg-muted" type="button">
                <HelpCircle className="mr-1 inline h-3.5 w-3.5" aria-hidden="true" />帮助
              </button>
            </div>
          </aside>

          <section className="ucd-panel flex min-h-0 min-w-0 flex-col rounded-xl p-3">
            <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
              <h2 className="text-sm font-semibold">聊天模式</h2>
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <span>状态: {isStreaming ? "生成中" : "空闲"}</span>
                <span>消息: {chatMessages.length}</span>
                <span>{activeSession ? formatLifecycle(activeSession) : "未选择会话"}</span>
              </div>
            </div>
            <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border border-border bg-[hsl(var(--panel-muted))]">
              <div className="flex items-center justify-between gap-3 border-b border-border p-4">
                <div>
                  <h3 className="text-sm font-semibold">{activeSession?.title ?? "未选择会话"}</h3>
                  <p className="mt-1 text-xs text-muted-foreground">
                    {activeSession ? `${activeSession.agentId} · ${activeSession.interactionMode}` : "新建或选择一个会话后开始聊天"}
                  </p>
                </div>
                <span className="rounded-full bg-[hsl(var(--success-soft))] px-2 py-1 text-xs text-[hsl(var(--success))]">
                  {isStreaming ? "生成中" : "就绪"}
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
                availableModes={chatConfig.availableModes.map((mode) => mode.id)}
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

          <aside className={cn("ucd-panel min-w-0 overflow-hidden rounded-xl transition-[opacity,transform] duration-200", infoPanelCollapsed ? "pointer-events-none translate-x-2 opacity-0" : "opacity-100")}>
            <div className="flex h-full min-h-0 flex-col p-3">
              <div className="mb-3 flex items-center justify-between gap-2">
                <h2 className="text-sm font-semibold">信息面板</h2>
                <Button className="h-7 px-2 text-xs" onClick={() => setInfoPanelCollapsed(true)} variant="outline">
                  <PanelRightClose className="h-3.5 w-3.5" aria-hidden="true" />
                  收起
                </Button>
              </div>
              <div className="mb-3 grid grid-cols-3 gap-1">
                {infoTabs.map((tab) => (
                  <button className={cn("h-8 rounded border border-border text-xs", activeInfoTab === tab.key ? "bg-[hsl(var(--nav-active-soft))] font-semibold text-primary" : "text-muted-foreground hover:bg-muted")} key={tab.key} onClick={() => setActiveInfoTab(tab.key)} type="button">
                    {tab.label}
                  </button>
                ))}
              </div>
              <div className="min-h-0 flex-1 overflow-y-auto pr-1">
                <KeepAlivePane active={activeInfoTab === "agent"}>
                  <div className="grid gap-4">
                    <section className="ucd-muted-panel rounded-lg p-3">
                      <div className="mb-3 flex items-center justify-between">
                        <h3 className="text-sm font-semibold">任务进度</h3>
                        <strong className="text-sm text-primary">{progressPercent}%</strong>
                      </div>
                      <div className="h-2 rounded bg-muted"><div className="h-2 w-[46%] rounded bg-primary" /></div>
                      <div className="mt-3 grid grid-cols-3 gap-2 text-center text-xs">
                        <div className="rounded border border-border p-2"><CheckCircle2 className="mx-auto mb-1 h-4 w-4 text-[hsl(var(--success))]" />{progressStats.complete}<br />已完成</div>
                        <div className="rounded border border-border p-2"><CircleDot className="mx-auto mb-1 h-4 w-4 text-primary" />{progressStats.running}<br />进行中</div>
                        <div className="rounded border border-border p-2"><Clock3 className="mx-auto mb-1 h-4 w-4 text-muted-foreground" />{progressStats.pending}<br />待处理</div>
                      </div>
                    </section>
                    <section className="ucd-muted-panel rounded-lg p-3">
                      <h3 className="mb-3 text-sm font-semibold">会话基础配置</h3>
                      <div className="grid gap-3 text-sm">
                        <label className="grid gap-1"><span className="text-muted-foreground">会话名称</span><input className="ucd-input h-8 rounded px-2" defaultValue="智能客服优化方案" /></label>
                        <label className="grid gap-1"><span className="text-muted-foreground">描述</span><input className="ucd-input h-8 rounded px-2" placeholder="输入会话描述..." /></label>
                        <div className="flex justify-between gap-3"><span className="text-muted-foreground">自动保存</span><span className="rounded-full bg-[hsl(var(--success-soft))] px-2 py-1 text-xs text-[hsl(var(--success))]">已启用</span></div>
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
            <Button className="absolute right-2 top-1/2 h-9 w-9 -translate-y-1/2 px-0" onClick={() => setInfoPanelCollapsed(false)} size="icon" title="展开信息面板" variant="outline">
              <PanelRightOpen className="h-4 w-4" aria-hidden="true" />
            </Button>
          ) : null}
        </div>
        <StatusBar />
      </div>
      {renderContextMenu()}
      {renderCenteredDialog()}
    </main>
  );
}
