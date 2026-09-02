import { useTranslation } from "react-i18next";
import { formatAppNumber } from "../i18n/format";
import type { ExecutionSpanSummary } from "../types/execution-observability";

/**
 * One span row shared by the Timeline/Tools/Files facets — they differ only in which spans they
 * pass in (see `use-execution-timeline.ts` and each facet's own kind filter), not in how a single
 * span is rendered.
 *
 * `showKind` is true only for Timeline: Tools/Files each already imply their own kind by which facet
 * the row is rendered inside, so repeating a kind badge there would be redundant rather than
 * informative. Reuses the same `traces.*` i18n vocabulary the pre-existing trace waterfall
 * (`trace-span-row.tsx`) already established for this exact type — status, kind, critical path, and
 * duration/offset wording stay identical wherever an `ExecutionSpanSummary` is shown, rather than
 * growing a second, driftable vocabulary for the same fields.
 */
export function ExecutionSpanRow({ showKind, span }: { showKind: boolean; span: ExecutionSpanSummary }) {
  const { t, i18n } = useTranslation();
  const offset = span.startOffsetMs === undefined
    ? t("traces.unplaceable")
    : t("traces.axisTick", { offset: formatAppNumber(span.startOffsetMs, i18n.language) });
  const duration = span.completedDurationMs === undefined
    ? t("traces.stillRunning")
    : t("traces.duration", { duration: formatAppNumber(span.completedDurationMs, i18n.language) });

  return (
    <li className="rounded-md border border-border bg-card px-2 py-1.5 text-xs" data-testid="mission-control-execution-span-row">
      <div className="flex flex-wrap items-center justify-between gap-1">
        <span className="flex min-w-0 items-center gap-1.5 font-medium">
          {span.criticalPath ? (
            <span aria-hidden="true" className="h-3 w-0.5 shrink-0 rounded bg-primary" title={t("traces.criticalPath")} />
          ) : null}
          <span className="truncate">{span.name}</span>
          {showKind && span.kind !== "unknown" ? (
            <span className="shrink-0 rounded bg-muted px-1 text-[10px] uppercase text-muted-foreground">{t(`traces.kind.${span.kind}`)}</span>
          ) : null}
        </span>
        <span className="shrink-0 text-muted-foreground">{t(`traces.status.${span.status}`)}</span>
      </div>
      <p className="mt-0.5 text-muted-foreground">{offset} · {duration}</p>
    </li>
  );
}
