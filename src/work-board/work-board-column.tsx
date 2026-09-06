import { type DragEvent } from "react";
import { useTranslation } from "react-i18next";
import type { WorkItem, WorkItemStage } from "../types/work-board";
import { WorkBoardCard } from "./work-board-card";

export function WorkBoardColumn({ filtersActive, items, onArchive, onDelete, onDrop, onEdit, onMove, onRestore, stage }: {
  filtersActive: boolean;
  items: WorkItem[];
  onArchive: (item: WorkItem) => void;
  onDelete: (item: WorkItem) => void;
  onDrop: (event: DragEvent<HTMLElement>, stage: WorkItemStage) => void;
  onEdit: (item: WorkItem) => void;
  onMove: (item: WorkItem, stage: WorkItemStage) => void;
  onRestore: (item: WorkItem) => void;
  stage: WorkItemStage;
}) {
  const { t } = useTranslation();

  return (
    <section
      // min-h-0 here and on the list below are what let a full column scroll: without them the
      // column grew to its cards and the board's overflow clip cut the bottom ones off unseen.
      // 13rem is the narrowest a card's action row fits in; at 17rem five columns needed 1.4k px
      // and every window under that hid the last column behind a scrollbar nobody saw.
      className="flex min-h-0 min-w-[13rem] flex-1 flex-col rounded-lg border border-border bg-muted/10"
      onDragOver={(event) => event.preventDefault()}
      onDrop={(event) => onDrop(event, stage)}
    >
      <header className="flex items-center justify-between gap-2 rounded-t-lg border-b border-border bg-[hsl(var(--panel-muted))] px-3 py-2">
        <h2 className="min-w-0 truncate text-sm font-semibold">{t(`todoBoard.stage.${stage}`)}</h2>
        <span className="shrink-0 rounded-full bg-muted px-2 py-0.5 text-[11px] tabular-nums text-muted-foreground">{items.length}</span>
      </header>
      <div className="grid min-h-0 flex-1 content-start gap-2 overflow-y-auto p-2">
        {items.map((item) => (
          <WorkBoardCard
            item={item}
            key={item.id}
            onArchive={() => onArchive(item)}
            onDelete={() => onDelete(item)}
            onEdit={() => onEdit(item)}
            onMove={(target) => onMove(item, target)}
            onRestore={() => onRestore(item)}
          />
        ))}
        {items.length === 0 ? (
          <p className="rounded-md border border-dashed border-border/70 p-4 text-center text-xs text-muted-foreground">
            {filtersActive ? t("todoBoard.emptyFiltered") : t("todoBoard.empty")}
          </p>
        ) : null}
      </div>
    </section>
  );
}
