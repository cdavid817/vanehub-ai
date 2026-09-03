import { useCallback, useEffect, useState } from "react";
import type { WorkItem, WorkItemStage } from "../types/work-board";
import { partitionBatchSelection, pruneBatchSelection, type WorkBoardBatchAction, type WorkBoardBatchOutcome } from "./work-board-batch";

export interface WorkBoardBatchOutcomeEntry {
  id: string;
  title: string;
  result: WorkBoardBatchOutcome;
}

/**
 * Stateful half of task 14.12 (batch mode), separate from the pure eligibility model in
 * work-board-batch.ts and the panel's own presentation in work-board-batch-panel.tsx -- mirrors
 * this file's neighbors (`use-work-board-actions.ts` vs. `work-board-card.tsx`) splitting
 * orchestration from rendering.
 *
 * `archive`/`move` are the *same* functions `useWorkBoardActions` already exposes (14.10/14.11's
 * per-card `useMutationRegistry`, keyed by `item.id`) -- this hook does not begin/succeed/fail a
 * second mutation slot per item; it only orchestrates calling those functions over the eligible
 * subset and remembers each call's own boolean result for the per-item outcome readout, since
 * `mutateCard` itself never rethrows (rollback already happened internally by the time the
 * promise settles) and the registry entry for a *successful* mutation is deleted, not kept as a
 * tombstone -- there would be nothing left to read a "which succeeded" list back out of otherwise.
 */
export function useWorkBoardBatch({ archive, move, visibleItems }: {
  archive: (item: WorkItem) => Promise<boolean>;
  move: (item: WorkItem, stage: WorkItemStage) => Promise<boolean>;
  visibleItems: WorkItem[];
}) {
  const [batchMode, setBatchMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(() => new Set());
  const [running, setRunning] = useState(false);
  const [outcome, setOutcome] = useState<WorkBoardBatchOutcomeEntry[] | null>(null);

  // A filter change, an archive/restore crossing the active/archived boundary, or a page-size
  // fixture change can all remove a selected card from view without this hook hearing about it
  // directly -- prune on every visible-set change while batch mode is active, matching
  // session-sidebar.tsx's own identical effect for the same reason.
  useEffect(() => {
    if (!batchMode) return;
    setSelectedIds((current) => pruneBatchSelection(current, visibleItems));
  }, [batchMode, visibleItems]);

  const enter = useCallback(() => { setBatchMode(true); setOutcome(null); }, []);
  const exit = useCallback(() => { setBatchMode(false); setSelectedIds(new Set()); setOutcome(null); }, []);

  const toggle = useCallback((id: string) => setSelectedIds((current) => {
    const next = new Set(current);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    return next;
  }), []);

  const selectAllVisible = useCallback(() => setSelectedIds(new Set(visibleItems.map((item) => item.id))), [visibleItems]);
  const clearSelection = useCallback(() => setSelectedIds(new Set()), []);

  /** Ineligible items are recorded as "skipped" immediately (never attempted); eligible items run
   *  concurrently via the same per-card mutation path any individual card action already uses, so
   *  one card's own failure/rollback cannot block or corrupt another's -- exactly the concurrent,
   *  independent-per-id case use-work-board-actions.ts's own doc comment says this registry exists
   *  for. */
  const run = useCallback(async (action: WorkBoardBatchAction) => {
    const { eligible, ineligible } = partitionBatchSelection(visibleItems, selectedIds, action);
    const skipped: WorkBoardBatchOutcomeEntry[] = ineligible.map((item) => ({ id: item.id, title: item.title, result: "skipped" }));
    setRunning(true);
    setOutcome(skipped);
    const settled = await Promise.all(eligible.map(async (item) => {
      const ok = action.kind === "archive" ? await archive(item) : await move(item, action.stage);
      return { id: item.id, title: item.title, result: (ok ? "success" : "error") as WorkBoardBatchOutcome };
    }));
    setOutcome([...skipped, ...settled]);
    setRunning(false);
  }, [archive, move, selectedIds, visibleItems]);

  return { batchMode, clearSelection, enter, exit, outcome, run, running, selectAllVisible, selectedIds, toggle };
}
