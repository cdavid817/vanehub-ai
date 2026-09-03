import { useTranslation } from "react-i18next";
import { computeNextScheduledOccurrences } from "../lib/scheduled-task-recurrence";
import type { ScheduledTaskFrequency } from "../types/agent";
import { formatDateTime } from "./scheduled-task-presentation";

const OCCURRENCE_COUNT = 5;

export interface ScheduledTaskOccurrencesProps {
  frequency: ScheduledTaskFrequency;
  nextRunAt: string;
  enabled: boolean;
  language: string;
}

/**
 * 19.12: "next five occurrences," anchored at the task's own already-computed `nextRunAt` --
 * not at "now" -- and extended by feeding each result back into `computeNextScheduledRun`, the
 * exact function the due-task sweep and every `next_run_at` recompute already use. Anchoring at
 * "now" instead would be wrong for `minutes`/`hours` frequencies specifically: their real
 * `nextRunAt` sits on a grid set by whenever the task last actually ran or was last edited/
 * enabled (`compute_next_run(&frequency, Local::now())` at that moment), not on a grid aligned to
 * whatever instant this preview happens to render at -- an interval task due at 9:30 would
 * otherwise preview 9:45/10:15/... from a 9:15 render, silently disagreeing with the "Next 9:30"
 * already shown one line above it. Starting from the real `nextRunAt` makes every entry after the
 * first a provable continuation of the one value execution has already committed to.
 *
 * A disabled task's own `nextRunAt` is stale by design (`set_scheduled_task_enabled` only
 * recomputes it when *enabling*, never when disabling), so chaining from it would preview times
 * that re-enabling would not actually reproduce -- re-enabling recomputes fresh from
 * `Local::now()` at that moment, not from the frozen value. Showing that chain anyway would be a
 * fabricated preview, not an honest one, so a disabled task gets a plain explanation instead.
 */
export function ScheduledTaskOccurrences({ enabled, frequency, language, nextRunAt }: ScheduledTaskOccurrencesProps) {
  const { t } = useTranslation();

  let occurrences: string[] | null = null;
  if (enabled) {
    try {
      occurrences = [nextRunAt, ...computeNextScheduledOccurrences(frequency, OCCURRENCE_COUNT - 1, new Date(nextRunAt))];
    } catch {
      occurrences = null;
    }
  }

  return (
    <div className="grid gap-1.5" data-testid="scheduled-task-occurrences">
      <h4 className="text-xs font-semibold uppercase text-muted-foreground">{t("scheduledTasks.occurrences.title")}</h4>
      {!enabled ? (
        <p className="text-xs text-muted-foreground">{t("scheduledTasks.occurrences.disabled")}</p>
      ) : occurrences === null || occurrences.length === 0 ? (
        <p className="text-xs text-muted-foreground">{t("scheduledTasks.occurrences.unavailable")}</p>
      ) : (
        <ol className="grid gap-1 text-xs text-muted-foreground">
          {occurrences.map((occurrence) => <li key={occurrence}>{formatDateTime(occurrence, language)}</li>)}
        </ol>
      )}
    </div>
  );
}
