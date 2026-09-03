import type { ScheduledTask, ScheduledTaskLatestStatus } from "../types/agent";
import type { FrequencyKind } from "./scheduled-task-presentation";

/**
 * 19.4: next-run range bucket over the real `nextRunAt` field, mirroring `work-board-filter.ts`'s
 * own `WorkItemDueBucket` shape -- the established precedent for a "range" filter inside
 * `FilterPopover`, which only ever renders `<select>` fields (no date-range picker exists anywhere
 * in this codebase's list filters). "overdue" is a real, meaningful state here, not just a rounding
 * artifact: `nextRunAt` genuinely sits in the past whenever the app has been closed past a task's
 * due time and the due-task sweep has not yet caught it up on next startup (19.11/19.15's own
 * "backfill" concept) -- filtering to it surfaces exactly the tasks waiting on that catch-up.
 */
export const nextRunRangeBuckets = ["all", "overdue", "next24h", "next7d", "later"] as const;
export type NextRunRangeBucket = (typeof nextRunRangeBuckets)[number];

const NEXT_RUN_SOON_WINDOW_HOURS = 24;
const NEXT_RUN_WEEK_WINDOW_DAYS = 7;
const HOUR_MS = 60 * 60 * 1000;

/** `referenceDate` defaults to "now" but is overridable so callers and tests never depend on the
 *  real clock -- mirrors `matchesDueBucket`'s own `referenceDate` parameter. */
export function matchesNextRunRange(task: ScheduledTask, bucket: NextRunRangeBucket, referenceDate: Date = new Date()): boolean {
  if (bucket === "all") return true;
  const nextRun = new Date(task.nextRunAt).getTime();
  const now = referenceDate.getTime();
  if (bucket === "overdue") return nextRun < now;
  const soonEnd = now + NEXT_RUN_SOON_WINDOW_HOURS * HOUR_MS;
  if (bucket === "next24h") return nextRun >= now && nextRun <= soonEnd;
  const weekEnd = now + NEXT_RUN_WEEK_WINDOW_DAYS * 24 * HOUR_MS;
  if (bucket === "next7d") return nextRun >= now && nextRun <= weekEnd;
  return nextRun > weekEnd;
}

/** Every value `ScheduledTask.latestStatus` can actually take -- mirrors
 *  `MISSION_CONTROL_STATUS_OPTIONS`'s own manually-maintained-subset-of-a-union shape, kept in
 *  sync with `types/agent.ts`'s `ScheduledTaskLatestStatus` by the row's own exhaustive
 *  `scheduledTasks.status.*` usage (scheduled-task-row.tsx), not re-derived here. */
export const scheduledTaskLatestStatuses: ScheduledTaskLatestStatus[] = [
  "never-run", "running", "succeeded", "failed", "skipped",
];

/**
 * 19.4's own literal field list -- enabled, Agent, recurrence, status, and next-run range are all
 * backed by real fields on `ScheduledTask` (`enabled`, `agentId`, `frequency.kind`, `latestStatus`,
 * `nextRunAt`). "Recurrence" is kept as `frequencyKind` / `FrequencyKind`, this codebase's own
 * established field/i18n vocabulary everywhere else a task's schedule kind is shown
 * (`ScheduledTaskFrequency`, `scheduledTasks.frequency.*`, `formatScheduledTaskFrequency`) rather
 * than introducing a second, competing term for the same concept -- mirrors 19.15's own "keep the
 * established term" precedent.
 *
 * "attention" and "project/workspace" (19.4's own remaining listed dimensions) are deliberately
 * absent, not an oversight: confirmed absent on both `ScheduledTask` (types/agent.ts) and
 * `dto::ScheduledTask` (src-tauri/src/commands/sessions/dto.rs) -- no attention concept exists
 * anywhere for a scheduled task (unlike Mission Control's own runs), and a scheduled task has no
 * project/workspace association at all (unlike Loop definitions' real `projectPath`). Building
 * filter controls for either would be exactly the fabricated-field trap `mission-control-query.ts`'s
 * own "Attention" investigation documented and avoided (tasks.md 16.5) -- this task's own "when
 * supported" qualifier covers exactly this case.
 */
export interface ScheduledTaskFilterState {
  search: string;
  agentId: string;
  frequencyKind: FrequencyKind | "";
  enabled: "" | "true" | "false";
  status: ScheduledTaskLatestStatus | "";
  nextRunRange: NextRunRangeBucket;
}

export const defaultScheduledTaskFilterState: ScheduledTaskFilterState = {
  search: "", agentId: "", frequencyKind: "", enabled: "", status: "", nextRunRange: "all",
};

/** Mirrors `isWorkBoardFilterActive`'s/`isMissionControlFilterActive`'s own narrowing-only split --
 *  every field here narrows which tasks are visible (this list has no separate sort/view dimension
 *  to exclude), so all of them count. */
export function isScheduledTaskFilterActive(filter: ScheduledTaskFilterState): boolean {
  return Boolean(filter.search.trim()) || Boolean(filter.agentId) || Boolean(filter.frequencyKind)
    || Boolean(filter.enabled) || Boolean(filter.status)
    || filter.nextRunRange !== defaultScheduledTaskFilterState.nextRunRange;
}

/**
 * 19.4's own "bounded search": a plain case-insensitive substring match over the already-fetched
 * `tasks` list (never a second, unbounded server round trip or full-text index), matching
 * `filterWorkItems`'s own `query` text match and this change's own 6.3-6.5 "bounded X search
 * provider using existing Y" precedent -- scoped to what the caller already has, over name and
 * content only.
 */
export function filterScheduledTasks(tasks: ScheduledTask[], filter: ScheduledTaskFilterState, referenceDate?: Date): ScheduledTask[] {
  const search = filter.search.trim().toLocaleLowerCase();
  return tasks.filter((task) => {
    if (search && !`${task.name} ${task.content}`.toLocaleLowerCase().includes(search)) return false;
    if (filter.agentId && task.agentId !== filter.agentId) return false;
    if (filter.frequencyKind && task.frequency.kind !== filter.frequencyKind) return false;
    if (filter.enabled && task.enabled !== (filter.enabled === "true")) return false;
    if (filter.status && task.latestStatus !== filter.status) return false;
    return matchesNextRunRange(task, filter.nextRunRange, referenceDate);
  });
}
