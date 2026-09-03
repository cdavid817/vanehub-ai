import { X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { ExecutionTimeline } from "../types/execution-observability";
import {
  comparedDelta,
  compareRuns,
  type ComparedCount,
  type ComparedDuration,
} from "./trace-comparison";
import { TraceStatusBadge } from "./trace-span-row";

/**
 * Two runs side by side, as counts and statuses.
 *
 * Every row here can be in one of three states, and the third is the one that matters: the pair is
 * comparable, the pair differs, or the pair cannot be compared at all. A view with only the first
 * two would render "12 vs 8" for two runs that were watched to different depths, and a reader
 * would conclude the second did less work rather than that it was observed less closely.
 */
export function TraceComparisonPanel({
  left,
  onClose,
  right,
}: {
  left: ExecutionTimeline;
  onClose: () => void;
  right: ExecutionTimeline;
}) {
  const { t } = useTranslation();
  const comparison = compareRuns(left, right);

  return (
    <section
      aria-label={t("traces.compare.title")}
      className="flex min-h-0 flex-col gap-2 overflow-y-auto rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-3"
    >
      <header className="flex items-start justify-between gap-2 border-b border-border pb-2">
        <div className="min-w-0">
          <h3 className="text-sm font-semibold">{t("traces.compare.title")}</h3>
          <p className="mt-0.5 text-[11px] text-muted-foreground">
            {t("traces.compare.description")}
          </p>
        </div>
        <button
          aria-label={t("traces.compare.close")}
          className="flex h-7 w-7 shrink-0 items-center justify-center rounded border border-border hover:bg-muted"
          onClick={onClose}
          type="button"
        >
          <X className="h-3.5 w-3.5" aria-hidden="true" />
        </button>
      </header>

      {comparison.observationDiffers ? (
        // Stated once, above everything it qualifies. A reader who misses it reads the whole panel
        // as a set of measurements rather than as two differently-observed samples.
        <p className="ucd-status-warning rounded border px-2 py-1 text-[11px]" role="status">
          {t("traces.compare.observationDiffers")}
        </p>
      ) : null}

      <dl className="grid gap-1 text-[11px]">
        <Row label={t("traces.compare.status")}>
          <span className="flex items-center gap-1.5">
            <TraceStatusBadge status={comparison.status.left} />
            <span aria-hidden="true">→</span>
            <TraceStatusBadge status={comparison.status.right} />
          </span>
        </Row>
        <NumericRow
          compared={comparison.duration}
          format={(value) => t("traces.duration", { duration: value })}
          label={t("traces.compare.duration")}
          // A run still going has no duration; comparing elapsed-so-far to a finished total would
          // report the unfinished one as faster.
          unavailable={t("traces.compare.durationUnavailable")}
        />
        <NumericRow compared={comparison.spans} label={t("traces.compare.spans")} />
        <NumericRow compared={comparison.failures} label={t("traces.compare.failures")} />
        <NumericRow compared={comparison.changes} label={t("traces.compare.changes")} />
        {comparison.toolCounts.map((entry) => (
          <NumericRow
            compared={entry}
            key={entry.kind}
            label={t(`traces.kind.${entry.kind}`)}
          />
        ))}
      </dl>

      <section>
        <h4 className="mb-1 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
          {t("traces.compare.usageQuality")}
        </h4>
        <dl className="grid gap-1 text-[11px]">
          {(["native", "proxied", "inferred", "opaque"] as const).map((fidelity) => (
            <Row key={fidelity} label={t(`traces.fidelity.${fidelity}`)}>
              <span className="tabular-nums">
                {comparison.usageQuality.left[fidelity]} → {comparison.usageQuality.right[fidelity]}
              </span>
            </Row>
          ))}
        </dl>
      </section>
    </section>
  );
}

function Row({ children, label }: { children: React.ReactNode; label: string }) {
  return (
    <div className="grid grid-cols-[minmax(6rem,45%)_minmax(0,1fr)] items-center gap-2">
      <dt className="truncate text-muted-foreground" title={label}>{label}</dt>
      <dd className="min-w-0">{children}</dd>
    </div>
  );
}

function NumericRow({
  compared,
  format = String,
  label,
  unavailable,
}: {
  compared: ComparedCount | ComparedDuration;
  format?: (value: number) => string;
  label: string;
  unavailable?: string;
}) {
  const { t } = useTranslation();
  const delta = comparedDelta(compared);

  if (compared.left === null || compared.right === null) {
    return (
      <Row label={label}>
        <span className="italic text-muted-foreground">
          {unavailable ?? t("traces.compare.notComparable")}
        </span>
      </Row>
    );
  }

  return (
    <Row label={label}>
      <span className="flex flex-wrap items-center gap-1.5 tabular-nums">
        <span>{format(compared.left)}</span>
        <span aria-hidden="true">→</span>
        <span>{format(compared.right)}</span>
        {delta === null ? (
          // Two floors have no meaningful difference between them, so none is shown. Rendering
          // "+4" here would present a gap in observation as a change in behaviour.
          <span className="italic text-muted-foreground">{t("traces.compare.notComparable")}</span>
        ) : delta === 0 ? null : (
          <span className={cn("font-medium", delta > 0 ? "text-destructive" : "text-primary")}>
            {delta > 0 ? `+${delta}` : delta}
          </span>
        )}
      </span>
    </Row>
  );
}
