import { AlertTriangle, Bot, CheckCircle2, CircleStop, UserRound } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../../lib/utils";
import type { ChatMessage } from "../../types/chat";
import { ThinkingBlock } from "./ThinkingBlock";
import { ToolUseBlock } from "./ToolUseBlock";
import { WaitingIndicator } from "./WaitingIndicator";

function statusLabel(message: ChatMessage, t: (key: string) => string) {
  if (message.status === "streaming") return message.content ? t("chat.status.streaming") : t("chat.status.waiting");
  if (message.status === "failed") return t("chat.status.failed");
  if (message.status === "cancelled") return t("chat.status.cancelled");
  return t("chat.status.completed");
}

function formatTime(value: string, language: string) {
  return new Intl.DateTimeFormat(language, {
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(value));
}

export function MessageItem({ message }: { message: ChatMessage }) {
  const { i18n, t } = useTranslation();
  const isUser = message.role === "user";
  const Icon = isUser ? UserRound : Bot;
  return (
    <article className={cn("flex gap-3", isUser && "justify-end")}>
      {!isUser ? (
        <span className="mt-1 flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-border bg-background text-primary">
          <Icon className="h-4 w-4" aria-hidden="true" />
        </span>
      ) : null}
      <div
        className={cn(
          "max-w-[78%] rounded-lg border border-border p-3 text-sm",
          isUser ? "bg-primary text-primary-foreground" : "bg-background",
          message.status === "failed" && "border-destructive/50",
          message.status === "cancelled" && "border-warning/50",
        )}
      >
        <div className={cn("mb-2 flex items-center gap-2 text-xs", isUser ? "text-primary-foreground/80" : "text-muted-foreground")}>
          <span>{isUser ? t("chat.you") : message.role === "assistant" ? t("chat.agent") : message.role}</span>
          <span className="font-mono">{formatTime(message.updatedAt, i18n.language)}</span>
          <span className="ml-auto inline-flex items-center gap-1">
            {message.status === "failed" ? <AlertTriangle className="h-3.5 w-3.5" aria-hidden="true" /> : null}
            {message.status === "cancelled" ? <CircleStop className="h-3.5 w-3.5" aria-hidden="true" /> : null}
            {message.status === "completed" ? <CheckCircle2 className="h-3.5 w-3.5" aria-hidden="true" /> : null}
            {statusLabel(message, t)}
          </span>
        </div>
        {message.content ? (
          <p className="whitespace-pre-wrap leading-6">{message.content}</p>
        ) : message.status === "streaming" ? (
          <WaitingIndicator />
        ) : null}
        {message.error ? <p className="mt-2 text-xs text-destructive">{message.error}</p> : null}
        <ThinkingBlock content={message.thinkingContent ?? ""} />
        <ToolUseBlock toolUse={message.toolUse ?? []} />
      </div>
    </article>
  );
}
