import type { WorkItem, WorkItemPriority, WorkItemSourceKind, WorkItemStage } from "../types/work-board";
import { workItemPriorities, workItemStages } from "../types/work-board";
import type { WorkBoardFilters, WorkItemDueBucket } from "./work-board-filter";

export const workBoardSorts = ["manual", "dueAt", "priority", "updatedAt", "title"] as const;
export type WorkBoardSort = (typeof workBoardSorts)[number];

export const workBoardPresentations = ["board", "list"] as const;
export type WorkBoardPresentation = (typeof workBoardPresentations)[number];

export const workBoardGroupings = ["stage", "none"] as const;
export type WorkBoardGrouping = (typeof workBoardGroupings)[number];

export const ALL_PROJECTS = "all";

/**
 * Task 14.3 lists text, Agent, project, source, priority, due, status, sort, grouping, and
 * presentation. Every one of those except Agent is represented below -- "status" is this
 * domain's own `stage` field (WorkItem never uses the word "status", and the existing
 * `todoBoard.stageFilter`/`todoBoard.stage.*` i18n keys already establish "stage" as this app's
 * vocabulary for it).
 *
 * Agent is deliberately absent, not an oversight: neither `WorkItem` nor `WorkItemSourceLink`
 * (src/types/work-board.ts) nor the Rust model (src-tauri/src/contexts/work_board/models.rs)
 * carries any Agent-attribution field today. A work item's only Agent-adjacent data is its linked
 * sources' own `sourceKind` ("session" | "scheduled_task"), which is already the `source`
 * dimension below, not a distinct Agent identity. Adding a typed `agentId` slot with nothing to
 * populate or match it against would be exactly the fabricated field this task warns against --
 * mirrors use-work-board-actions.ts's own documented omission of a version/conflict field for the
 * same reason: nothing exists yet to build it from.
 */
export interface WorkBoardQuery {
  text: string;
  project: string;
  source: WorkItemSourceKind | "all";
  priority: WorkItemPriority | "all";
  due: WorkItemDueBucket;
  stage: WorkItemStage | "all";
  sort: WorkBoardSort;
  grouping: WorkBoardGrouping;
  presentation: WorkBoardPresentation;
}

export const defaultWorkBoardQuery: WorkBoardQuery = {
  text: "",
  project: ALL_PROJECTS,
  source: "all",
  priority: "all",
  due: "all",
  stage: "all",
  sort: "manual",
  grouping: "stage",
  presentation: "board",
};

/**
 * Only the dimensions that narrow *which* items are visible count as "filters" -- sort, grouping,
 * and presentation change how the same result set is displayed, not what is in it (design.md
 * Decision 11 lists "active filters" and "view/sort" as separate Toolbar clusters, and the
 * pre-existing `filtersActive` this replaces never considered them either).
 */
export function isWorkBoardFilterActive(query: WorkBoardQuery): boolean {
  return Boolean(query.text.trim()) || query.project !== defaultWorkBoardQuery.project
    || query.source !== defaultWorkBoardQuery.source || query.priority !== defaultWorkBoardQuery.priority
    || query.due !== defaultWorkBoardQuery.due || query.stage !== defaultWorkBoardQuery.stage;
}

/** Translates the UI-facing query into the shape `filterWorkItems` understands. `archived` is
 *  threaded separately (not part of `WorkBoardQuery`) because it selects which server-side scope
 *  `useWorkBoardActions` has already fetched, not a local narrowing of that scope. */
export function toWorkBoardFilters(query: WorkBoardQuery, archived: boolean): WorkBoardFilters {
  return {
    archived,
    query: query.text,
    sourceKinds: query.source === "all" ? undefined : [query.source],
    stages: query.stage === "all" ? undefined : [query.stage],
    priorities: query.priority === "all" ? undefined : [query.priority],
    projectPaths: query.project === ALL_PROJECTS ? undefined : [query.project],
    due: query.due,
  };
}

const priorityRank: Record<WorkItemPriority, number> = Object.fromEntries(
  workItemPriorities.map((value, index) => [value, index]),
) as Record<WorkItemPriority, number>;

// Items with no due date sink to the end of a due-date sort rather than cluttering the "soonest
// due" head of the list -- any real dueAt (a 4-digit-year ISO string) sorts before this sentinel.
const NO_DUE_DATE_SORT_SENTINEL = "9999";

const sortComparators: Record<Exclude<WorkBoardSort, "manual">, (left: WorkItem, right: WorkItem) => number> = {
  dueAt: (left, right) => (left.dueAt ?? NO_DUE_DATE_SORT_SENTINEL).localeCompare(right.dueAt ?? NO_DUE_DATE_SORT_SENTINEL),
  priority: (left, right) => priorityRank[right.priority] - priorityRank[left.priority],
  updatedAt: (left, right) => right.updatedAt.localeCompare(left.updatedAt),
  title: (left, right) => left.title.localeCompare(right.title),
};

/**
 * "manual" preserves the pre-existing drag-and-drop rank order (ascending, the only order `rank`
 * has ever meant) -- every other sort is a pure display reorder over the already-filtered set and
 * never writes back to `rank`. Sorting the full cross-stage list once here, before Board
 * presentation splits it by stage, produces the same within-stage order as sorting each stage's
 * own subset independently would (a comparator that only reads the two compared items' own fields
 * is unaffected by which other items are interleaved), so WorkBoardColumn does not need its own
 * per-column sort anymore.
 */
export function sortWorkItems(items: WorkItem[], sort: WorkBoardSort): WorkItem[] {
  if (sort === "manual") return [...items].sort((left, right) => left.rank - right.rank);
  return [...items].sort(sortComparators[sort]);
}

export interface WorkBoardStageGroup {
  stage: WorkItemStage;
  items: WorkItem[];
}

/** Non-empty stage groups only -- unlike WorkBoardColumn's Board-presentation columns (which must
 *  always render, empty or not, since they are drag-drop targets), the "list" presentation's
 *  grouped view has no drop target under an empty header, so an empty group is just clutter. */
export function groupWorkItemsByStage(items: WorkItem[]): WorkBoardStageGroup[] {
  return workItemStages
    .map((stage) => ({ stage, items: items.filter((item) => item.stage === stage) }))
    .filter((group) => group.items.length > 0);
}
