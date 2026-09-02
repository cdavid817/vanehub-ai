import type { WorkItem, WorkItemFilters } from "../types/work-board";

export const workItemDueBuckets = ["all", "overdue", "dueSoon", "noDueDate"] as const;
export type WorkItemDueBucket = (typeof workItemDueBuckets)[number];

// A saved/typed "due soon" bucket needs a fixed boundary somewhere; a week matches how this
// codebase already frames near-term work elsewhere (e.g. formatAppWeekdayNames' own 7-day week)
// and is short enough that "soon" still reads as urgent rather than "eventually".
const DUE_SOON_WINDOW_DAYS = 7;

function startOfDay(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

/**
 * Buckets `item.dueAt` against `referenceDate` (defaults to "now"; overridable so callers and
 * tests never depend on the real clock). Compared as local calendar days, not raw instants --
 * WorkItemForm's own `due` field is a bare `type="date"` input with no time-of-day component, so
 * a due date must bucket by the reader's local day, not flip between buckets the instant UTC
 * midnight passes in some other timezone.
 */
export function matchesDueBucket(item: WorkItem, bucket: WorkItemDueBucket, referenceDate: Date = new Date()): boolean {
  if (bucket === "all") return true;
  if (bucket === "noDueDate") return !item.dueAt;
  if (!item.dueAt) return false;
  const due = startOfDay(new Date(item.dueAt));
  const today = startOfDay(referenceDate);
  if (bucket === "overdue") return due.getTime() < today.getTime();
  const soonEnd = new Date(today);
  soonEnd.setDate(soonEnd.getDate() + DUE_SOON_WINDOW_DAYS);
  return due.getTime() >= today.getTime() && due.getTime() <= soonEnd.getTime();
}

/**
 * Superset of the canonical `WorkItemFilters` (src/types/work-board.ts) with the due-date bucket
 * this board's own local filtering understands. Kept local to this file rather than folded into
 * the shared type: `due` is a UI-invented presentation bucket over the raw `dueAt` timestamp, not
 * a concept the backend or either service client (tauri/web) has ever needed to know about --
 * every real caller (use-work-board-actions.ts) only ever sends `{ archived }` over the service
 * boundary and applies the rest, including this, to the already-fetched list in memory.
 */
export interface WorkBoardFilters extends WorkItemFilters {
  due?: WorkItemDueBucket;
  /** Test/story override for "now"; production callers never set this. */
  dueReferenceDate?: Date;
}

export function filterWorkItems(items: WorkItem[], filters: WorkBoardFilters): WorkItem[] {
  const query = filters.query?.trim().toLocaleLowerCase();
  return items.filter((item) => {
    if (item.archived !== Boolean(filters.archived)) return false;
    if (query && !`${item.title} ${item.description} ${item.projectPath ?? ""}`.toLocaleLowerCase().includes(query)) return false;
    if (filters.sourceKinds?.length && !item.sources.some((source) => filters.sourceKinds?.includes(source.sourceKind))) return false;
    if (filters.stages?.length && !filters.stages.includes(item.stage)) return false;
    if (filters.priorities?.length && !filters.priorities.includes(item.priority)) return false;
    if (filters.due && !matchesDueBucket(item, filters.due, filters.dueReferenceDate)) return false;
    return !filters.projectPaths?.length || Boolean(item.projectPath && filters.projectPaths.includes(item.projectPath));
  });
}
