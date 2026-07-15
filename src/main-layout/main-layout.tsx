import { useMemo, useState, type ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { Bot, BrainCircuit, CheckCircle2, ChevronDown, ChevronRight, CircleDot, Clock3, Code2, FileText, Folder, GitBranch, HelpCircle, PanelRightClose, PanelRightOpen, Paperclip, Plus, Send, Settings, Sparkles, TerminalSquare, Trash2, type LucideIcon } from "lucide-react";
import { Button } from "../components/ui/button";
import { cn } from "../lib/utils";
import { emptyWorkspaceSnapshot } from "../services/mock-workspace-data";
import { workspaceService } from "../services/runtime-workspace-client";
import { getNextThemeId, getThemeDefinition } from "../theme/theme-registry";
import { useTheme } from "../theme/theme-provider";
import type { WorkspaceChatMessage, WorkspaceConversation } from "../types/workspace";
import { StatusBar } from "./status-bar";
import { TopBar } from "./top-bar";

type SidebarMode = "activity" | "group";
type ActivityKey = "needs-input" | "pending-verification" | "in-progress" | "inactive";
type AgentKey = "codex" | "claude-code" | "opencode" | "gemini" | "unknown";
type InfoTab = "agent" | "files" | "changes";

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

function getActivityKey(conversation: WorkspaceConversation): ActivityKey {
  if (conversation.archived) return "inactive";
  if (conversation.active) return "needs-input";
  return conversation.status.includes("验证") ? "pending-verification" : "in-progress";
}

function getConversationFolder(conversation: WorkspaceConversation) {
  if (conversation.title.includes("代码") || conversation.title.includes("数据")) return "工程项目";
  return conversation.title.includes("文档") || conversation.title.includes("营销") ? "内容项目" : "当前工作区";
}

function getAgentKey(conversation: WorkspaceConversation): AgentKey {
  if (conversation.title.includes("代码")) return "codex";
  if (conversation.title.includes("客服")) return "claude-code";
  if (conversation.title.includes("数据")) return "gemini";
  return conversation.title.includes("文档") || conversation.title.includes("营销") ? "opencode" : "unknown";
}

function ConversationCard({ conversation }: { conversation: WorkspaceConversation }) {
  const meta = agentMeta[getAgentKey(conversation)];
  const Icon = meta.Icon;

  return (
    <button className={cn("relative w-full rounded-lg border border-border p-2.5 text-left transition-colors hover:bg-muted", conversation.active && "bg-[hsl(var(--nav-active-soft))]")} type="button">
      {conversation.active ? <span className="absolute left-0 top-2 h-10 w-0.5 rounded bg-primary" /> : null}
      <div className="flex min-w-0 items-center gap-2">
        <span className={cn("flex h-6 w-6 shrink-0 items-center justify-center rounded border", meta.tone)} title={meta.label}>
          <Icon className="h-3.5 w-3.5" aria-hidden="true" />
        </span>
        <span className={cn("truncate text-sm font-medium", conversation.archived && "text-muted-foreground")}>{conversation.title}</span>
      </div>
      <div className="mt-2 flex items-center gap-2 text-xs text-muted-foreground">
        <span className={cn("h-2 w-2 rounded-full", conversation.archived ? "bg-muted-foreground" : "bg-[hsl(var(--success))]")} />
        <span>{conversation.status}</span>
        <span className="font-mono">{conversation.agents}</span>
        <span className="ml-auto font-mono">{conversation.date}</span>
      </div>
    </button>
  );
}

function ChatMessage({ message, own }: { message: WorkspaceChatMessage; own: boolean }) {
  return (
    <div className={cn("max-w-[78%] rounded-lg border border-border p-3 text-sm", own ? "ml-auto bg-primary text-primary-foreground" : "bg-[hsl(var(--panel))]")}>
      <div className={cn("mb-1 flex items-center justify-between gap-3 text-xs", own ? "text-primary-foreground/80" : "text-muted-foreground")}>
        <span>{message.role}</span>
        <span>{message.time}</span>
      </div>
      <p className="leading-6">{message.content}</p>
    </div>
  );
}

function KeepAlivePane({ active, children }: { active: boolean; children: ReactNode }) {
  return <div className={cn("h-full", active ? "block" : "hidden")}>{children}</div>;
}

export function MainLayout({ onOpenSettings }: { onOpenSettings: () => void }) {
  const [sidebarMode, setSidebarMode] = useState<SidebarMode>("activity");
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(() => new Set(["当前工作区", "工程项目", "内容项目"]));
  const [activeInfoTab, setActiveInfoTab] = useState<InfoTab>("agent");
  const [infoPanelCollapsed, setInfoPanelCollapsed] = useState(false);
  const { theme, setTheme } = useTheme();
  const nextTheme = getNextThemeId(theme);
  const nextThemeDefinition = getThemeDefinition(nextTheme);

  const workspaceQuery = useQuery({
    queryKey: ["workspace", "snapshot"],
    queryFn: () => workspaceService.getWorkspaceSnapshot(),
  });
  const workspace = workspaceQuery.data ?? emptyWorkspaceSnapshot;

  const activityBuckets = useMemo(
    () =>
      activityGroups.map((group) => ({
        ...group,
        conversations: workspace.conversations.filter((conversation) => getActivityKey(conversation) === group.key),
      })),
    [workspace.conversations],
  );
  const folderBuckets = useMemo(() => {
    const groups = new Map<string, WorkspaceConversation[]>();
    workspace.conversations.forEach((conversation) => {
      const folder = getConversationFolder(conversation);
      groups.set(folder, [...(groups.get(folder) ?? []), conversation]);
    });
    return Array.from(groups.entries()).map(([folder, conversations]) => ({ folder, conversations }));
  }, [workspace.conversations]);
  const progressStats = { complete: 6, running: 3, pending: 4 };
  const infoFiles = ["src/main-layout/main-layout.tsx", "openspec/changes/improve-main-layout-ui/tasks.md", "openspec/changes/improve-main-layout-ui/design.md"];
  const changeItems = ["侧边栏工具入口迁移", "信息面板折叠与 keep-alive", "主内容弹性布局"];
  const progressTotal = progressStats.complete + progressStats.running + progressStats.pending;
  const progressPercent = Math.round((progressStats.complete / progressTotal) * 100);

  function toggleFolder(folder: string) {
    setExpandedFolders((current) => {
      const next = new Set(current);
      if (next.has(folder)) next.delete(folder);
      else next.add(folder);
      return next;
    });
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
          <aside className="ucd-panel flex min-h-0 flex-col rounded-xl p-3">
            <div className="mb-3 flex items-center justify-between gap-2">
              <h2 className="text-sm font-semibold">会话列表</h2>
              <Button className="h-7 px-2 text-xs"><Plus className="h-3.5 w-3.5" aria-hidden="true" />新建</Button>
            </div>
            <div className="mb-3 grid grid-cols-2 gap-1 rounded border border-border bg-[hsl(var(--panel-muted))] p-1">
                {[["activity", "活动"], ["group", "分组"]].map(([key, label]) => (
                <button className={cn("h-7 rounded text-xs", sidebarMode === key ? "bg-background font-semibold text-primary" : "text-muted-foreground hover:bg-muted")} key={key} onClick={() => setSidebarMode(key as SidebarMode)} type="button">
                  {label}
                </button>
              ))}
            </div>
            <div className="min-h-0 flex-1 overflow-y-auto pr-1">
              {sidebarMode === "activity" ? (
                <div className="grid gap-3">
                  {activityBuckets.map((group) => (
                    <section className="grid gap-2" key={group.key}>
                      <div className="flex items-center justify-between text-xs text-muted-foreground">
                        <span>{group.label}</span>
                        <span className="rounded-full border border-border px-1.5 font-mono">{group.conversations.length}</span>
                      </div>
                      {group.conversations.map((conversation) => <ConversationCard conversation={conversation} key={conversation.title} />)}
                    </section>
                  ))}
                </div>
              ) : (
                <div className="grid gap-2">
                  {folderBuckets.map((group) => {
                    const expanded = expandedFolders.has(group.folder);
                    return (
                      <section className="grid gap-2" key={group.folder}>
                        <button className="flex h-8 items-center gap-2 rounded border border-border px-2 text-left text-xs hover:bg-muted" onClick={() => toggleFolder(group.folder)} type="button">
                          {expanded ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}
                          <Folder className="h-3.5 w-3.5 text-primary" />
                          <span className="truncate">{group.folder}</span>
                          <span className="ml-auto font-mono text-muted-foreground">{group.conversations.length}</span>
                        </button>
                        {expanded ? group.conversations.map((conversation) => <ConversationCard conversation={conversation} key={conversation.title} />) : null}
                      </section>
                    );
                  })}
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
                <span>状态: 空闲</span>
                <span>Token: 2,340</span>
                <span>调用: 15</span>
              </div>
            </div>
            <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border border-border bg-[hsl(var(--panel-muted))]">
              <div className="flex items-center justify-between gap-3 border-b border-border p-4">
                <div><h3 className="text-sm font-semibold">智能客服优化方案</h3><p className="mt-1 text-xs text-muted-foreground">3 Agents 正在协作 · 最近更新 14:27</p></div>
                <span className="rounded-full bg-[hsl(var(--success-soft))] px-2 py-1 text-xs text-[hsl(var(--success))]">进行中</span>
              </div>
              <div className="grid flex-1 content-start gap-3 overflow-y-auto p-4">
                {workspace.chatMessages.map((message, index) => <ChatMessage key={`${message.role}-${message.time}`} message={message} own={index === 0} />)}
              </div>
            </div>
            <div className="mt-3 shrink-0 rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-3">
              <textarea className="ucd-input h-16 w-full resize-none rounded-md px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring" placeholder="输入指令，下发任务给所有 Agent..." />
              <div className="mt-2 flex flex-wrap items-center gap-2">
                <Button variant="outline"><Paperclip className="h-4 w-4" aria-hidden="true" />附件</Button>
                <Button variant="outline"><Trash2 className="h-4 w-4" aria-hidden="true" />清空</Button>
                <div className="ml-auto flex flex-wrap gap-2">
                  <Button variant="outline">Claude</Button>
                  <Button variant="outline">High</Button>
                  <Button><Send className="h-4 w-4" aria-hidden="true" />发送</Button>
                </div>
              </div>
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
    </main>
  );
}
