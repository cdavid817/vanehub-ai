import { CheckCircle2, Loader2, PauseCircle } from "lucide-react";
import { useTranslation } from "react-i18next";

export type TurnStatus =
  | { kind: "agent"; seatId?: string; holderName: string; depth: number; maxDepth: number }
  | { kind: "waiting-human"; seatId?: string; requesterName: string; waitedMinutes: number }
  | { kind: "round-complete"; seatId?: string; finisherName: string };

/**
 * Always visible in a multi-seat session, because the one question a reader has is "who are we
 * waiting on". Only the paused state is emphasised: an informational handoff must not look like an
 * interruption, or Agents get blamed for using it.
 */
export function TurnStatusBar({ status }: { status: TurnStatus }) {
  const { t } = useTranslation();

  if (status.kind === "waiting-human") {
    return (
      <div className="sticky top-0 z-10 flex items-center gap-2 border-b border-warning/40 bg-warning/10 px-3 py-1.5 text-xs">
        <PauseCircle aria-hidden="true" className="h-3.5 w-3.5 text-warning" />
        <span className="font-medium">
          {t("chat.turn.waitingHuman", { name: status.requesterName })}
        </span>
        <span className="text-muted-foreground">
          {t("chat.turn.waitedFor", { minutes: status.waitedMinutes })}
        </span>
      </div>
    );
  }

  if (status.kind === "round-complete") {
    return (
      <div className="sticky top-0 z-10 flex items-center gap-2 border-b border-border px-3 py-1.5 text-xs">
        <CheckCircle2 aria-hidden="true" className="h-3.5 w-3.5 text-[hsl(var(--success))]" />
        <span>{t("chat.turn.roundComplete", { name: status.finisherName })}</span>
      </div>
    );
  }

  // The chain counter is quiet until it nears its limit, so a truncated chain is never a surprise.
  const nearLimit = status.depth >= status.maxDepth - 2;
  return (
    <div className="sticky top-0 z-10 flex items-center gap-2 border-b border-border px-3 py-1.5 text-xs">
      <Loader2 aria-hidden="true" className="h-3.5 w-3.5 animate-spin text-primary" />
      <span className="font-medium">{t("chat.turn.agentSpeaking", { name: status.holderName })}</span>
      <span className={nearLimit ? "text-warning" : "text-muted-foreground"}>
        {t("chat.turn.chainDepth", { depth: status.depth, maxDepth: status.maxDepth })}
      </span>
    </div>
  );
}
