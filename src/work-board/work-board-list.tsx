import { useTranslation } from "react-i18next";
import type { MutationState } from "../ui/async/mutation-state";
import type { WorkItem, WorkItemStage } from "../types/work-board";
import { WorkBoardCard } from "./work-board-card";
import { groupWorkItemsByStage, type WorkBoardGrouping } from "./work-board-query";

export interface WorkBoardListProps {
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
}

/**
 * The "list" presentation (task 14.3): a flat or stage-grouped alternative to WorkBoardColumn's
 * per-stage Kanban columns, deliberately not reusing that component. List presentation has no
 * drag-and-drop drop target and no always-rendered-even-when-empty column shell, both essential,
 * already-tested parts of WorkBoardColumn's own contract in Board presentation. Movement here
 * goes exclusively through each card's own WorkItemStageMenu -- unchanged from Board presentation,
 * just without a drop target as the alternative path.
 */
export function WorkBoardList({
  filterSummary, filtersActive, grouping, items, mutations,
  onArchive, onDelete, onDismissError, onEdit, onMove, onRestore,
}: WorkBoardListProps) {
  const { t } = useTranslation();
  const card = (item: WorkItem) => (
    <WorkBoardCard
      item={item}
      key={item.id}
      mutation={mutations.get(item.id)}
      onArchive={() => onArchive(item)}
      onDelete={() => onDelete(item)}
      onDismissError={() => onDismissError(item)}
      onEdit={() => onEdit(item)}
      onMove={(target) => onMove(item, target)}
      onRestore={() => onRestore(item)}
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
    return <div className="grid min-h-0 flex-1 content-start gap-2 overflow-y-auto p-3">{items.map(card)}</div>;
  }

  return (
    <div className="grid min-h-0 flex-1 content-start gap-4 overflow-y-auto p-3">
      {groupWorkItemsByStage(items).map((group) => (
        <section aria-labelledby={`work-board-list-${group.stage}`} className="grid gap-2" key={group.stage}>
          <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground" id={`work-board-list-${group.stage}`}>
            {t(`todoBoard.stage.${group.stage}`)} · {group.items.length}
          </h2>
          {group.items.map(card)}
        </section>
      ))}
    </div>
  );
}
