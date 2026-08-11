import type { ReactNode } from "react";
import { Circle, MessagesSquare } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import { activeSeatsFromSession } from "../services/session-seats";
import type { Session } from "../types/agent";

export function SessionConversationHeader({
  actions,
  isStreaming,
  session,
}: {
  actions?: ReactNode;
  isStreaming: boolean;
  session: Session | null;
}) {
  const { t } = useTranslation();
  const activeSeatCount = session ? activeSeatsFromSession(session).length : 0;
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
                  : session.interactionMode
                : t("layout.startChat")}
            </p>
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-1">
          <span
            aria-live="polite"
            className={cn(
              "inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs font-medium",
              isStreaming
                ? "border-primary/30 bg-[hsl(var(--nav-active-soft))] text-primary"
                : "border-[hsl(var(--success))]/30 bg-[hsl(var(--success-soft))] text-[hsl(var(--success))]",
            )}
          >
            <Circle aria-hidden="true" className={cn("h-2 w-2 fill-current", isStreaming && "animate-pulse motion-reduce:animate-none")} />
            {isStreaming ? t("layout.generating") : t("layout.ready")}
          </span>
          {actions}
        </div>
      </div>
    </header>
  );
}
