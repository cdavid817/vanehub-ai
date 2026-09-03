import type { ScheduledTask, ScheduledTaskFrequency, ScheduledTaskRunStatus } from "../types/agent";
import type { ScheduledTaskFrequencyLabel } from "../lib/scheduled-task-recurrence";

export type FrequencyKind = ScheduledTaskFrequency["kind"];

export const frequencyKinds: FrequencyKind[] = ["minutes", "hours", "daily", "weekly", "monthly"];

/**
 * 19.3: the shared row/status primitive `ScheduledTaskRow` and `ScheduledTaskDetail` both call --
 * pulled out of the former single-file `scheduled-tasks-panel.tsx` rather than duplicated, since
 * both surfaces now render the same frequency/next-run/status facts from the same `ScheduledTask`.
 * Framework-agnostic on purpose (no React/i18next import here), matching `goal-presentation.ts`'s
 * own precedent: callers do `t(frequencySummaryParams(...).key, ...)`.
 */

/**
 * The lib's `weekday` field stays a raw 0-6 index (framework-agnostic contract); resolving it to
 * a locale-native name is the caller's job before handing the result to `t()`.
 */
export function frequencySummaryParams(label: ScheduledTaskFrequencyLabel, weekdayNames: string[]) {
  return label.key === "scheduledTasks.frequency.summary.weekly"
    ? { ...label, weekday: weekdayNames[label.weekday] }
    : label;
}

export function initialFrequency(kind: FrequencyKind): ScheduledTaskFrequency {
  switch (kind) {
    case "minutes":
      return { kind, interval: 30 };
    case "hours":
      return { kind, interval: 1 };
    case "daily":
      return { kind, timeOfDay: "09:00" };
    case "weekly":
      return { kind, weekday: 1, timeOfDay: "09:00" };
    case "monthly":
      return { kind, dayOfMonth: 1, timeOfDay: "09:00" };
  }
}

export function formatDateTime(value: string | null, language: string) {
  if (!value) return "-";
  return new Intl.DateTimeFormat(language, {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(value));
}

export function statusClass(status: ScheduledTask["latestStatus"]) {
  if (status === "failed") return "text-destructive";
  if (status === "succeeded") return "text-[hsl(var(--success))]";
  if (status === "running") return "text-primary";
  return "text-muted-foreground";
}

const HISTORY_STATUS_KEYS: Record<ScheduledTaskRunStatus, string> = {
  backfill_running: "scheduledTasks.history.status.backfillRunning",
  backfilled: "scheduledTasks.history.status.backfilled",
  failed: "scheduledTasks.history.status.failed",
  running: "scheduledTasks.history.status.running",
  skipped: "scheduledTasks.history.status.skipped",
  succeeded: "scheduledTasks.history.status.succeeded",
};

/** 19.11: a run row's own status vocabulary is wider than a task's `latestStatus` (see
 *  `ScheduledTaskRunStatus`'s own doc comment in types/agent.ts) -- `scheduledTasks.history.*`
 *  is a deliberately separate key namespace from `scheduledTasks.status.*` above, not a reuse,
 *  because "Latest succeeded" reads correctly for a single current-status badge but would be
 *  actively misleading repeated on every row of a multi-row history list. */
export function historyStatusKey(status: ScheduledTaskRunStatus): string {
  return HISTORY_STATUS_KEYS[status];
}

export function historyStatusClass(status: ScheduledTaskRunStatus): string {
  if (status === "failed") return "text-destructive";
  if (status === "succeeded" || status === "backfilled") return "text-[hsl(var(--success))]";
  if (status === "running" || status === "backfill_running") return "text-primary";
  return "text-muted-foreground";
}
