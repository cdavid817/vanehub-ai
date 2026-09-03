import { type DragEvent } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../components/ui/badge";
import type { MutationState } from "../ui/async/mutation-state";
import type { WorkItem, WorkItemStage } from "../types/work-board";
import { isOverWipLimit } from "./work-board-wip-limits";
import { WorkBoardCard } from "./work-board-card";
import { WorkBoardItemList } from "./work-board-item-list";

// Shared by both branches below so an empty column's message sits inside the exact same
// flex-1/min-h-0/overflow-y-auto grid region real cards would occupy, rather than being pushed
// below a now-empty flex-1 box -- `min-h-0` is load-bearing, not decorative: without it this grid
// child cannot shrink below its content size inside the section's own flex column, and
// `WorkBoardItemList`'s virtualized branch would never get a real bounded viewport to virtualize
// against (14.15).
const ITEM_REGION_CLASS = "grid min-h-0 flex-1 content-start gap-2 overflow-y-auto p-2";

export function WorkBoardColumn({ batchMode, filterSummary, filtersActive, items, mutations, onArchive, onDelete, onDismissError, onDrop, onEdit, onMove, onRestore, onToggleSelected, selectedIds, stage, wipLimit }: {
  /** 14.12: while true, every card in this column shows a checkbox instead of its normal action
   *  row -- see WorkBoardCard's own doc comment. */
  batchMode?: boolean;
  /** Human-readable "why nothing matches" text (e.g. "Priority: High, Due: Overdue"), built once
   *  from the active query and shared across every column -- undefined when no filter is active. */
  filterSummary?: string;
  filtersActive: boolean;
  items: WorkItem[];
  /** Every card's own mutation state, keyed by work item id -- see use-work-board-actions.ts. */
  mutations: ReadonlyMap<string, MutationState>;
  onArchive: (item: WorkItem) => void;
  onDelete: (item: WorkItem) => void;
  onDismissError: (item: WorkItem) => void;
  onDrop: (event: DragEvent<HTMLElement>, stage: WorkItemStage) => void;
  onEdit: (item: WorkItem) => void;
  onMove: (item: WorkItem, stage: WorkItemStage) => void;
  onRestore: (item: WorkItem) => void;
  onToggleSelected?: (item: WorkItem) => void;
  selectedIds?: ReadonlySet<string>;
  stage: WorkItemStage;
  /** 14.14: optional, presentation-only soft limit for this stage -- undefined means "no limit
   *  configured," never "zero capacity." */
  wipLimit?: number;
}) {
  const { t } = useTranslation();
  const overWip = isOverWipLimit(items.length, wipLimit);

  return (
    <section
      className="flex min-w-[17rem] flex-1 flex-col rounded-lg border border-border bg-muted/10"
      onDragOver={(event) => event.preventDefault()}
      onDrop={(event) => onDrop(event, stage)}
    >
      <header className="flex items-center justify-between gap-2 rounded-t-lg border-b border-border bg-[hsl(var(--panel-muted))] px-3 py-2">
        <h2 className="min-w-0 truncate text-sm font-semibold">{t(`todoBoard.stage.${stage}`)}</h2>
        <span className="flex shrink-0 items-center gap-1.5">
          {overWip ? (
            <Badge title={t("todoBoard.wip.badgeTitle")} tone="warning">{t("todoBoard.wip.badge", { count: items.length, limit: wipLimit })}</Badge>
          ) : null}
          <span className="rounded-full bg-muted px-2 py-0.5 text-[11px] tabular-nums text-muted-foreground">{items.length}</span>
        </span>
      </header>
      {items.length === 0 ? (
        <div className={ITEM_REGION_CLASS}>
          <p className="rounded-md border border-dashed border-border/70 p-4 text-center text-xs text-muted-foreground">
            {filtersActive
              ? (filterSummary ? t("todoBoard.emptyFilteredReason", { reason: filterSummary }) : t("todoBoard.emptyFiltered"))
              : t("todoBoard.empty")}
          </p>
        </div>
      ) : (
        <WorkBoardItemList
          ariaLabel={t(`todoBoard.stage.${stage}`)}
          className={ITEM_REGION_CLASS}
          items={items}
          renderItem={(item) => (
            <WorkBoardCard
              batchMode={batchMode}
              item={item}
              key={item.id}
              mutation={mutations.get(item.id)}
              onArchive={() => onArchive(item)}
              onDelete={() => onDelete(item)}
              onDismissError={() => onDismissError(item)}
              onEdit={() => onEdit(item)}
              onMove={(target) => onMove(item, target)}
              onRestore={() => onRestore(item)}
              onToggleSelected={() => onToggleSelected?.(item)}
              selected={selectedIds?.has(item.id)}
            />
          )}
        />
      )}
    </section>
  );
}
