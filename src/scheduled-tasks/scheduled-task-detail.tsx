import { Loader2, Play } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { formatScheduledTaskFrequency } from "../lib/scheduled-task-recurrence";
import type { AsyncViewState } from "../ui/async/async-view-state";
import type { AgentRegistryEntry, ScheduledTask, ScheduledTaskRun } from "../types/agent";
import { ScheduledTaskCapabilityNotice } from "./scheduled-task-capability-notice";
import { ScheduledTaskHistory } from "./scheduled-task-history";
import { ScheduledTaskOccurrences } from "./scheduled-task-occurrences";
import { formatDateTime, frequencySummaryParams, statusClass } from "./scheduled-task-presentation";
import { ScheduledTaskSessionLink } from "./scheduled-task-session-link";

export interface ScheduledTaskDetailProps {
  task: ScheduledTask | null;
  agent?: AgentRegistryEntry;
  weekdayNames: string[];
  language: string;
  /** 19.10: true only while this task's own on-demand run is in flight -- the panel tracks at
   *  most one running task id at a time, the same shape `confirmingDeleteId` already uses for
   *  "at most one row has this pending action." */
  isRunningNow: boolean;
  runNowError: string | null;
  onRunNow: () => void;
  /** 19.11: `useScheduledTaskHistory`'s own state/reload for the currently selected task. */
  history: AsyncViewState<ScheduledTaskRun[]>;
  onRetryHistory: () => void;
  /** 19.11: `useScheduledTaskHistory`'s own pagination trio, threaded straight through to
   *  `ScheduledTaskHistory` -- see that component's own doc comment. */
  historyHasMore: boolean;
  historyLoadingMore: boolean;
  onLoadMoreHistory: () => void;
  /** See `ScheduledTaskSessionLink`'s own doc comment: optional, and not wired to any real
   *  navigation by this task batch -- that reaches into `src/main-layout/`, out of scope here. */
  onOpenSession?: (sessionId: string) => void;
}

/**
 * 19.6: the route-backed task detail, composing 19.3's own placeholder fields (name, agent,
 * frequency, next-run, status) with the four pieces that placeholder explicitly deferred (its own
 * doc comment): future occurrence preview (19.12, `ScheduledTaskOccurrences`), capability notice
 * (19.6/19.15, `ScheduledTaskCapabilityNotice`), an extended latest-Run section (below, building
 * on the existing Run-now button rather than duplicating it), and full run history (19.11,
 * `ScheduledTaskHistory`). Still reuses `scheduled-task-presentation.ts` and
 * `formatScheduledTaskFrequency` exactly as `ScheduledTaskRow` does for the fields that were
 * already correct -- nothing here recomputes a fact a sibling component already owns.
 */
export function ScheduledTaskDetail({
  agent, history, historyHasMore, historyLoadingMore, isRunningNow, language, onLoadMoreHistory, onOpenSession, onRetryHistory, onRunNow, runNowError, task, weekdayNames,
}: ScheduledTaskDetailProps) {
  const { t } = useTranslation();

  if (!task) {
    return (
      <div className="grid content-start gap-3 rounded-lg border border-dashed border-border p-4 text-center text-sm text-muted-foreground" data-testid="scheduled-task-detail">
        {t("scheduledTasks.detailEmpty")}
      </div>
    );
  }

  const frequencyLabel = formatScheduledTaskFrequency(task.frequency);

  return (
    <div className="grid content-start gap-4 rounded-lg border border-border p-3" data-testid="scheduled-task-detail">
      <div className="grid gap-3">
        <div className="flex items-start justify-between gap-2">
          <h4 className="text-xs font-semibold uppercase text-muted-foreground">{t("scheduledTasks.detailTitle")}</h4>
          <span className={`shrink-0 text-xs font-medium ${task.enabled ? "text-foreground" : "text-muted-foreground"}`}>
            {task.enabled ? t("scheduledTasks.enabled") : t("scheduledTasks.disabled")}
          </span>
        </div>
        <div className="min-w-0">
          <div className="truncate text-sm font-medium">{task.name}</div>
          <div className="mt-1 text-xs text-muted-foreground">{agent?.displayName ?? task.agentId}</div>
        </div>
        <div className="grid gap-1.5 text-xs text-muted-foreground">
          <span>{t(frequencyLabel.key, frequencySummaryParams(frequencyLabel, weekdayNames))}</span>
          <span>{t("scheduledTasks.nextRun", { time: formatDateTime(task.nextRunAt, language) })}</span>
          <span className={statusClass(task.latestStatus)}>
            {t(`scheduledTasks.status.${task.latestStatus}`)}
          </span>
        </div>
      </div>

      <ScheduledTaskOccurrences enabled={task.enabled} frequency={task.frequency} language={language} nextRunAt={task.nextRunAt} />

      <ScheduledTaskCapabilityNotice agent={agent} agentId={task.agentId} />

      <div className="grid gap-2" data-testid="scheduled-task-latest-run">
        <h4 className="text-xs font-semibold uppercase text-muted-foreground">{t("scheduledTasks.latestRun.title")}</h4>
        {task.latestRunAt ? (
          <div className="grid gap-1 text-xs text-muted-foreground">
            <span>{t("scheduledTasks.latestRun.at", { time: formatDateTime(task.latestRunAt, language) })}</span>
            <div className="flex items-center gap-2">
              <span>{t("scheduledTasks.latestRun.session")}</span>
              <ScheduledTaskSessionLink onOpenSession={onOpenSession} sessionId={task.latestRunSessionId} />
            </div>
            {task.latestError ? <p className="text-destructive" role="alert">{task.latestError}</p> : null}
          </div>
        ) : (
          <p className="text-xs text-muted-foreground">{t("scheduledTasks.latestRun.none")}</p>
        )}
        <div className="flex items-center justify-between gap-3">
          <Button className="h-8 px-3 text-xs" disabled={isRunningNow} onClick={onRunNow} type="button">
            {isRunningNow ? <Loader2 className="h-3.5 w-3.5 animate-spin" aria-hidden="true" /> : <Play className="h-3.5 w-3.5" aria-hidden="true" />}
            {t("scheduledTasks.runNow")}
          </Button>
        </div>
        {runNowError ? (
          <p className="text-xs text-destructive" data-testid="scheduled-task-run-now-error" role="alert">{runNowError}</p>
        ) : null}
      </div>

      <ScheduledTaskHistory
        hasMore={historyHasMore}
        language={language}
        loadingMore={historyLoadingMore}
        onLoadMore={onLoadMoreHistory}
        onOpenSession={onOpenSession}
        onRetry={onRetryHistory}
        state={history}
      />
    </div>
  );
}
