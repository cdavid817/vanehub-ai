import type { WorkItemPriority, WorkItemSourceKind, WorkItemStage } from "../types/work-board";
import { workItemPriorities, workItemSourceKinds, workItemStages } from "../types/work-board";
import type { WorkItemDueBucket } from "./work-board-filter";
import { workItemDueBuckets } from "./work-board-filter";
import type { WorkBoardGrouping, WorkBoardPresentation, WorkBoardQuery, WorkBoardSort } from "./work-board-query";
import { workBoardGroupings, workBoardPresentations, workBoardSorts } from "./work-board-query";

export const WORK_BOARD_SAVED_VIEW_NAME_MAX_LENGTH = 60;

/**
 * Task 14.5's own qualifier -- "without storing unrestricted description or path content" --
 * deliberately excludes `WorkBoardQuery.text` (free-text search) from this shape. Every other
 * field here is a bounded enum or a project path the reader picked from the board's own existing
 * filter options, not typed prose; a search box invites arbitrary pasted text, and persisting
 * that indefinitely in localStorage is exactly the "unrestricted content" the task warns against.
 * `name` is the one free-text field a saved view needs to be usable at all (it cannot be
 * addressed without a label), so it is capped rather than excluded -- see
 * `captureWorkBoardSavedView`.
 */
export interface WorkBoardSavedView {
  id: string;
  name: string;
  project: string;
  source: WorkItemSourceKind | "all";
  priority: WorkItemPriority | "all";
  due: WorkItemDueBucket;
  stage: WorkItemStage | "all";
  sort: WorkBoardSort;
  grouping: WorkBoardGrouping;
  presentation: WorkBoardPresentation;
}

const STORAGE_KEY = "vanehub.work-board.saved-views.v1";
const CURRENT_VERSION = 1;

interface StoredPayload {
  version: number;
  views: WorkBoardSavedView[];
}

function isOneOf<T extends string>(value: unknown, allowed: readonly T[]): value is T {
  return typeof value === "string" && (allowed as readonly string[]).includes(value);
}

function isValidSavedView(value: unknown): value is WorkBoardSavedView {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<WorkBoardSavedView>;
  return typeof candidate.id === "string" && candidate.id.length > 0
    && typeof candidate.name === "string" && candidate.name.length > 0
    && typeof candidate.project === "string"
    && isOneOf(candidate.source, [...workItemSourceKinds, "all"])
    && isOneOf(candidate.priority, [...workItemPriorities, "all"])
    && isOneOf(candidate.due, workItemDueBuckets)
    && isOneOf(candidate.stage, [...workItemStages, "all"])
    && isOneOf(candidate.sort, workBoardSorts)
    && isOneOf(candidate.grouping, workBoardGroupings)
    && isOneOf(candidate.presentation, workBoardPresentations);
}

/**
 * Versioned (task 14.5): the stored payload's own `version` is checked against
 * `CURRENT_VERSION` before `views` is trusted at all -- mirrors mission-control-view-state.ts's
 * `.v1`-suffixed storage key, the only existing local-storage versioning precedent in this
 * codebase, extended here with a matching in-payload version because Saved Views are a named,
 * user-authored list (not an ephemeral last-filter cache), so a targeted "one entry is bad" path
 * that preserves the reader's other views is worth the extra field mission-control's own simpler
 * "discard the one cached blob" precedent did not need.
 *
 * A whole-payload version mismatch discards every saved view (a future format this build cannot
 * understand at all); an individual malformed entry within an otherwise-current payload is
 * dropped on its own so one corrupt row cannot take down the reader's other saved views. Both
 * fail closed to `[]` rather than throwing -- a saved view is a convenience, not something a
 * parse error should be allowed to crash the board over.
 */
export function readWorkBoardSavedViews(): WorkBoardSavedView[] {
  if (typeof localStorage === "undefined") return [];
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as Partial<StoredPayload>;
    if (parsed.version !== CURRENT_VERSION || !Array.isArray(parsed.views)) return [];
    return parsed.views.filter(isValidSavedView);
  } catch {
    return [];
  }
}

export function writeWorkBoardSavedViews(views: WorkBoardSavedView[]): void {
  if (typeof localStorage === "undefined") return;
  const payload: StoredPayload = { version: CURRENT_VERSION, views };
  localStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
}

/** Captures only bounded, enumerable selections plus the chosen project id -- see this file's own
 *  top-of-file comment for why `query.text` and any item's own description/unbounded content are
 *  never part of this shape. `name` is trimmed and length-capped rather than excluded outright. */
export function captureWorkBoardSavedView(query: WorkBoardQuery, name: string, id: string): WorkBoardSavedView {
  return {
    id,
    name: name.trim().slice(0, WORK_BOARD_SAVED_VIEW_NAME_MAX_LENGTH),
    project: query.project,
    source: query.source,
    priority: query.priority,
    due: query.due,
    stage: query.stage,
    sort: query.sort,
    grouping: query.grouping,
    presentation: query.presentation,
  };
}

/** Restores every stored dimension and resets free-text search to empty -- applying a saved view
 *  is meant to reproduce exactly what was saved, and a leftover unrelated search string would
 *  silently narrow that reproduction further than the view itself specifies. */
export function applyWorkBoardSavedView(view: WorkBoardSavedView): WorkBoardQuery {
  return {
    text: "",
    project: view.project,
    source: view.source,
    priority: view.priority,
    due: view.due,
    stage: view.stage,
    sort: view.sort,
    grouping: view.grouping,
    presentation: view.presentation,
  };
}
