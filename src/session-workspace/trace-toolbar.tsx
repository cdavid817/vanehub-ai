import { Minus, Plus } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import {
  NO_TRACE_FILTERS,
  TRACE_FIDELITIES,
  TRACE_FILTER_TOGGLES,
  type TraceFilters,
} from "./trace-filters";
import { MAX_ZOOM, MIN_ZOOM } from "./trace-time-scale";

/**
 * The legend entries, in the order they appear in a bar.
 *
 * A legend exists because the bars are coloured, and a colour with no key is a decoration. Each
 * entry names a thing the reader can act on: a failure to look at, the chain that decided the
 * duration, work handed elsewhere, and a span nothing could place.
 */
const LEGEND = [
  { key: "failed", className: "bg-destructive" },
  { key: "criticalPath", className: "bg-primary" },
  { key: "delegated", className: "bg-primary/60" },
  { key: "gap", className: "ucd-status-warning border" },
] as const;

export function TraceToolbar({
  filters,
  hiddenCount,
  onFiltersChange,
  onZoomChange,
  spanCount,
  zoom,
}: {
  filters: TraceFilters;
  /** How many spans the filters removed. Never hidden from the reader. */
  hiddenCount: number;
  onFiltersChange: (filters: TraceFilters) => void;
  onZoomChange: (zoom: number) => void;
  spanCount: number;
  zoom: number;
}) {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col gap-2 border-b border-border pb-2">
      <div aria-label={t("traces.filters")} className="flex flex-wrap items-center gap-1.5" role="group">
        {TRACE_FILTER_TOGGLES.map((toggle) => (
          <button
            aria-pressed={filters[toggle]}
            className={cn(
              "h-6 rounded-full border border-border px-2 text-[11px]",
              filters[toggle] ? "bg-primary text-primary-foreground" : "bg-background hover:bg-muted",
            )}
            key={toggle}
            onClick={() => onFiltersChange({ ...filters, [toggle]: !filters[toggle] })}
            type="button"
          >
            {t(`traces.filter.${toggle}`)}
          </button>
        ))}
        <span aria-hidden="true" className="mx-1 h-4 w-px bg-border" />
        {TRACE_FIDELITIES.map((fidelity) => (
          <button
            aria-pressed={filters.fidelities.includes(fidelity)}
            className={cn(
              "h-6 rounded-full border border-border px-2 text-[11px]",
              filters.fidelities.includes(fidelity)
                ? "bg-primary text-primary-foreground"
                : "bg-background hover:bg-muted",
            )}
            key={fidelity}
            onClick={() => onFiltersChange({
              ...filters,
              fidelities: filters.fidelities.includes(fidelity)
                ? filters.fidelities.filter((item) => item !== fidelity)
                : [...filters.fidelities, fidelity],
            })}
            type="button"
          >
            {t(`traces.fidelity.${fidelity}`)}
          </button>
        ))}
        {hiddenCount > 0 ? (
          <span className="ml-1 flex items-center gap-1.5 text-[11px]">
            {/* A waterfall showing three rows looks the same whether the run had three spans or
                two hundred. Saying how many are hidden is what keeps a narrowed view from reading
                as a run that did almost nothing. */}
            <span className="ucd-status-warning rounded px-1.5 py-0.5">
              {t("traces.hiddenSpans", { count: hiddenCount })}
            </span>
            <button
              className="rounded border border-border px-1.5 py-0.5 hover:bg-muted"
              onClick={() => onFiltersChange(NO_TRACE_FILTERS)}
              type="button"
            >
              {t("traces.clearFilters")}
            </button>
          </span>
        ) : null}
      </div>
      <div className="flex flex-wrap items-center gap-3">
      <div aria-label={t("traces.legend")} className="flex flex-wrap items-center gap-3" role="group">
        {LEGEND.map((entry) => (
          <span className="flex items-center gap-1.5 text-[11px] text-muted-foreground" key={entry.key}>
            <span aria-hidden="true" className={cn("h-2.5 w-4 rounded-sm", entry.className)} />
            {t(`traces.legend.${entry.key}`)}
          </span>
        ))}
        {/* Listed with the colours because it is the same kind of fact, even though it has no bar:
            a span nothing could place is a thing the reader will see and needs a name for. */}
        <span className="text-[11px] italic text-muted-foreground">
          {t("traces.legend.unplaceable")}
        </span>
      </div>
      <div className="ml-auto flex items-center gap-1">
        <span className="text-[11px] tabular-nums text-muted-foreground">
          {t("traces.spanCount", { count: spanCount })}
        </span>
        <button
          aria-label={t("traces.zoomOut")}
          className="flex h-7 w-7 items-center justify-center rounded border border-border hover:bg-muted disabled:opacity-50"
          disabled={zoom <= MIN_ZOOM}
          onClick={() => onZoomChange(zoom / 2)}
          type="button"
        >
          <Minus className="h-3.5 w-3.5" aria-hidden="true" />
        </button>
        <span className="min-w-10 text-center text-[11px] tabular-nums text-muted-foreground">
          {t("traces.zoomLevel", { zoom })}
        </span>
        <button
          aria-label={t("traces.zoomIn")}
          className="flex h-7 w-7 items-center justify-center rounded border border-border hover:bg-muted disabled:opacity-50"
          disabled={zoom >= MAX_ZOOM}
          onClick={() => onZoomChange(zoom * 2)}
          type="button"
        >
          <Plus className="h-3.5 w-3.5" aria-hidden="true" />
        </button>
        </div>
      </div>
    </div>
  );
}
