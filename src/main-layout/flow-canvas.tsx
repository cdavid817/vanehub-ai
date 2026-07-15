import { useState } from "react";
import { Paperclip, Send, Trash2 } from "lucide-react";
import { Button } from "../components/ui/button";
import { cn } from "../lib/utils";
import type { WorkspaceAgentNode, WorkspaceChatMessage } from "../types/workspace";

function AgentNode({ node }: { node: WorkspaceAgentNode }) {
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

export function FlowCanvas({
  agentNodes,
  chatMessages,
}: {
  agentNodes: WorkspaceAgentNode[];
  chatMessages: WorkspaceChatMessage[];
}) {
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
