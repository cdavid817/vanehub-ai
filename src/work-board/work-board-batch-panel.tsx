import { useState } from "react";
import { Archive, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import type { WorkItem, WorkItemStage } from "../types/work-board";
import { workItemStages } from "../types/work-board";
import type { WorkBoardBatchOutcomeEntry } from "./use-work-board-batch";
import { partitionBatchSelection } from "./work-board-batch";
import { fieldClass } from "./work-board-form";

export interface WorkBoardBatchPanelProps {
  /** The currently visible (post-filter) item set -- eligibility is only meaningful against
   *  items the reader can actually see selecting. */
  items: WorkItem[];
  onArchive: () => void;
  onClearSelection: () => void;
  onExit: () => void;
  onMove: (stage: WorkItemStage) => void;
  onSelectAllVisible: () => void;
  outcome: WorkBoardBatchOutcomeEntry[] | null;
  running: boolean;
  selectedIds: Set<string>;
}

const outcomeTone: Record<WorkBoardBatchOutcomeEntry["result"], string> = {
  success: "text-[hsl(var(--success))]",
  error: "text-destructive",
  skipped: "text-muted-foreground",
};

/**
 * 14.12's own bounded action region: selected count, eligibility preview for both batch actions
 * (recomputed live as the Move-to target changes, via the same pure `partitionBatchSelection`
 * `use-work-board-batch.ts` uses to actually run the action -- one definition of eligibility, not
 * a second guess for the preview), and a per-item outcome list once a run completes. Kept as a
 * dumb, prop-driven component (no hook call of its own) so it is testable the same way every
 * other component in this directory already is -- render with plain props, assert on output.
 */
export function WorkBoardBatchPanel({ items, onArchive, onClearSelection, onExit, onMove, onSelectAllVisible, outcome, running, selectedIds }: WorkBoardBatchPanelProps) {
  const { t } = useTranslation();
  const [moveTarget, setMoveTarget] = useState<WorkItemStage>(workItemStages[0]);

  const archivePartition = partitionBatchSelection(items, selectedIds, { kind: "archive" });
  const movePartition = partitionBatchSelection(items, selectedIds, { kind: "move", stage: moveTarget });

  return (
    <div aria-label={t("todoBoard.batch.trigger")} className="ucd-muted-panel mx-3 mb-3 grid gap-2 rounded-md border border-border p-3 md:mx-4" role="region">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <p className="text-sm font-medium" role="status">{t("todoBoard.batch.selectedCount", { count: selectedIds.size })}</p>
        <div className="flex items-center gap-1.5">
          <Button className="h-7 px-2 text-xs" onClick={onSelectAllVisible} size="sm" type="button" variant="outline">{t("todoBoard.batch.selectAllVisible")}</Button>
          <Button className="h-7 px-2 text-xs" disabled={selectedIds.size === 0} onClick={onClearSelection} size="sm" type="button" variant="outline">{t("todoBoard.batch.clearSelection")}</Button>
          <Button className="h-7 px-2 text-xs" onClick={onExit} size="sm" type="button" variant="outline"><X aria-hidden="true" className="h-3.5 w-3.5" />{t("todoBoard.batch.exit")}</Button>
        </div>
      </div>

      <div className="flex flex-wrap items-center gap-2 border-t border-border pt-2">
        <Button className="h-8 px-2.5 text-xs" disabled={running || archivePartition.eligible.length === 0} onClick={onArchive} size="sm" type="button" variant="outline">
          <Archive aria-hidden="true" className="h-3.5 w-3.5" />
          {t("todoBoard.batch.archiveAction")}
        </Button>
        <span className="text-xs text-muted-foreground">{t("todoBoard.batch.eligibleCount", { eligible: archivePartition.eligible.length, total: selectedIds.size })}</span>

        <span aria-hidden="true" className="mx-1 h-4 w-px bg-border" />

        <select aria-label={t("todoBoard.batch.moveTo")} className={fieldClass} onChange={(event) => setMoveTarget(event.target.value as WorkItemStage)} value={moveTarget}>
          {workItemStages.map((stage) => <option key={stage} value={stage}>{t(`todoBoard.stage.${stage}`)}</option>)}
        </select>
        <Button className="h-8 px-2.5 text-xs" disabled={running || movePartition.eligible.length === 0} onClick={() => onMove(moveTarget)} size="sm" type="button">
          {t("todoBoard.batch.moveAction", { stage: t(`todoBoard.stage.${moveTarget}`) })}
        </Button>
        <span className="text-xs text-muted-foreground">{t("todoBoard.batch.eligibleCount", { eligible: movePartition.eligible.length, total: selectedIds.size })}</span>
      </div>

      {outcome && outcome.length > 0 ? (
        <ul aria-label={t("todoBoard.batch.outcomeTitle")} className="grid gap-1 border-t border-border pt-2 text-xs">
          {outcome.map((entry) => (
            <li className="flex items-center gap-2" key={entry.id}>
              <span className={outcomeTone[entry.result]}>{t(`todoBoard.batch.outcome.${entry.result}`)}</span>
              <span className="min-w-0 flex-1 truncate">{entry.title}</span>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
