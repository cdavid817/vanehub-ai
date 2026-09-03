import { useTranslation } from "react-i18next";
import { AsyncBoundary } from "../ui/async/AsyncBoundary";
import type { AsyncViewState } from "../ui/async/async-view-state";
import type { ScheduledTaskRun } from "../types/agent";
import { formatDateTime, historyStatusClass, historyStatusKey } from "./scheduled-task-presentation";
import { ScheduledTaskSessionLink } from "./scheduled-task-session-link";

export interface ScheduledTaskHistoryProps {
  state: AsyncViewState<ScheduledTaskRun[]>;
  language: string;
  onRetry: () => void;
  onOpenSession?: (sessionId: string) => void;
  /** 19.11: whether the service reports a further page beyond `state.data`. Optional so a caller
   *  with nothing to page (e.g. a hand-built test fixture) still renders without a "load more"
   *  control -- mirrors `EvaluationArenaList`'s own identical optional trio. */
  hasMore?: boolean;
  loadingMore?: boolean;
  onLoadMore?: () => void;
}

/**
 * 19.11: run history, one row per `ScheduledTaskRun` returned by `listScheduledTaskRuns` -- a
 * real, multi-row query on the Tauri side (`list_scheduled_task_runs`, `ORDER BY started_at DESC,
 * id DESC`) and, as of this same pass, a genuinely multi-row Web/mock too
 * (`web-scheduled-task-client.ts`).
 *
 * Trigger classification is honestly partial, not fabricated: `backfilled`/`backfill_running` are
 * real, distinguishable status values the sweep itself writes (`mark_task_succeeded`'s own `CASE
 * status WHEN 'backfill_running' THEN 'backfilled' ELSE 'succeeded' END`), so "was this run caught
 * up at app startup" is knowable and shown via `historyStatusKey`. A manual ("Run now") dispatch
 * and a normal on-time sweep run are genuinely indistinguishable in the data as it exists today --
 * both simply read `succeeded`, at the same granularity of timestamp -- so both render identically
 * rather than guessing a "manual" label onto data that cannot actually support it.
 *
 * Real pagination, closing this same task's own previously-open gap: `listEvaluationArenas` had an
 * identical `list(0, 100)` hard-coded-window gap before this same OpenSpec change's own 18.6 closed
 * it, and this follows the exact cross-layer shape that pass established (Tauri command
 * cursor/limit params, a `{ items, nextCursor }` service contract, real slicing in the Web mock).
 * `hasMore`/`onLoadMore` render the same "Load more" control `EvaluationArenaList` does, rather
 * than a bounded note that only ever fired on landing exactly on a hard-coded cap.
 */
export function ScheduledTaskHistory({ hasMore = false, language, loadingMore = false, onLoadMore, onOpenSession, onRetry, state }: ScheduledTaskHistoryProps) {
  const { t } = useTranslation();
  return (
    <div className="grid gap-2" data-testid="scheduled-task-history">
      <h4 className="text-xs font-semibold uppercase text-muted-foreground">{t("scheduledTasks.history.title")}</h4>
      <AsyncBoundary
        emptyState={{ title: t("scheduledTasks.history.empty") }}
        isEmpty={(runs) => runs.length === 0}
        onRetry={onRetry}
        state={state}
      >
        {(runs) => (
          <div className="grid gap-2">
            <ul className="grid gap-1.5">
              {runs.map((run) => (
                <li className="grid gap-1 rounded-md border border-border p-2 text-xs" data-testid={`scheduled-task-history-row-${run.id}`} key={run.id}>
                  <div className="flex items-center justify-between gap-2">
                    <span className={historyStatusClass(run.status)}>{t(historyStatusKey(run.status))}</span>
                    <ScheduledTaskSessionLink onOpenSession={onOpenSession} sessionId={run.sessionId} />
                  </div>
                  <div className="flex flex-wrap items-center gap-x-3 gap-y-0.5 text-muted-foreground">
                    <span>{t("scheduledTasks.history.startedAt", { time: formatDateTime(run.startedAt, language) })}</span>
                    <span>{t("scheduledTasks.history.completedAt", { time: formatDateTime(run.completedAt, language) })}</span>
                  </div>
                  {/* Safe failure: a failed/skipped run's own error renders as plain text (React
                      escapes it by default) rather than being swallowed -- the reader sees exactly
                      why, never a silently blank row. */}
                  {run.error ? <p className="text-destructive" role="alert">{run.error}</p> : null}
                </li>
              ))}
            </ul>
            <p className="text-xs text-muted-foreground">{t("scheduledTasks.history.summary", { count: runs.length })}</p>
            {hasMore ? (
              <button
                className="mt-1 w-full rounded-md border border-input px-3 py-1.5 text-xs disabled:opacity-50"
                data-testid="scheduled-task-history-load-more"
                disabled={loadingMore}
                onClick={onLoadMore}
                type="button"
              >
                {t(loadingMore ? "scheduledTasks.history.loadingMore" : "scheduledTasks.history.loadMore")}
              </button>
            ) : null}
          </div>
        )}
      </AsyncBoundary>
    </div>
  );
}
