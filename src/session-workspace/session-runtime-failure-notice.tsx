import { PlugZap, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import type { Session } from "../types/agent";
import type { ChatMessage } from "../types/chat";

/**
 * Deliberately separate from `SessionRecoveryNotice`. That one reports crash-recovery
 * reconciliation of business evidence and asks for an acknowledgement decision; this one reports
 * that the runtime died and offers a retry. Folding them together would make an acknowledgement
 * look like a reconnect.
 */
export function SessionRuntimeFailureNotice({
  messages,
  onRecover,
  recovering,
  session,
}: {
  messages: ChatMessage[];
  onRecover: () => void;
  recovering: boolean;
  session: Session | null;
}) {
  const { t } = useTranslation();
  if (!session || session.archived || session.lifecycleState !== "failed") return null;

  // The reason stays on screen next to the action: recovery restores a usable state, it does not
  // fix whatever failed, so hiding the diagnostic would promise more than the button delivers.
  const reason = [...messages].reverse().find((message) => message.error)?.error ?? null;

  return (
    <section
      aria-live="assertive"
      className="rounded-lg border border-[hsl(var(--danger))]/40 bg-[hsl(var(--danger-soft))] p-3 shadow-xs"
      data-testid="session-runtime-failure-notice"
      role="alert"
    >
      <div className="flex items-start gap-3">
        <span className="mt-0.5 grid h-8 w-8 shrink-0 place-items-center rounded-full bg-background">
          <PlugZap aria-hidden="true" className="h-4 w-4 text-[hsl(var(--danger))]" />
        </span>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <h3 className="text-sm font-semibold">{t("sessionRuntime.failure.title")}</h3>
            <Badge tone="danger">{t("layout.lifecycle.failed")}</Badge>
          </div>
          <p className="mt-1 text-sm leading-5 text-muted-foreground">{t("sessionRuntime.failure.description")}</p>
          {reason ? (
            <p className="mt-1 wrap-break-word text-xs leading-5 text-muted-foreground" title={reason}>{reason}</p>
          ) : null}
        </div>
        <Button disabled={recovering} onClick={onRecover} size="sm" type="button" variant="outline">
          <RefreshCw aria-hidden="true" className={recovering ? "h-3.5 w-3.5 animate-spin motion-reduce:animate-none" : "h-3.5 w-3.5"} />
          {recovering ? t("sessionRuntime.recover.pending") : t("sessionRuntime.recover.action")}
        </Button>
      </div>
    </section>
  );
}
