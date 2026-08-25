import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { Network } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSessionSpeakers } from "../hooks/use-session-speakers";
import type { Session } from "../types/agent";
import type { ExecutionObservabilityService } from "../services/execution-observability-service";
import { executionObservabilityService } from "../services/runtime-execution-observability-client";
import type { ExecutionTimeline } from "../types/execution-observability";
import { traceTransitionStream } from "../services/runtime-trace-transition-client";
import { TraceDetailDrawer } from "./trace-detail-drawer";
import { filterTraceSpans, NO_TRACE_FILTERS, type TraceFilters } from "./trace-filters";
import { TraceRunList } from "./trace-run-list";
import { TraceStatusBadge } from "./trace-span-row";
import { TraceToolbar } from "./trace-toolbar";
import { TraceWaterfall } from "./trace-waterfall";
import { useTraceLiveRefresh } from "./use-trace-live-refresh";
import { useTraceSelection } from "./use-trace-selection";
import { WorkspaceState } from "./workspace-state";

export function ExecutionTimelineTab({
  isVisible = true,
  session = null,
  sessionId,
  service = executionObservabilityService,
}: {
  /** False while the panel stays mounted behind another tab. */
  isVisible?: boolean;
  session?: Session | null;
  sessionId: string | null;
  service?: ExecutionObservabilityService;
}) {
  const { t } = useTranslation();
  const speakers = useSessionSpeakers(session);
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  const live = useTraceLiveRefresh({
    isVisible,
    runId: selectedRunId,
    subscribe: traceTransitionStream.subscribe,
  });
  const runs = useInfiniteQuery({
    // Re-read when a *run* transition settles, which is rarer than a span one — re-reading the
    // list once per span is how a busy run makes the whole panel unusable.
    queryKey: ["execution-runs", sessionId, live.runListToken],
    queryFn: ({ pageParam }) => service.listRuns({ limit: 20, pageToken: pageParam, sessionId }),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.nextPageToken ?? undefined,
    // A hidden waterfall keeps every page it has fetched and the run the user selected; what stops
    // is the polling for new runs, which is the only part of it that costs anything.
    enabled: Boolean(sessionId) && isVisible,
  });
  const runItems = useMemo(
    () => runs.data?.pages.flatMap((page) => page.items) ?? [],
    [runs.data?.pages],
  );
  useEffect(() => {
    if (!runItems.some((run) => run.runId === selectedRunId)) {
      setSelectedRunId(runItems[0]?.runId ?? null);
    }
  }, [runItems, selectedRunId]);
  const timeline = useQuery({
    // The token is part of the key, so a settled burst refetches and a quiet panel does not.
    queryKey: ["execution-timeline", selectedRunId, live.refreshToken],
    queryFn: () => service.getTimeline(selectedRunId ?? ""),
    enabled: Boolean(selectedRunId) && isVisible,
  });

  if (!sessionId) return <WorkspaceState kind="unavailable" />;
  if (runs.isLoading) return <WorkspaceState kind="loading" message={t("traces.loading")} />;
  if (runs.isError) return <WorkspaceState kind="error" message={t("traces.error")} />;
  if (!runItems.length) return <WorkspaceState kind="empty" message={t("traces.empty")} />;

  return (
    <div className="grid h-full min-h-0 gap-3 overflow-hidden lg:grid-cols-[minmax(220px,28%)_minmax(0,1fr)]">
      <TraceRunList
        hasNextPage={Boolean(runs.hasNextPage)}
        isFetchingNextPage={runs.isFetchingNextPage}
        onFetchNextPage={() => runs.fetchNextPage()}
        onSelect={setSelectedRunId}
        runs={runItems}
        selectedRunId={selectedRunId}
      />
      <section className="relative flex min-h-0 flex-col rounded-lg border border-border bg-background p-3 sm:p-4">
        {timeline.isLoading ? <WorkspaceState kind="loading" message={t("traces.loading")} /> : null}
        {timeline.isError ? <WorkspaceState kind="error" message={t("traces.error")} /> : null}
        {timeline.data ? (
          <TraceViewport sessionId={sessionId} speakers={speakers} timeline={timeline.data} />
        ) : null}
      </section>
    </div>
  );
}

function TraceViewport({
  sessionId,
  speakers,
  timeline,
}: {
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

  return (
    <div className="grid min-h-0 flex-1 gap-3 lg:grid-cols-[minmax(0,1fr)_minmax(16rem,26rem)]">
      <div className="flex min-h-0 flex-col gap-2">
        <header className="flex flex-wrap items-center gap-2">
          <Network className="h-4 w-4 text-primary" aria-hidden="true" />
          <h2 className="font-semibold">{t("traces.title")}</h2>
          <TraceStatusBadge status={timeline.run.status} />
          <span className="font-mono text-[11px] text-muted-foreground">{timeline.run.traceId}</span>
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
      {selection.detailOpen && selectedSpan ? (
        <TraceDetailDrawer
          events={timeline.events}
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
