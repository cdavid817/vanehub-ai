import { Minus, Plus } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
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
  onZoomChange,
  spanCount,
  zoom,
}: {
  onZoomChange: (zoom: number) => void;
  spanCount: number;
  zoom: number;
}) {
  const { t } = useTranslation();

  return (
    <div className="flex flex-wrap items-center gap-3 border-b border-border pb-2">
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
  );
}
