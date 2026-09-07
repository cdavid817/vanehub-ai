import { useMemo, useState } from "react";
import { Network } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import { useSessionSpeakers } from "../hooks/use-session-speakers";
import type { ExecutionTimeline } from "../types/execution-observability";
import { TraceComparisonPanel } from "./trace-comparison-panel";
import { TraceDetailDrawer } from "./trace-detail-drawer";
import { filterTraceSpans, NO_TRACE_FILTERS, type TraceFilters } from "./trace-filters";
import { TraceStatusBadge } from "./trace-span-row";
import { TraceToolbar } from "./trace-toolbar";
import { TraceWaterfall } from "./trace-waterfall";
import { useTraceSelection } from "./use-trace-selection";
import { WorkspaceState } from "./workspace-state";

/**
 * One run's timeline as the reader sees it: waterfall, filters, detail, and comparison.
 *
 * Split from the tab so that fetching and presenting are separate concerns — the tab owns the
 * queries and what invalidates them, this owns zoom, filters and selection, and neither has to be
 * read to change the other.
 */
export function TraceViewport({
  comparison,
  isLoadingMoreEvents,
  onCloseComparison,
  onLoadMoreEvents,
  sessionId,
  speakers,
  timeline,
}: {
  /** The other run, when the reader asked for a comparison. */
  comparison: ExecutionTimeline | null;
  isLoadingMoreEvents: boolean;
  onCloseComparison: () => void;
  /** Fetches the next page of events, or null when none remain. */
  onLoadMoreEvents: (() => void) | null;
  sessionId: string | null;
  speakers: ReturnType<typeof useSessionSpeakers>;
  timeline: ExecutionTimeline;
}) {
  const { t } = useTranslation();
  const [zoom, setZoom] = useState(1);
  const [filters, setFilters] = useState<TraceFilters>(NO_TRACE_FILTERS);
  const filtered = useMemo(
    () => filterTraceSpans(timeline.spans, filters),
    [filters, timeline.spans],
  );
  // Selection runs over what is visible. A selection pointing at a filtered-out span would open a
  // drawer for a row the reader cannot see, with no way to reach it again.
  const spanIds = useMemo(() => filtered.spans.map((span) => span.spanId), [filtered.spans]);
  const selection = useTraceSelection(spanIds);
  const selectedSpan = filtered.spans.find((span) => span.spanId === selection.selectedId) ?? null;

  const sidePanelOpen = Boolean(comparison) || (selection.detailOpen && selectedSpan !== null);

  return (
    // The second column only exists when something occupies it. Reserving 16-26rem for a drawer
    // that is not open left the waterfall with a fraction of the width it had available, and at
    // 1440px that was enough to squeeze its time axis to nothing.
    <div
      // minmax(0,1fr) and min-w-0 are load-bearing: the waterfall sets a minimum row width from
      // its last measured viewport, and an auto column let that minimum widen the column, which
      // the next measurement then reported back, locking the timeline wider than its panel.
      // Sized by the enclosing section (a container), not the window: the workspace column sits
      // between two collapsible side panels.
      className={cn(
        "grid min-h-0 min-w-0 flex-1 grid-cols-[minmax(0,1fr)] gap-3",
        sidePanelOpen && "grid-rows-[minmax(10rem,auto)_minmax(0,1fr)] @3xl:grid-cols-[minmax(0,1fr)_minmax(16rem,26rem)] @3xl:grid-rows-1",
      )}
    >
      <div className="flex min-h-0 min-w-0 flex-col gap-2">
        <header className="flex flex-wrap items-center gap-2">
          <Network className="h-4 w-4 text-primary" aria-hidden="true" />
          <h2 className="font-semibold">{t("traces.title")}</h2>
          <TraceStatusBadge status={timeline.run.status} />
          <span className="font-mono text-[11px] text-muted-foreground">{timeline.run.traceId}</span>
          {timeline.eventCoverage.truncated ? (
            // Stated once for the run, because that is what the bound applies to. Repeating it in
            // every span's detail claimed each of those spans was missing events, including the
            // ones whose events were all present.
            <span className="ucd-status-warning rounded border px-1.5 py-0.5 text-[11px]">
              {t("traces.section.eventsTruncated")}
            </span>
          ) : null}
          {onLoadMoreEvents ? (
            // Beside the notice rather than buried in a span's detail: the omission is the run's,
            // and telling a reader something is missing without a way to reach it is barely better
            // than not telling them.
            <button
              className="rounded border border-border px-1.5 py-0.5 text-[11px] hover:bg-muted"
              disabled={isLoadingMoreEvents}
              onClick={onLoadMoreEvents}
              type="button"
            >
              {t(isLoadingMoreEvents ? "traces.loading" : "traces.loadMoreEvents")}
            </button>
          ) : null}
        </header>
        <TraceToolbar
          filters={filters}
          hiddenCount={filtered.hiddenCount}
          onFiltersChange={setFilters}
          onZoomChange={setZoom}
          spanCount={filtered.spans.length}
          zoom={zoom}
        />
        {filtered.spans.length ? (
          <TraceWaterfall
            selection={selection}
            spans={filtered.spans}
            speakers={speakers}
            zoom={zoom}
          />
        ) : (
          <WorkspaceState
            kind="empty"
            // The two empty states are different facts, and only the message distinguishes them:
            // a run with no spans recorded nothing, and a filtered-out one recorded plenty.
            message={t(filtered.hiddenCount > 0 ? "traces.allFiltered" : "traces.noSpans")}
          />
        )}
      </div>
      {comparison ? (
        <TraceComparisonPanel left={timeline} onClose={onCloseComparison} right={comparison} />
      ) : null}
      {!comparison && selection.detailOpen && selectedSpan ? (
        <TraceDetailDrawer
          events={timeline.events}
          eventCoverage={timeline.eventCoverage}
          onClose={selection.closeDetail}
          runId={timeline.run.runId}
          sessionId={sessionId}
          span={selectedSpan}
          traceId={timeline.run.traceId}
        />
      ) : null}
    </div>
  );
}
