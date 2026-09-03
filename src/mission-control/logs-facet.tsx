import { useTranslation } from "react-i18next";
import { formatAppDateTime } from "../i18n/format";
import { cn } from "../lib/utils";
import type { MissionControlRunSummary } from "../types/mission-control";
import type { SessionLogEntry } from "../types/session-workspace";
import { AsyncBoundary } from "../ui/async/AsyncBoundary";
import { LOG_LIST_LIMIT, useMissionControlLogs } from "./use-mission-control-logs";

/** Same raw-level, no-translation convention `log-entry-article.tsx` already established for the
 *  full Session Log tab — a level is a fixed, small vocabulary, and it keeps the two surfaces from
 *  drifting on what a level is called. */
function LogEntryRow({ entry, language }: { entry: SessionLogEntry; language: string }) {
  return (
    <li className="rounded-md border border-border bg-card px-2 py-1.5 text-xs" data-log-id={entry.id}>
      <div className="flex items-center justify-between gap-2">
        <span className={cn(
          "font-semibold uppercase",
          entry.level === "error" && "text-destructive",
          entry.level === "warn" && "text-primary",
        )}>
          {entry.level}
        </span>
        <time className="text-muted-foreground">{formatAppDateTime(entry.timestamp, language, { dateStyle: "short", timeStyle: "medium" })}</time>
      </div>
      <p className="mt-0.5 text-muted-foreground">{entry.category}</p>
      <p className="mt-0.5 whitespace-pre-wrap">{entry.message}</p>
    </li>
  );
}

/**
 * The Logs facet (16.9/16.11/16.12): the first genuinely new facet built in this pass, not just a
 * placeholder swap. Backed by a real, dual-backend, bounded, run-correlated query
 * (`agentService.listSessionLogs`, see `use-mission-control-logs.ts` for the full join reasoning) —
 * a bounded summary of recent entries, not the full paginated Session Log tab this run's own session
 * already has elsewhere.
 */
export function LogsFacet({ run }: { run: MissionControlRunSummary }) {
  const { t, i18n } = useTranslation();
  const { reload, ...state } = useMissionControlLogs(run, t("missionControl.logs.empty"), t("missionControl.logs.error"));

  return (
    <div className="mt-4 space-y-2" data-testid="mission-control-logs-facet">
      <AsyncBoundary
        emptyState={{ title: t("missionControl.logs.noEntries") }}
        isEmpty={(data) => data.entries.length === 0}
        onRetry={reload}
        state={state}
        unavailableState={{ title: t("missionControl.logs.empty") }}
      >
        {(data) => (
          <>
            <ul className="space-y-1" data-testid="mission-control-logs-entries">
              {data.entries.map((entry) => <LogEntryRow entry={entry} key={entry.id} language={i18n.language} />)}
            </ul>
            {data.truncated ? (
              <p className="text-xs text-muted-foreground">{t("missionControl.logs.cappedNote", { count: LOG_LIST_LIMIT })}</p>
            ) : null}
          </>
        )}
      </AsyncBoundary>
    </div>
  );
}
