import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSessionSpeakers } from "../hooks/use-session-speakers";
import type { Session } from "../types/agent";
import type { ExecutionObservabilityService } from "../services/execution-observability-service";
import { executionObservabilityService } from "../services/runtime-execution-observability-client";
import type { ExecutionTimeline } from "../types/execution-observability";
import { traceTransitionStream } from "../services/runtime-trace-transition-client";
import { TraceRunList } from "./trace-run-list";
import { TraceViewport } from "./trace-viewport";
import { useTraceLiveRefresh } from "./use-trace-live-refresh";
import { WorkspaceState } from "./workspace-state";

export function ExecutionTimelineTab({
  isVisible = true,
  session = null,
  sessionId,
  service = executionObservabilityService,
  subscribe = traceTransitionStream.subscribe,
}: {
  /** False while the panel stays mounted behind another tab. */
  isVisible?: boolean;
  session?: Session | null;
  sessionId: string | null;
  service?: ExecutionObservabilityService;
  /** The live transition stream. Injectable so a test can drive refreshes without a native event. */
  subscribe?: typeof traceTransitionStream.subscribe;
}) {
  const { t } = useTranslation();
  const speakers = useSessionSpeakers(session);
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  // Optional and off by default. A comparison answers a question a reader arrives with; opening
  // one they did not ask for costs a second timeline read on every run they click through.
  const [compareRunId, setCompareRunId] = useState<string | null>(null);
  useTraceLiveRefresh({
    isVisible,
    // Both open timelines. The compared run advances on its own, and watching only the selected
    // one left the comparison showing whatever it held when it was opened.
    runIds: [selectedRunId, compareRunId],
    sessionId,
    subscribe,
  });
  const runs = useInfiniteQuery({
    // Identity only. A refresh counter here would make every refresh a different query, which
    // starts with no pages and no selection -- see `useTraceLiveRefresh` for what that cascades
    // into. Live updates arrive as invalidations of this same key instead.
    queryKey: ["execution-runs", sessionId],
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
  const newestRunId = runItems[0]?.runId ?? null;
  // What the reader has already seen at the top of the list. Derived from the list rather than
  // from a transition notice: a notice carries no session id and fires on finishes too, so it
  // would announce a newer run when the reader's own run merely ended, or when one started in a
  // session they are not looking at. The list is already scoped to this session.
  const seenNewestRunId = useRef<string | null>(null);
  useEffect(() => {
    // Arriving at a list is not the same as a run appearing while you read it, so the first load
    // establishes the baseline silently.
    if (seenNewestRunId.current === null) seenNewestRunId.current = newestRunId;
  }, [newestRunId]);
  const hasNewerRun =
    newestRunId !== null &&
    seenNewestRunId.current !== null &&
    newestRunId !== seenNewestRunId.current;

  useEffect(() => {
    // Only corrects a selection that has become unreachable, and only once the list has actually
    // loaded. Running against an empty list is how a refresh used to clear the reader's choice:
    // no run matches, so it "corrects" to the first of a list that has not arrived yet.
    if (!runItems.length) return;
    if (!runItems.some((run) => run.runId === selectedRunId)) {
      setSelectedRunId(runItems[0]?.runId ?? null);
    }
  }, [runItems, selectedRunId]);
  const compared = useQuery({
    queryKey: ["execution-timeline", compareRunId],
    queryFn: () => service.getTimeline(compareRunId ?? ""),
    enabled: Boolean(compareRunId) && isVisible,
  });
  // Infinite because a run's events are bounded per response, and a reader told that events were
  // omitted needs a way to reach them. Only the events page: every response also carries the run
  // and its spans, which the later pages repeat and this ignores. That waste buys not adding a
  // second command for a path only a >5000-event run ever takes.
  const timeline = useInfiniteQuery({
    queryKey: ["execution-timeline", selectedRunId],
    queryFn: ({ pageParam }) => service.getTimeline(selectedRunId ?? "", pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.eventCoverage.nextPageToken ?? undefined,
    enabled: Boolean(selectedRunId) && isVisible,
  });
  const timelineData = useMemo<ExecutionTimeline | null>(() => {
    const pages = timeline.data?.pages ?? [];
    const first = pages[0];
    if (!first) return null;
    return {
      ...first,
      events: pages.flatMap((page) => page.events),
      // The last page's coverage, not the first's: once the reader has followed the continuation
      // to the end, nothing is omitted any more and the notice must stop saying otherwise.
      eventCoverage: pages[pages.length - 1]?.eventCoverage ?? first.eventCoverage,
    };
  }, [timeline.data?.pages]);

  if (!sessionId) return <WorkspaceState kind="unavailable" />;
  if (runs.isLoading) return <WorkspaceState kind="loading" message={t("traces.loading")} />;
  if (runs.isError) return <WorkspaceState kind="error" message={t("traces.error")} />;
  if (!runItems.length) return <WorkspaceState kind="empty" message={t("traces.empty")} />;

  return (
    <div className="grid h-full min-h-0 gap-3 overflow-hidden lg:grid-cols-[minmax(220px,28%)_minmax(0,1fr)]">
      <div className="flex min-h-0 flex-col gap-2">
        {hasNewerRun ? (
          // Announced, not applied. The reader chose this run; a newer one appearing is news, not
          // an instruction to move them off what they are reading.
          <button
            className="rounded border border-primary px-2 py-1 text-[11px] text-primary hover:bg-primary/10"
            onClick={() => {
              seenNewestRunId.current = newestRunId;
              setSelectedRunId(newestRunId);
            }}
            type="button"
          >
            {t("traces.newerRun")}
          </button>
        ) : null}
        <TraceRunList
          compareRunId={compareRunId}
          hasNextPage={Boolean(runs.hasNextPage)}
          isFetchingNextPage={runs.isFetchingNextPage}
          onFetchNextPage={() => runs.fetchNextPage()}
          onCompare={(runId) => setCompareRunId((current) => (current === runId ? null : runId))}
          onSelect={(runId) => {
            // Choosing any run acknowledges the list as it stands; the notice is about arrivals
            // since the reader last looked, not about which run they picked.
            seenNewestRunId.current = newestRunId;
            setSelectedRunId(runId);
          }}
          runs={runItems}
          selectedRunId={selectedRunId}
        />
      </div>
      <section className="relative flex min-h-0 flex-col rounded-lg border border-border bg-background p-3 sm:p-4">
        {timeline.isLoading ? <WorkspaceState kind="loading" message={t("traces.loading")} /> : null}
        {timeline.isError ? <WorkspaceState kind="error" message={t("traces.error")} /> : null}
        {timelineData ? (
          <TraceViewport
            comparison={compareRunId && compared.data ? compared.data : null}
            isLoadingMoreEvents={timeline.isFetchingNextPage}
            onCloseComparison={() => setCompareRunId(null)}
            onLoadMoreEvents={timeline.hasNextPage ? () => void timeline.fetchNextPage() : null}
            sessionId={sessionId}
            speakers={speakers}
            timeline={timelineData}
          />
        ) : null}
      </section>
    </div>
  );
}
