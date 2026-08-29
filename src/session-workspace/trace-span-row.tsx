import { useTranslation } from "react-i18next";
import { Badge } from "../components/ui/badge";
import { cn } from "../lib/utils";
import type { MessageSpeaker } from "../services/message-speaker";
import type { ExecutionSpanSummary, ExecutionStatus } from "../types/execution-observability";
import { traceSeat } from "./trace-seat";
import { placeSpanBar, type TraceTimeScale } from "./trace-time-scale";

/** How far each depth level indents the label column. */
const INDENT_PX = 12;

export function TraceStatusBadge({ status }: { status: ExecutionStatus }) {
  const { t } = useTranslation();
  const tone = status === "succeeded"
    ? "success"
    : status === "failed" || status === "cancelled"
      ? "danger"
      : status === "incomplete"
        ? "warning"
        : "muted";
  return <Badge tone={tone}>{t(`traces.status.${status}`)}</Badge>;
}

export interface TraceSpanRowProps {
  span: ExecutionSpanSummary;
  depth: number;
  scale: TraceTimeScale;
  selected: boolean;
  speaker: MessageSpeaker | null;
  onSelect: () => void;
}

/**
 * One row: a label column and a bar on the time axis.
 *
 * The accessible name carries everything the bar conveys visually — status, fidelity, kind, and
 * whether the span is still running. A reader using a screen reader gets a chart made of coloured
 * rectangles otherwise, which is to say nothing at all.
 */
export function TraceSpanRow({
  depth,
  onSelect,
  scale,
  selected,
  span,
  speaker,
}: TraceSpanRowProps) {
  const { t } = useTranslation();
  const placement = placeSpanBar(span, scale);
  const gap = span.fidelity === "opaque" || span.status === "incomplete";

  return (
    <div
      aria-current={selected ? "true" : undefined}
      aria-label={spanAccessibleLabel(span, t)}
      className={cn(
        "grid grid-cols-[minmax(10rem,18rem)_minmax(0,1fr)] items-center gap-2 rounded px-1",
        selected ? "bg-primary/10 outline outline-2 outline-primary" : "hover:bg-muted/50",
      )}
      onClick={onSelect}
      role="listitem"
    >
      <div className="flex min-w-0 items-center gap-1.5" style={{ paddingInlineStart: depth * INDENT_PX }}>
        {span.criticalPath ? (
          <span
            aria-hidden="true"
            className="h-3 w-0.5 shrink-0 rounded bg-primary"
            title={t("traces.criticalPath")}
          />
        ) : null}
        <span className="truncate font-mono text-xs">{span.name}</span>
        {speaker ? <Badge tone="muted">{speaker.roleName ?? speaker.agentName}</Badge> : null}
        {span.kind === "unknown" ? null : (
          <Badge tone="muted">{t(`traces.kind.${span.kind}`)}</Badge>
        )}
        {span.attempt !== undefined && span.attempt > 1 ? (
          <Badge tone="warning">{t("traces.attempt", { attempt: span.attempt })}</Badge>
        ) : null}
      </div>
      <div className="relative h-5">
        {placement.kind === "unplaceable" ? (
          // Deliberately not a bar at zero. This span could not be placed on the axis, and drawing
          // it at the origin would say it happened at the start of the run.
          <span className="text-[11px] italic text-muted-foreground">
            {t("traces.unplaceable")}
          </span>
        ) : (
          <span
            aria-hidden="true"
            className={cn(
              "absolute top-1 h-3 rounded-sm",
              gap ? "ucd-status-warning border" : barTone(span),
              // An open bar has no right edge, because where it ends has not happened yet. The
              // gradient is the only honest way to draw "still going" without inventing a number.
              placement.openEnded ? "opacity-70 [mask-image:linear-gradient(to_right,black_60%,transparent)]" : null,
            )}
            style={{ insetInlineStart: placement.leftPx, width: placement.widthPx }}
          />
        )}
      </div>
    </div>
  );
}

function barTone(span: ExecutionSpanSummary): string {
  if (span.status === "failed" || span.status === "cancelled") return "bg-destructive";
  if (span.criticalPath) return "bg-primary";
  if (span.delegated) return "bg-primary/60";
  return "bg-muted-foreground/60";
}

/**
 * Everything the bar says, in words.
 *
 * Includes the absences, because those are the part a colour cannot show: a running span has no
 * duration and a span that could not be placed has no position, and both read as ordinary bars to
 * anyone who cannot see where they sit.
 */
export function spanAccessibleLabel(
  span: ExecutionSpanSummary,
  t: (key: string, values?: Record<string, string | number>) => string,
): string {
  const parts = [
    span.name,
    t(`traces.status.${span.status}`),
    t(`traces.fidelity.${span.fidelity}`),
  ];
  if (span.kind !== "unknown") parts.push(t(`traces.kind.${span.kind}`));
  parts.push(
    span.completedDurationMs === undefined
      ? t("traces.stillRunning")
      : t("traces.duration", { duration: span.completedDurationMs }),
  );
  if (span.startOffsetMs === undefined) parts.push(t("traces.unplaceable"));
  if (span.criticalPath) parts.push(t("traces.criticalPath"));
  if (span.delegated) parts.push(t("traces.delegated"));
  if (span.attempt !== undefined && span.attempt > 1) {
    parts.push(t("traces.attempt", { attempt: span.attempt }));
  }
  return parts.join(", ");
}

/** The seat's speaker, when a span carries one. */
export function spanSpeaker(
  span: ExecutionSpanSummary,
  speakers: Map<string | number, MessageSpeaker>,
): MessageSpeaker | null {
  const seat = traceSeat(span.attributes);
  if (!seat) return null;
  return speakers.get(seat.seatId ?? seat.seatIndex) ?? null;
}
