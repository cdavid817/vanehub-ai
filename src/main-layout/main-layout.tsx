import { useState } from "react";
import {
  Bell,
  ChevronDown,
  Circle,
  Layers,
  Minus,
  Paperclip,
  Plus,
  Search,
  Send,
  Shield,
  SlidersHorizontal,
  Trash2,
  Users,
  Wrench,
  Zap,
} from "lucide-react";
import { Button } from "../components/ui/button";
import { cn } from "../lib/utils";
import { SettingsShell } from "../settings/settings-shell";
import { getNextThemeId, getThemeDefinition } from "../theme/theme-registry";
import { useTheme } from "../theme/theme-provider";

const conversations = [
  { title: "智能客服优化方案", status: "进行中", agents: "3 Agents", date: "07-14", active: true },
  { title: "数据分析报告生成", status: "进行中", agents: "2 Agents", date: "07-13" },
  { title: "代码审查自动化", status: "已归档", agents: "4 Agents", date: "07-10", archived: true },
  { title: "产品文档协作", status: "已归档", agents: "2 Agents", date: "07-08", archived: true },
  { title: "营销文案创作", status: "已归档", agents: "3 Agents", date: "07-05", archived: true },
];

const tools = [
  { label: "技能", icon: Shield, tone: "text-purple-400" },
  { label: "MCP 服务器", icon: Wrench, tone: "text-cyan-400" },
  { label: "插件", icon: Zap, tone: "text-primary" },
  { label: "看板", icon: Layers, tone: "text-emerald-400" },
  { label: "规则", icon: SlidersHorizontal, tone: "text-amber-400" },
  { label: "连接器", icon: Users, tone: "text-primary" },
];

const agentNodes = [
  {
    id: "reviewer",
    title: "代码审查员",
    description: "代码分析 · 安全检测",
    icon: "A",
    x: "left-[7%] top-[9%]",
    tone: "text-purple-400",
  },
  {
    id: "tester",
    title: "测试工程师",
    description: "单元测试 · 集成测试",
    icon: "T",
    x: "right-[30%] top-[9%]",
    tone: "text-cyan-400",
  },
  {
    id: "docs",
    title: "文档生成器",
    description: "文档编写 · 格式转换",
    icon: "D",
    x: "left-[27%] top-[43%]",
    tone: "text-emerald-400",
  },
];

const chatMessages = [
  {
    role: "用户",
    content: "优化智能客服的回答质量，重点关注多轮追问、转人工判断和知识库引用。",
    time: "14:20",
  },
  {
    role: "代码审查员",
    content: "已检查当前客服策略模块，建议把转人工规则拆分为意图识别、置信度阈值和兜底策略三层。",
    time: "14:22",
  },
  {
    role: "测试工程师",
    content: "我会补充多轮追问和低置信度场景的回归用例，覆盖 FAQ、订单、退款三类流程。",
    time: "14:24",
  },
  {
    role: "文档生成器",
    content: "已整理优化方案草案，包括配置项说明、上线步骤和客服运营验证清单。",
    time: "14:27",
  },
];

function TopBar() {
  return (
    <header className="ucd-panel mx-2 mt-2 flex min-h-12 items-center justify-between gap-3 rounded-xl px-4 py-2">
      <div className="flex min-w-0 items-center gap-3">
        <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md border border-primary bg-[hsl(var(--nav-active-soft))] text-sm font-bold text-primary">
          V
        </div>
        <div className="min-w-0">
          <div className="flex items-center gap-3">
            <h1 className="truncate text-base font-bold">VaneHub AI</h1>
            <span className="hidden font-mono text-xs text-muted-foreground sm:inline">#SID-20260714</span>
          </div>
        </div>
      </div>

      <div className="hidden min-w-72 max-w-sm flex-1 lg:block">
        <div className="relative">
          <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <input
            className="ucd-input h-8 w-full rounded-md px-9 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
            placeholder="搜索 Agent、对话、任务..."
          />
        </div>
      </div>

      <div className="flex items-center gap-2">
        <button className="relative flex h-8 w-9 items-center justify-center rounded border border-border hover:bg-muted" type="button">
          <Bell className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
          <span className="absolute right-2 top-1.5 h-2 w-2 rounded-full bg-[hsl(var(--danger))]" />
        </button>
      </div>
    </header>
  );
}

function ConversationSidebar({ onOpenSettings }: { onOpenSettings: () => void }) {
  const { theme, setTheme } = useTheme();
  const nextTheme = getNextThemeId(theme);
  const currentTheme = getThemeDefinition(theme);
  const nextThemeDefinition = getThemeDefinition(nextTheme);

  return (
    <aside className="ucd-panel flex min-h-0 flex-col rounded-xl p-3">
      <div className="mb-3 flex items-center justify-between gap-3">
        <h2 className="text-sm font-semibold">会话列表</h2>
        <Button className="h-7 px-2 text-xs">
          <Plus className="h-3.5 w-3.5" aria-hidden="true" />
          新建
        </Button>
      </div>

      <div className="mb-3 flex gap-1">
        {["全部", "收藏", "归档"].map((item, index) => (
          <button
            className={cn(
              "h-7 rounded border border-border px-3 text-xs",
              index === 0 ? "bg-background text-foreground" : "text-muted-foreground hover:bg-muted",
            )}
            key={item}
            type="button"
          >
            {item}
            {index === 0 ? <ChevronDown className="ml-1 inline h-3 w-3" aria-hidden="true" /> : null}
          </button>
        ))}
      </div>

      <div className="grid gap-2">
        {conversations.map((item) => (
          <button
            className={cn(
              "relative rounded-lg border border-border p-3 text-left transition-colors hover:bg-muted",
              item.active && "bg-[hsl(var(--nav-active-soft))]",
            )}
            key={item.title}
            type="button"
          >
            {item.active ? <span className="absolute left-0 top-2 h-10 w-0.5 rounded bg-primary" /> : null}
            <div className={cn("truncate text-sm font-medium", item.archived && "text-muted-foreground")}>{item.title}</div>
            <div className="mt-2 flex items-center gap-2 text-xs text-muted-foreground">
              <span className={cn("h-2 w-2 rounded-full", item.archived ? "bg-muted-foreground" : "bg-[hsl(var(--success))]")} />
              <span>{item.status}</span>
              <span>·</span>
              <span className="font-mono">{item.agents}</span>
              <span className="ml-auto font-mono">{item.date}</span>
            </div>
          </button>
        ))}
      </div>

      <div className="mt-auto border-t border-border pt-3">
        <h3 className="mb-2 text-xs font-semibold">工具</h3>
        <div className="grid gap-1.5">
          {tools.map((tool) => {
            const Icon = tool.icon;
            return (
              <button className="flex h-8 items-center gap-2 rounded border border-border px-2 text-sm hover:bg-muted" key={tool.label} type="button">
                <Icon className={cn("h-4 w-4", tool.tone)} aria-hidden="true" />
                <span>{tool.label}</span>
              </button>
            );
          })}
        </div>
        <div className="mt-3 grid grid-cols-3 gap-1.5">
          <button className="h-7 rounded border border-border text-xs hover:bg-muted" onClick={onOpenSettings} type="button">
            设置
          </button>
          <button
            className="h-7 rounded border border-border text-xs hover:bg-muted"
            onClick={() => setTheme(nextTheme)}
            title={`当前${currentTheme.displayName}，点击切换为${nextThemeDefinition.displayName}`}
            type="button"
          >
            {currentTheme.displayName}
          </button>
          <button className="h-7 rounded border border-border text-xs hover:bg-muted" type="button">
            帮助
          </button>
        </div>
      </div>
    </aside>
  );
}

function AgentNode({ node }: { node: (typeof agentNodes)[number] }) {
  return (
    <div className={cn("absolute w-36 rounded-xl border border-border bg-[hsl(var(--panel))] p-3 shadow-lg", node.x)}>
      <div className="flex items-center gap-2">
        <span className={cn("flex h-6 w-6 items-center justify-center rounded-full border border-current text-xs font-bold", node.tone)}>
          {node.icon}
        </span>
        <h3 className="truncate text-sm font-semibold">{node.title}</h3>
      </div>
      <p className="mt-3 text-xs text-muted-foreground">{node.description}</p>
      <div className="mt-3 flex gap-1">
        <button className="h-6 rounded border border-border px-2 text-xs text-muted-foreground" type="button">配置</button>
        <button className="h-6 rounded bg-primary px-2 text-xs text-primary-foreground" type="button">启动</button>
        <button className="h-6 rounded border border-border px-2 text-xs text-muted-foreground" type="button">删除</button>
      </div>
    </div>
  );
}

function FlowCanvas() {
  const [canvasMode, setCanvasMode] = useState<"chat" | "flow">("flow");

  return (
    <section className="ucd-panel flex min-h-[620px] min-w-0 flex-col rounded-xl p-3">
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <div className="flex gap-1">
          <button
            className={cn(
              "h-8 rounded border px-3 text-xs",
              canvasMode === "chat"
                ? "border-primary bg-[hsl(var(--nav-active-soft))] font-semibold text-primary"
                : "border-border text-muted-foreground hover:bg-muted",
            )}
            onClick={() => setCanvasMode("chat")}
            type="button"
          >
            对话聊天
          </button>
          <button
            className={cn(
              "h-8 rounded border px-3 text-xs",
              canvasMode === "flow"
                ? "border-primary bg-[hsl(var(--nav-active-soft))] font-semibold text-primary"
                : "border-border text-muted-foreground hover:bg-muted",
            )}
            onClick={() => setCanvasMode("flow")}
            type="button"
          >
            流程图模式
          </button>
        </div>
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <span>状态: 空闲</span>
          <span>Token: 2,340</span>
          <span>调用: 15</span>
        </div>
      </div>

      <div className="relative min-h-[530px] flex-1 overflow-hidden rounded-lg border border-border bg-[linear-gradient(hsl(var(--primary)/0.08)_1px,transparent_1px),linear-gradient(90deg,hsl(var(--primary)/0.08)_1px,transparent_1px)] bg-[size:100px_100px]">
        {canvasMode === "flow" ? (
          <>
            <svg className="absolute inset-0 h-full w-full" aria-hidden="true">
              <defs>
                <marker id="flow-arrow" markerHeight="7" markerWidth="7" orient="auto" refX="6" refY="3.5">
                  <path d="M0,0 L7,3.5 L0,7 Z" fill="hsl(var(--primary))" opacity="0.8" />
                </marker>
              </defs>
              <path d="M 175 115 C 260 100, 330 120, 430 115" fill="none" stroke="hsl(var(--primary))" strokeWidth="2" markerEnd="url(#flow-arrow)" opacity="0.65" />
              <path d="M 175 150 C 250 260, 320 300, 390 315" fill="none" stroke="hsl(var(--primary))" strokeWidth="2" markerEnd="url(#flow-arrow)" opacity="0.45" />
              <path d="M 500 150 C 480 245, 460 280, 445 315" fill="none" stroke="hsl(var(--primary))" strokeWidth="2" markerEnd="url(#flow-arrow)" opacity="0.45" />
            </svg>

            {agentNodes.map((node) => (
              <AgentNode key={node.id} node={node} />
            ))}

            <div className="absolute bottom-24 left-1/2 -translate-x-1/2 text-xs text-muted-foreground">
              拖拽 Agent 至画布 · 滚轮缩放 · 框选批量操作
            </div>
            <div className="absolute bottom-4 right-4 rounded border border-border bg-background/80 p-2 text-xs text-muted-foreground">
              <div className="mb-1 h-10 w-14 rounded border border-border bg-[hsl(var(--panel-muted))]" />
              100%
            </div>
          </>
        ) : (
          <div className="flex h-full min-h-[530px] flex-col gap-3 p-4">
            <div className="flex items-center justify-between gap-3 border-b border-border pb-3">
              <div>
                <h3 className="text-sm font-semibold">智能客服优化方案</h3>
                <p className="mt-1 text-xs text-muted-foreground">3 Agents 正在协作 · 最近更新 14:27</p>
              </div>
              <span className="rounded-full bg-[hsl(var(--success-soft))] px-2 py-1 text-xs text-[hsl(var(--success))]">进行中</span>
            </div>
            <div className="grid flex-1 content-start gap-3 overflow-auto pr-1">
              {chatMessages.map((message, index) => (
                <div
                  className={cn(
                    "max-w-[82%] rounded-lg border border-border p-3 text-sm",
                    index === 0 ? "ml-auto bg-primary text-primary-foreground" : "bg-[hsl(var(--panel))]",
                  )}
                  key={`${message.role}-${message.time}`}
                >
                  <div className={cn("mb-1 flex items-center justify-between gap-3 text-xs", index === 0 ? "text-primary-foreground/80" : "text-muted-foreground")}>
                    <span>{message.role}</span>
                    <span>{message.time}</span>
                  </div>
                  <p className="leading-6">{message.content}</p>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      <div className="mt-3 rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-3">
        <textarea
          className="ucd-input min-h-16 w-full resize-none rounded-md px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
          placeholder="输入指令，下发任务给所有 Agent..."
        />
        <div className="mt-2 flex flex-wrap items-center gap-2">
          <Button variant="outline"><Paperclip className="h-4 w-4" aria-hidden="true" />附件</Button>
          <Button variant="outline"><Trash2 className="h-4 w-4" aria-hidden="true" />清空</Button>
          <div className="ml-auto flex flex-wrap gap-2">
            <Button variant="outline">Claude</Button>
            <Button variant="outline">GLM4.5</Button>
            <Button variant="outline">High</Button>
            <Button><Send className="h-4 w-4" aria-hidden="true" />发送</Button>
          </div>
        </div>
      </div>
    </section>
  );
}

function InfoPanel() {
  return (
    <aside className="ucd-panel min-h-[620px] rounded-xl p-3">
      <div className="mb-3 flex items-center justify-between gap-2">
        <h2 className="text-sm font-semibold">信息面板</h2>
        <Button className="h-7 px-2 text-xs" variant="outline">收起</Button>
      </div>
      <div className="mb-4 grid grid-cols-5 gap-1">
        {["Agent", "日志", "历史", "素材", "配置"].map((tab, index) => (
          <button
            className={cn(
              "h-8 rounded border border-border text-xs",
              index === 0 ? "bg-[hsl(var(--nav-active-soft))] font-semibold text-primary" : "text-muted-foreground",
            )}
            key={tab}
            type="button"
          >
            {tab}
          </button>
        ))}
      </div>

      <div className="grid gap-4">
        <section className="ucd-muted-panel rounded-lg p-3">
          <h3 className="mb-3 text-sm font-semibold">会话基础配置</h3>
          <div className="grid gap-3 text-sm">
            <label className="grid gap-1">
              <span className="text-muted-foreground">会话名称</span>
              <input className="ucd-input h-8 rounded px-2" defaultValue="智能客服优化方案" />
            </label>
            <label className="grid gap-1">
              <span className="text-muted-foreground">描述</span>
              <input className="ucd-input h-8 rounded px-2" placeholder="输入会话描述..." />
            </label>
            <div className="flex items-center justify-between gap-3">
              <span className="text-muted-foreground">协作权限</span>
              <strong>可编辑</strong>
            </div>
            <div className="flex items-center justify-between gap-3">
              <span className="text-muted-foreground">自动保存</span>
              <span className="rounded-full bg-[hsl(var(--success-soft))] px-2 py-1 text-xs text-[hsl(var(--success))]">已启用</span>
            </div>
          </div>
        </section>

        <section className="ucd-muted-panel rounded-lg p-3">
          <h3 className="mb-3 text-sm font-semibold">全局模型参数</h3>
          <div className="grid gap-3 text-sm">
            <div className="flex items-center gap-3">
              <span className="w-16 text-muted-foreground">温度</span>
              <strong>0.7</strong>
              <div className="h-2 flex-1 rounded bg-muted"><div className="h-2 w-3/5 rounded bg-primary" /></div>
            </div>
            <div className="flex justify-between gap-3"><span className="text-muted-foreground">最大上下文</span><strong>4096</strong></div>
            <div className="flex justify-between gap-3"><span className="text-muted-foreground">输出格式</span><strong>文本</strong></div>
          </div>
        </section>

        <section className="ucd-muted-panel rounded-lg p-3">
          <h3 className="mb-3 text-sm font-semibold">权限管理</h3>
          <div className="grid gap-2 text-sm">
            {[
              ["Z", "张三", "所有者"],
              ["L", "李四", "可编辑"],
              ["W", "王五", "只读"],
            ].map(([abbr, name, role]) => (
              <div className="flex items-center gap-2 rounded border border-border p-2" key={name}>
                <span className="flex h-6 w-6 items-center justify-center rounded-full bg-[hsl(var(--nav-active-soft))] text-xs font-semibold text-primary">{abbr}</span>
                <span>{name}</span>
                <span className="ml-auto text-xs text-muted-foreground">{role}</span>
              </div>
            ))}
            <button className="h-8 rounded border border-dashed border-border text-xs text-primary" type="button">+ 添加协作者</button>
          </div>
        </section>

        <section className="ucd-muted-panel rounded-lg p-3">
          <h3 className="mb-3 text-sm font-semibold">运行时信息</h3>
          <div className="grid grid-cols-2 gap-2 text-sm">
            {[
              ["Token 用量", "60%"],
              ["API 调用", "1,247"],
              ["预估费用", "$3.42"],
              ["运行时长", "02:34:18"],
            ].map(([label, value]) => (
              <div className="rounded border border-border p-2" key={label}>
                <div className="text-xs text-muted-foreground">{label}</div>
                <strong>{value}</strong>
              </div>
            ))}
          </div>
        </section>
      </div>
    </aside>
  );
}

function StatusBar() {
  return (
    <footer className="mx-2 mb-2 flex min-h-8 flex-wrap items-center justify-between gap-2 rounded border border-border px-3 text-xs text-muted-foreground">
      <div className="flex items-center gap-3">
        <span className="inline-flex items-center gap-1"><Circle className="h-3 w-3 fill-[hsl(var(--success))] text-[hsl(var(--success))]" />已连接</span>
        <span>状态: 空闲</span>
        <span>Token: 2,340</span>
        <span>调用: 15</span>
      </div>
      <div className="flex items-center gap-3">
        <button className="inline-flex items-center gap-1" type="button"><Plus className="h-3 w-3" />100%</button>
        <button type="button"><Minus className="h-3 w-3" /></button>
        <span>已自动保存</span>
        <span>v0.1.0</span>
      </div>
    </footer>
  );
}

export function MainLayout() {
  const [view, setView] = useState<"workspace" | "settings">("workspace");

  if (view === "settings") {
    return (
      <div>
        <SettingsShell onReturn={() => setView("workspace")} />
      </div>
    );
  }

  return (
    <main className="min-h-screen bg-background text-foreground">
      <div className="pointer-events-none fixed inset-0 opacity-[0.035] [background-image:linear-gradient(hsl(var(--primary))_1px,transparent_1px),linear-gradient(90deg,hsl(var(--primary))_1px,transparent_1px)] [background-size:100px_100px]" />
      <div className="relative flex min-h-screen flex-col">
        <TopBar />
        <div className="grid flex-1 gap-4 p-2 xl:grid-cols-[230px_minmax(0,620px)_minmax(290px,1fr)]">
          <ConversationSidebar onOpenSettings={() => setView("settings")} />
          <FlowCanvas />
          <InfoPanel />
        </div>
        <StatusBar />
      </div>
    </main>
  );
}
