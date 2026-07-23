import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { AlertTriangle, Clock3, Network } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../components/ui/badge";
import { cn } from "../lib/utils";
import type { ExecutionObservabilityService } from "../services/execution-observability-service";
import { executionObservabilityService } from "../services/runtime-execution-observability-client";
import type { ExecutionSpanSummary, ExecutionStatus } from "../types/execution-observability";
import { WorkspaceState } from "./workspace-state";

export function ExecutionTimelineTab({
  sessionId,
  service = executionObservabilityService,
}: {
  sessionId: string | null;
  service?: ExecutionObservabilityService;
}) {
  const { i18n, t } = useTranslation();
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  const runs = useInfiniteQuery({
    queryKey: ["execution-runs", sessionId],
    queryFn: ({ pageParam }) => service.listRuns({ limit: 20, pageToken: pageParam, sessionId }),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.nextPageToken ?? undefined,
    enabled: Boolean(sessionId),
  });
  const runItems = useMemo(() => runs.data?.pages.flatMap((page) => page.items) ?? [], [runs.data?.pages]);
  useEffect(() => {
    if (!runItems.some((run) => run.runId === selectedRunId)) {
      setSelectedRunId(runItems[0]?.runId ?? null);
    }
  }, [runItems, selectedRunId]);
  const timeline = useQuery({
    queryKey: ["execution-timeline", selectedRunId],
    queryFn: () => service.getTimeline(selectedRunId ?? ""),
    enabled: Boolean(selectedRunId),
  });

  if (!sessionId) return <WorkspaceState kind="unavailable" />;
  if (runs.isLoading) return <WorkspaceState kind="loading" message={t("traces.loading")} />;
  if (runs.isError) return <WorkspaceState kind="error" message={t("traces.error")} />;
  if (!runItems.length) return <WorkspaceState kind="empty" message={t("traces.empty")} />;

  return (
    <div className="grid h-full min-h-0 gap-3 overflow-hidden lg:grid-cols-[minmax(220px,28%)_minmax(0,1fr)]">
      <aside className="min-h-0 overflow-y-auto rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-2">
        <h2 className="px-2 py-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("traces.runs")}</h2>
        <div className="mt-1 grid gap-1">
          {runItems.map((run) => (
            <button
              aria-pressed={selectedRunId === run.runId}
              className={cn("rounded-md border p-2 text-left", selectedRunId === run.runId ? "border-primary bg-background shadow-sm" : "border-transparent hover:bg-background")}
              key={run.runId}
              onClick={() => setSelectedRunId(run.runId)}
              type="button"
            >
              <div className="flex items-center justify-between gap-2">
                <StatusBadge status={run.status} />
                <time className="text-[11px] text-muted-foreground">{new Intl.DateTimeFormat(i18n.language, { dateStyle: "short", timeStyle: "short" }).format(new Date(run.startedAt))}</time>
              </div>
              <div className="mt-2 truncate font-mono text-xs">{run.agentId ?? run.source}</div>
              <div className="mt-1 text-[11px] text-muted-foreground">{durationLabel(run.durationMs, t)}</div>
            </button>
          ))}
        </div>
        {runs.hasNextPage ? <button className="mt-2 w-full rounded border border-border px-3 py-2 text-xs hover:bg-background" disabled={runs.isFetchingNextPage} onClick={() => runs.fetchNextPage()} type="button">{t(runs.isFetchingNextPage ? "traces.loading" : "traces.loadMore")}</button> : null}
      </aside>
      <section className="min-h-0 overflow-y-auto rounded-lg border border-border bg-background p-3 sm:p-4">
        {timeline.isLoading ? <WorkspaceState kind="loading" message={t("traces.loading")} /> : null}
        {timeline.isError ? <WorkspaceState kind="error" message={t("traces.error")} /> : null}
        {timeline.data ? <TimelineDetail timeline={timeline.data} /> : null}
      </section>
    </div>
  );
}

function TimelineDetail({ timeline }: { timeline: Awaited<ReturnType<ExecutionObservabilityService["getTimeline"]>> }) {
  const { t } = useTranslation();
  const roots = useMemo(() => spanTree(timeline.spans), [timeline.spans]);
  return (
    <div className="grid gap-4">
      <header className="border-b border-border pb-3">
        <div className="flex flex-wrap items-center gap-2">
          <Network className="h-4 w-4 text-primary" aria-hidden="true" />
          <h2 className="font-semibold">{t("traces.title")}</h2>
          <StatusBadge status={timeline.run.status} />
        </div>
        <p className="mt-1 text-xs text-muted-foreground">{t("traces.description")}</p>
      </header>
      <section>
        <h3 className="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("traces.correlation")}</h3>
        <div className="grid gap-2 text-xs sm:grid-cols-2 xl:grid-cols-3">
          <SafeId label={t("traces.runId")} value={timeline.run.runId} />
          <SafeId label={t("traces.traceId")} value={timeline.run.traceId} />
          <SafeId href={timeline.run.sessionId ? `#session-${encodeURIComponent(timeline.run.sessionId)}` : undefined} label={t("traces.sessionId")} value={timeline.run.sessionId} />
          <SafeId href={timeline.run.operationId ? `#operation-${encodeURIComponent(timeline.run.operationId)}` : undefined} label={t("traces.operationId")} value={timeline.run.operationId} />
          <SafeId label={t("traces.agentId")} value={timeline.run.agentId} />
        </div>
      </section>
      <section>
        <h3 className="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("traces.topology")}</h3>
        <div className="grid gap-2">{roots.map((node) => <SpanNode key={node.span.spanId} node={node} />)}</div>
      </section>
      <section>
        <h3 className="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("traces.events")}</h3>
        {timeline.events.length ? <div className="grid gap-2">{timeline.events.map((event) => <div className="rounded border border-border p-2 text-xs" key={`${event.spanId}-${event.sequence}`}><div className="flex flex-wrap items-center justify-between gap-2"><span className="font-mono font-medium">{event.name}</span><time className="text-muted-foreground">{new Date(event.timestamp).toLocaleString()}</time></div><div className="mt-1 text-muted-foreground">{t("traces.eventCorrelation", { sequence: event.sequence, spanId: event.spanId })}</div></div>)}</div> : <p className="text-xs text-muted-foreground">{t("traces.noEvents")}</p>}
      </section>
    </div>
  );
}

interface SpanTreeNode {
  span: ExecutionSpanSummary;
  children: SpanTreeNode[];
}

export function spanTree(spans: ExecutionSpanSummary[]): SpanTreeNode[] {
  const nodes = new Map(spans.map((span) => [span.spanId, { span, children: [] as SpanTreeNode[] }]));
  const roots: SpanTreeNode[] = [];
  for (const node of nodes.values()) {
    const parent = node.span.parentSpanId ? nodes.get(node.span.parentSpanId) : undefined;
    if (parent && parent !== node) parent.children.push(node);
    else roots.push(node);
  }
  return roots;
}

function SpanNode({ node }: { node: SpanTreeNode }) {
  const { t } = useTranslation();
  const gap = node.span.fidelity === "opaque" || node.span.status === "incomplete";
  const stage = node.span.name.includes("mcp") ? "MCP" : node.span.name.includes("tool") ? t("traces.toolStage") : null;
  return (
    <div className="rounded-md border border-border bg-[hsl(var(--panel-muted))] p-3">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="break-words font-mono text-sm font-medium">{node.span.name}</div>
          <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
            <Clock3 className="h-3.5 w-3.5" aria-hidden="true" />
            {durationLabel(node.span.durationMs, t)}
            <span className="font-mono">{node.span.spanId}</span>
          </div>
        </div>
        <div className="flex gap-1.5">
          {stage ? <Badge tone="muted">{stage}</Badge> : null}
          <StatusBadge status={node.span.status} />
          <Badge tone={node.span.fidelity === "native" ? "success" : node.span.fidelity === "opaque" ? "danger" : "warning"}>{t(`traces.fidelity.${node.span.fidelity}`)}</Badge>
        </div>
      </div>
      {gap ? <div className="mt-3 flex gap-2 rounded border p-2 text-xs ucd-status-warning"><AlertTriangle className="h-4 w-4 shrink-0" aria-hidden="true" /><span><strong>{t("traces.gap")}</strong> — {t("traces.gapDescription")}</span></div> : null}
      {node.children.length ? <div className="ml-2 mt-3 grid gap-2 border-l border-border pl-3">{node.children.map((child) => <SpanNode key={child.span.spanId} node={child} />)}</div> : null}
    </div>
  );
}

function StatusBadge({ status }: { status: ExecutionStatus }) {
  const { t } = useTranslation();
  const tone = status === "succeeded" ? "success" : status === "failed" || status === "cancelled" ? "danger" : status === "incomplete" ? "warning" : "muted";
  return <Badge tone={tone}>{t(`traces.status.${status}`)}</Badge>;
}

function SafeId({ href, label, value }: { href?: string; label: string; value?: string | null }) {
  const content = <div className="mt-1 truncate font-mono" title={value ?? "—"}>{value ?? "—"}</div>;
  return <div className="min-w-0 rounded border border-border bg-muted/40 p-2"><div className="text-muted-foreground">{label}</div>{href ? <a className="text-primary underline-offset-2 hover:underline" href={href}>{content}</a> : content}</div>;
}

function durationLabel(duration: number | null | undefined, t: (key: string, values?: { duration: number }) => string) {
  return duration === null || duration === undefined ? t("traces.durationUnknown") : t("traces.duration", { duration });
}
