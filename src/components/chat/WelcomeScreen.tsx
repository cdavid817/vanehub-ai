import { Bot, MessageSquarePlus } from "lucide-react";

export function WelcomeScreen({ hasActiveSession }: { hasActiveSession: boolean }) {
  return (
    <div className="grid h-full place-items-center p-6 text-center">
      <div className="max-w-sm">
        <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-lg border border-border bg-background">
          {hasActiveSession ? (
            <MessageSquarePlus className="h-5 w-5 text-primary" aria-hidden="true" />
          ) : (
            <Bot className="h-5 w-5 text-primary" aria-hidden="true" />
          )}
        </div>
        <h3 className="text-sm font-semibold">{hasActiveSession ? "开始新的对话" : "请选择或新建会话"}</h3>
        <p className="mt-2 text-xs leading-5 text-muted-foreground">
          {hasActiveSession ? "输入消息后，Agent 响应会在这里流式显示。" : "会话用于保存 Agent、模式和聊天历史。"}
        </p>
      </div>
    </div>
  );
}
