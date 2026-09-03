import { useTranslation } from "react-i18next";
import { Badge } from "../components/ui/badge";
import type { MutationState } from "../ui/async/mutation-state";
import type { WorkItem, WorkItemStage } from "../types/work-board";
import { WorkBoardCard } from "./work-board-card";
import { WorkBoardItemList } from "./work-board-item-list";
import { groupWorkItemsByStage, type WorkBoardGrouping } from "./work-board-query";
import { isOverWipLimit, type WorkBoardWipLimits } from "./work-board-wip-limits";

// `min-h-0` matters here the same way it does for WorkBoardColumn's own item region (14.15): it is
// what lets this grid child shrink below its content size inside the section's flex column, so
// WorkBoardItemList's virtualized branch gets a real bounded viewport rather than one that just
// grows to fit every row.
const UNGROUPED_REGION_CLASS = "grid min-h-0 flex-1 content-start gap-2 overflow-y-auto p-3";
const GROUP_SECTION_CLASS = "grid gap-2";
// A stage group is normally left to its own natural height (many small stacked sections read as
// one continuous page, the existing pre-14.15 behavior) -- `max-h`+`overflow-y-auto` only starts
// doing anything once a single group's content actually exceeds it, which is also exactly the
// case where WorkBoardItemList's virtualized branch needs a real bounded viewport to virtualize
// against. A small group renders identically to before: nothing here is visually active for it.
const GROUP_ITEM_REGION_CLASS = "grid max-h-[28rem] gap-2 overflow-y-auto";

export interface WorkBoardListProps {
  /** 14.12: forwarded straight through to every rendered `WorkBoardCard` -- see that component's
   *  own doc comment. */
  batchMode?: boolean;
  filterSummary?: string;
  filtersActive: boolean;
  grouping: WorkBoardGrouping;
  items: WorkItem[];
  mutations: ReadonlyMap<string, MutationState>;
  onArchive: (item: WorkItem) => void;
  onDelete: (item: WorkItem) => void;
  onDismissError: (item: WorkItem) => void;
  onEdit: (item: WorkItem) => void;
  onMove: (item: WorkItem, stage: WorkItemStage) => void;
  onRestore: (item: WorkItem) => void;
  onToggleSelected?: (item: WorkItem) => void;
  selectedIds?: ReadonlySet<string>;
  /** 14.14: optional, presentation-only soft limits, read per stage-group header. */
  wipLimits?: WorkBoardWipLimits;
}

/**
 * The "list" presentation (task 14.3): a flat or stage-grouped alternative to WorkBoardColumn's
 * per-stage Kanban columns, deliberately not reusing that component. List presentation has no
 * drag-and-drop drop target and no always-rendered-even-when-empty column shell, both essential,
 * already-tested parts of WorkBoardColumn's own contract in Board presentation. Movement here
 * goes exclusively through each card's own WorkItemStageMenu -- unchanged from Board presentation,
 * just without a drop target as the alternative path.
 *
 * 14.13: also the compact Stage List's own renderer -- `work-board.tsx` calls this with
 * `grouping="stage"` forced at narrow viewports, since stage-grouped sections stacked vertically
 * with no drag target *is* "a compact grouped Stage List that does not require horizontal
 * dragging," and this component already builds exactly that for the wide List presentation. No
 * second grouped-list component was built to duplicate it.
 */
export function WorkBoardList({
  batchMode, filterSummary, filtersActive, grouping, items, mutations,
  onArchive, onDelete, onDismissError, onEdit, onMove, onRestore, onToggleSelected, selectedIds, wipLimits,
}: WorkBoardListProps) {
  const { t } = useTranslation();
  const card = (item: WorkItem) => (
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
  );

  if (items.length === 0) {
    return (
      <p className="m-3 rounded-md border border-dashed border-border/70 p-4 text-center text-xs text-muted-foreground">
        {filtersActive
          ? (filterSummary ? t("todoBoard.emptyFilteredReason", { reason: filterSummary }) : t("todoBoard.emptyFiltered"))
          : t("todoBoard.empty")}
      </p>
    );
  }

  if (grouping === "none") {
    return <WorkBoardItemList ariaLabel={t("todoBoard.title")} className={UNGROUPED_REGION_CLASS} items={items} renderItem={card} />;
  }

  return (
    <div className="grid min-h-0 flex-1 content-start gap-4 overflow-y-auto p-3">
      {groupWorkItemsByStage(items).map((group) => {
        const overWip = isOverWipLimit(group.items.length, wipLimits?.[group.stage]);
        return (
          <section aria-labelledby={`work-board-list-${group.stage}`} className={GROUP_SECTION_CLASS} key={group.stage}>
            <h2 className="flex items-center gap-1.5 text-xs font-semibold uppercase tracking-wide text-muted-foreground" id={`work-board-list-${group.stage}`}>
              <span>{t(`todoBoard.stage.${group.stage}`)} · {group.items.length}</span>
              {overWip ? (
                <Badge title={t("todoBoard.wip.badgeTitle")} tone="warning">{t("todoBoard.wip.badge", { count: group.items.length, limit: wipLimits?.[group.stage] })}</Badge>
              ) : null}
            </h2>
            <WorkBoardItemList ariaLabel={t(`todoBoard.stage.${group.stage}`)} className={GROUP_ITEM_REGION_CLASS} items={group.items} renderItem={card} />
          </section>
        );
      })}
    </div>
  );
}
