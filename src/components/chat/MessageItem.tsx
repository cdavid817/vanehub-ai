import { memo, type KeyboardEvent, type MouseEvent } from "react";
import { AlertTriangle, Bot, Check, CheckCircle2, CircleStop, FileText } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../../lib/utils";
import type { ChatMessage } from "../../types/chat";
import type { MessageSpeaker } from "../../services/message-speaker";
import { Badge } from "../ui/badge";
import { FileReferenceLines } from "./FileReferenceLines";
import { RichMarkdown } from "./RichMarkdown";
import { RichBlocks } from "./RichBlocks";
import { isInteractiveClickTarget, isSelfKeyActivation } from "./selection-click-target";
import { ThinkingBlock } from "./ThinkingBlock";
import { ToolUseBlock } from "./ToolUseBlock";
import { WaitingIndicator } from "./WaitingIndicator";
import { MessageFeedbackControls } from "./MessageFeedbackControls";
import { MessageMemoryMenu, type MessageMemoryContext } from "./MessageMemoryMenu";
import { ParticipantAvatar } from "../session-roster-presence";
import { AgentRunOwnerStatus } from "../ui/agent-run-owner-status";

// The message bubble wraps content that already owns its own interaction (feedback/memory
// buttons, file links, a disclosure, and the ToolUseBlock subtree, which has its own, separate
// per-tool-call selection) -- a click or Enter/Space landing on any of those must resolve that
// control, not also select the whole message.
const MESSAGE_SELECTION_EXCLUDED_SELECTOR =
  'button, a[href], input, textarea, select, [role="button"], summary, label, [data-testid="tool-activity"]';

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

function safeRoleColor(color: string) {
  return /^#[0-9a-f]{6}$/i.test(color) ? color : "currentColor";
}

// Memoized because streaming appends a token at a time: applyChatEvent keeps stable
// references for unchanged messages, so memo lets historical rows skip re-rendering
// (and re-parsing markdown / mermaid) on every token — only the streaming row updates.
export const MessageItem = memo(function MessageItem({
  memoryContext,
  message,
  onSelect,
  onSelectTool,
  selected = false,
  selectedToolCallId = null,
  speaker,
}: {
  /** Absent outside a session, where a message has no Agent or project to remember against. */
  memoryContext?: MessageMemoryContext | null;
  message: ChatMessage;
  /**
   * Absent until a caller wires message selection (design.md Decision 8's Follow Selection); the
   * bubble then stays the plain, non-interactive div it was before this existed.
   */
  onSelect?: () => void;
  /** Bound to a specific tool call id by `ToolUseBlock`; this component only forwards it. */
  onSelectTool?: (toolCallId: string) => void;
  selected?: boolean;
  selectedToolCallId?: string | null;
  /**
   * Present only in a multi-seat session. When absent the message renders exactly as it did before
   * seats existed, so single-Agent sessions are untouched.
   */
  speaker?: MessageSpeaker | null;
}) {
  const { i18n, t } = useTranslation();
  const isUser = message.role === "user";
  const showSpeaker = !isUser && Boolean(speaker);

  function handleBubbleClick(event: MouseEvent<HTMLDivElement>) {
    if (onSelect && !isInteractiveClickTarget(event.target, MESSAGE_SELECTION_EXCLUDED_SELECTOR, event.currentTarget)) onSelect();
  }

  function handleBubbleKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (!onSelect || !isSelfKeyActivation(event)) return;
    event.preventDefault();
    onSelect();
  }

  return (
    <article className={cn("flex min-w-0 items-start gap-2.5", isUser && "justify-end")}>
      {!isUser ? (
        showSpeaker && speaker ? <ParticipantAvatar agentId={speaker.agentId} label={`${speaker.roleName ?? speaker.agentName} · ${speaker.agentName}`} roleAvatar={speaker.avatar} roleName={speaker.roleName} /> : (
          <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border bg-background text-primary shadow-xs">
            <Bot className="h-4 w-4" aria-hidden="true" />
          </span>
        )
      ) : null}
      <div className="min-w-0 max-w-[88%] lg:max-w-3xl">
        <div className={cn("mb-1 flex items-center gap-2 px-1 text-xs text-muted-foreground", isUser && "justify-end")}>
          {showSpeaker ? (
            <span className="inline-flex items-center gap-1.5" data-testid="message-speaker">
              <svg aria-hidden="true" className="h-2.5 w-2.5 shrink-0 text-primary" data-testid="message-role-color" fill={safeRoleColor(speaker?.color ?? "")} viewBox="0 0 10 10">
                <circle cx="5" cy="5" r="4" />
              </svg>
              <span className="font-semibold text-primary">
                {speaker?.roleName ?? speaker?.agentName}
              </span>
              {speaker?.roleName ? <span>· {speaker.agentName}</span> : null}
              {speaker?.crossFamilyReviewer ? (
                <span className="rounded bg-muted px-1 py-0.5 text-[10px]">{t("chat.crossFamily")}</span>
              ) : null}
            </span>
          ) : (
            <span>{isUser ? t("chat.you") : message.role === "assistant" ? t("chat.agent") : message.role}</span>
          )}
          <span className="font-mono">{formatTime(message.updatedAt, i18n.language)}</span>
          {selected ? (
            <Badge className="gap-1" data-testid="message-selected-indicator" tone="default">
              <Check className="h-3 w-3" aria-hidden="true" />
              {t("chat.messageSelected")}
            </Badge>
          ) : null}
          <span className={cn("inline-flex items-center gap-1", !isUser && "ml-auto")}>
            {message.status === "failed" ? <AlertTriangle className="h-3.5 w-3.5" aria-hidden="true" /> : null}
            {message.status === "cancelled" ? <CircleStop className="h-3.5 w-3.5" aria-hidden="true" /> : null}
            {message.status === "completed" ? <CheckCircle2 className="h-3.5 w-3.5" aria-hidden="true" /> : null}
            {statusLabel(message, t)}
          </span>
        </div>
        {!isUser ? <AgentRunOwnerStatus ownerId={message.id} ownerType="session_generation" /> : null}
        <div
          aria-current={selected ? "true" : undefined}
          aria-label={onSelect ? t("chat.selectMessage") : undefined}
          className={cn(
            "rounded-lg border border-transparent px-3 py-2.5 text-sm",
            isUser ? "rounded-tr-sm bg-primary text-primary-foreground" : "rounded-tl-sm bg-[hsl(var(--panel))] shadow-xs",
            message.status === "failed" && "border-destructive/50",
            message.status === "cancelled" && "border-warning/50",
            selected && "ring-2 ring-primary",
          )}
          data-message-id={message.id}
          data-testid="message-bubble"
          onClick={handleBubbleClick}
          onKeyDown={handleBubbleKeyDown}
          role={onSelect ? "button" : undefined}
          tabIndex={onSelect ? 0 : undefined}
        >
          {message.content ? (
            <RichMarkdown>{message.content}</RichMarkdown>
          ) : message.status === "streaming" ? (
            <WaitingIndicator />
          ) : null}
          {message.fileReferences?.length ? (
            <div className="mt-2 flex flex-wrap gap-1.5">
              {message.fileReferences.map((reference) => (
                <span className={cn("inline-flex max-w-full items-center gap-1 rounded-md border px-2 py-1 text-xs", isUser ? "border-primary-foreground/30 bg-primary-foreground/10" : "border-border bg-muted")} key={reference.id}>
                  <FileText className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
                  <span className="truncate">{reference.name}</span>
                  <FileReferenceLines reference={reference} />
                </span>
              ))}
            </div>
          ) : null}
          {message.error ? <p className="mt-2 text-xs text-destructive">{message.error}</p> : null}
          <ThinkingBlock content={message.thinkingContent ?? ""} />
          <ToolUseBlock messageFailed={message.status === "failed"} messageStatus={message.status} onSelectTool={onSelectTool} selectedToolCallId={selectedToolCallId} sessionId={message.sessionId} toolUse={message.toolUse ?? []} />
          <RichBlocks blocks={message.richBlocks ?? []} />
          {!isUser && message.status === "completed" ? (
            <MessageFeedbackControls feedback={message.feedback} messageId={message.id} />
          ) : null}
          {message.status === "completed" && message.content && memoryContext ? (
            <MessageMemoryMenu content={message.content} context={memoryContext} />
          ) : null}
        </div>
      </div>
    </article>
  );
});
