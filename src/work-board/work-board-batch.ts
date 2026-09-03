import type { WorkItem, WorkItemStage } from "../types/work-board";

/**
 * 14.12: the two batch actions the task itself names ("batch move/archive"). Modeled as a
 * discriminated union rather than two independent flags, since "move" is meaningless without a
 * target stage and the two are never run at once.
 */
export type WorkBoardBatchAction = { kind: "archive" } | { kind: "move"; stage: WorkItemStage };

export type WorkBoardBatchIneligibleReason = "archived" | "sameStage";
export type WorkBoardBatchOutcome = "success" | "error" | "skipped";

/**
 * The only two eligibility rules `WorkItem`'s own real fields can support -- checked against the
 * type (`src/types/work-board.ts`), not invented: `archived` is the one boolean every mutation
 * already branches on (`use-work-board-actions.ts`'s own `mutateCard`), and "already in this
 * stage" mirrors `WorkItemStageMenu`'s own existing no-op-on-reselect rule (work-item-stage-
 * menu.tsx's `select()`) rather than a new, second definition of the same fact. An archived item
 * is ineligible for *both* actions: the single-card UI already hides the stage menu entirely for
 * an archived card (14.6, only Restore/Delete remain), so a batch move must honor that same rule.
 */
export function batchIneligibleReason(item: WorkItem, action: WorkBoardBatchAction): WorkBoardBatchIneligibleReason | null {
  if (item.archived) return "archived";
  if (action.kind === "move" && item.stage === action.stage) return "sameStage";
  return null;
}

export function isBatchEligible(item: WorkItem, action: WorkBoardBatchAction): boolean {
  return batchIneligibleReason(item, action) === null;
}

export interface WorkBoardBatchPartition {
  eligible: WorkItem[];
  ineligible: WorkItem[];
}

/** The "eligibility preview" (14.12's own wording): which of the currently selected items can
 *  actually receive `action`, computed live so changing the Move-to target recomputes it without
 *  re-running anything. */
export function partitionBatchSelection(items: WorkItem[], selectedIds: ReadonlySet<string>, action: WorkBoardBatchAction): WorkBoardBatchPartition {
  const selected = items.filter((item) => selectedIds.has(item.id));
  return {
    eligible: selected.filter((item) => isBatchEligible(item, action)),
    ineligible: selected.filter((item) => !isBatchEligible(item, action)),
  };
}

/**
 * Drops any selected id that no longer belongs to the currently visible/filtered item set (a
 * filter change, an archive/restore crossing the active/archived boundary, ...), mirroring
 * `session-sidebar-model.ts`'s own `pruneSelectionToVisible` -- reimplemented locally rather than
 * imported cross-context, since main-layout and work-board are otherwise independent domains and
 * this is six lines of generic Set logic, not a shared concept worth a cross-context dependency.
 * Returns the same `selectedIds` reference when nothing changed, so a caller's `useEffect` can
 * bail out via `setState`'s own `Object.is` short-circuit instead of looping.
 */
export function pruneBatchSelection(selectedIds: Set<string>, visibleItems: WorkItem[]): Set<string> {
  const visibleIds = new Set(visibleItems.map((item) => item.id));
  let changed = false;
  const next = new Set<string>();
  selectedIds.forEach((id) => {
    if (visibleIds.has(id)) next.add(id);
    else changed = true;
  });
  return changed ? next : selectedIds;
}
