import type { ReactNode } from "react";
import { Circle, MessagesSquare, Square } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import {
  interactionModeLabelKey,
  lifecycleLabelKey,
  lifecycleTone,
  type LifecycleTone,
} from "../lib/session-lifecycle";
import { activeSeatsFromSession } from "../services/session-seats";
import { SessionPersonalizationBadge } from "./session-personalization-badge";
import type { Session } from "../types/agent";

const pillClass: Record<LifecycleTone, string> = {
  active: "ucd-status-success",
  pending: "ucd-status-warning",
  danger: "ucd-status-danger",
  neutral: "border-border bg-muted text-muted-foreground",
};

export function SessionConversationHeader({
  actions,
  isStreaming,
  onOpenIm,
  onStop,
  session,
}: {
  actions?: ReactNode;
  isStreaming: boolean;
  onOpenIm?: () => void;
  /**
   * The header's one primary action (design.md: "一个主操作"), present only while there is
   * something to stop — Send itself stays in the composer, not duplicated up here, and a pending
   * recovery already has its own acknowledge affordance in the notice banner below this header.
   */
  onStop?: () => void;
  session: Session | null;
}) {
  const { t } = useTranslation();
  const activeSeatCount = session ? activeSeatsFromSession(session).length : 0;
  // Streaming is an overlay on the session lifecycle, not a replacement for it: reporting only
  // `isStreaming` painted a green "ready" pill over stopped and failed sessions.
  const tone = session ? (isStreaming ? "active" : lifecycleTone(session.lifecycleState)) : null;
  const statusLabel = session
    ? isStreaming ? t("layout.generating") : t(lifecycleLabelKey(session.lifecycleState))
    : null;
  return (
    <header className="shrink-0 border-b border-border bg-[hsl(var(--panel))] px-4 py-2.5" data-testid="session-conversation-header">
      <div className="flex min-h-9 items-center justify-between gap-4">
        <div className="flex min-w-0 items-center gap-3">
          <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border border-primary/30 bg-[hsl(var(--nav-active-soft))] text-primary">
            <MessagesSquare aria-hidden="true" className="h-4 w-4" />
          </span>
          <div className="min-w-0">
            <h2 className="truncate text-sm font-semibold">{session?.title ?? t("layout.noSession")}</h2>
            <p className="mt-0.5 truncate text-xs text-muted-foreground">
              {session
                ? activeSeatCount > 1
                  ? `${t("createSession.agentMode.multi")} · ${t("session.participantCount", { count: activeSeatCount })}`
                  : t(interactionModeLabelKey(session.interactionMode))
                : t("layout.startChat")}
            </p>
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-1">
          {session ? <SessionPersonalizationBadge mode={session.personalizationMode} /> : null}
          {session && onOpenIm ? (
            <button
              aria-label={t("im.session.open")}
              className="hidden h-8 items-center gap-1.5 rounded-md border border-border px-2 text-xs text-foreground hover:bg-muted max-[900px]:flex"
              onClick={onOpenIm}
              type="button"
            >
              <MessagesSquare aria-hidden="true" className="h-3.5 w-3.5" />
              IM
            </button>
          ) : null}
          {tone && statusLabel ? (
            <span
              aria-live="polite"
              className={cn(
                "inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs font-medium",
                pillClass[tone],
              )}
            >
              <Circle aria-hidden="true" className={cn("h-2 w-2 fill-current", isStreaming && "animate-pulse motion-reduce:animate-none")} />
              {statusLabel}
            </span>
          ) : null}
          {isStreaming && onStop ? (
            <button
              aria-label={t("chat.stopTitle")}
              className="flex h-8 items-center gap-1.5 rounded-md border border-border px-2.5 text-xs font-medium text-foreground hover:bg-muted"
              onClick={onStop}
              title={t("chat.stopTitle")}
              type="button"
            >
              <Square aria-hidden="true" className="h-3 w-3 fill-current" />
              {t("chat.stop")}
            </button>
          ) : null}
          {actions}
        </div>
      </div>
    </header>
  );
}
