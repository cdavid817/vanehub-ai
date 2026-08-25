import { useTranslation } from "react-i18next";
import { formatAppDateTime } from "../i18n/format";
import { cn } from "../lib/utils";
import type { ExecutionRunSummary } from "../types/execution-observability";
import { TraceStatusBadge } from "./trace-span-row";

export function TraceRunList({
  hasNextPage,
  isFetchingNextPage,
  onFetchNextPage,
  onSelect,
  runs,
  selectedRunId,
}: {
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  onFetchNextPage: () => void;
  onSelect: (runId: string) => void;
  runs: readonly ExecutionRunSummary[];
  selectedRunId: string | null;
}) {
  const { i18n, t } = useTranslation();

  return (
    <aside className="min-h-0 overflow-y-auto rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-2">
      <h2 className="px-2 py-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
        {t("traces.runs")}
      </h2>
      <div className="mt-1 grid gap-1">
        {runs.map((run) => (
          <button
            aria-pressed={selectedRunId === run.runId}
            className={cn(
              "rounded-md border p-2 text-left focus-visible:outline focus-visible:outline-2 focus-visible:outline-primary",
              selectedRunId === run.runId
                ? "border-primary bg-background shadow-xs"
                : "border-transparent hover:bg-background",
            )}
            key={run.runId}
            onClick={() => onSelect(run.runId)}
            type="button"
          >
            <div className="flex items-center justify-between gap-2">
              <TraceStatusBadge status={run.status} />
              <time className="text-[11px] text-muted-foreground">
                {formatAppDateTime(run.startedAt, i18n.language, {
                  dateStyle: "short",
                  timeStyle: "short",
                })}
              </time>
            </div>
            <div className="mt-2 truncate font-mono text-xs">{run.agentId ?? run.source}</div>
            <div className="mt-1 text-[11px] text-muted-foreground">
              {/* A running run has no duration, and "unknown" is the honest word for it — not a
                  zero, which would read as a run that took no time at all. */}
              {run.durationMs === null || run.durationMs === undefined
                ? t("traces.durationUnknown")
                : t("traces.duration", { duration: run.durationMs })}
            </div>
          </button>
        ))}
      </div>
      {hasNextPage ? (
        <button
          className="mt-2 w-full rounded border border-border px-3 py-2 text-xs hover:bg-background"
          disabled={isFetchingNextPage}
          onClick={onFetchNextPage}
          type="button"
        >
          {t(isFetchingNextPage ? "traces.loading" : "traces.loadMore")}
        </button>
      ) : null}
    </aside>
  );
}
