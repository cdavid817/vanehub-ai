import { Archive, ArrowDown, ArrowUp, Pencil, RotateCcw, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { formatAppDateTime } from "../i18n/format";
import type { WorkItem, WorkItemStage } from "../types/work-board";
import { workItemStages } from "../types/work-board";

export function WorkBoardCard({ item, onArchive, onDelete, onEdit, onMove, onRestore }: {
  item: WorkItem;
  onArchive: () => void;
  onDelete: () => void;
  onEdit: () => void;
  onMove: (stage: WorkItemStage) => void;
  onRestore: () => void;
}) {
  const { i18n, t } = useTranslation();
  const stageIndex = workItemStages.indexOf(item.stage);
  return (
    <article className="ucd-card grid gap-3 rounded-lg p-3" data-testid={`work-item-${item.id}`} draggable={!item.archived} onDragStart={(event) => event.dataTransfer.setData("text/work-item", item.id)}>
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0"><h3 className="truncate text-sm font-semibold">{item.title}</h3>{item.description ? <p className="mt-1 line-clamp-3 text-xs text-muted-foreground">{item.description}</p> : null}</div>
        <Badge tone={item.priority === "urgent" || item.priority === "high" ? "danger" : item.priority === "medium" ? "warning" : "muted"}>{t(`todoBoard.priority.${item.priority}`)}</Badge>
      </div>
      {item.projectPath ? <p className="truncate text-xs text-muted-foreground" title={item.projectPath}>{item.projectPath}</p> : null}
      {item.sources.length ? <ul className="grid gap-1" aria-label={t("todoBoard.sources")}>
        {item.sources.map((source) => <li className="flex min-w-0 items-center gap-2 text-xs" key={`${source.sourceKind}:${source.sourceId}`}><Badge tone={source.available ? "default" : "danger"}>{t(`todoBoard.source.${source.sourceKind}`)}</Badge><span className="truncate">{source.title}</span><span className="ml-auto shrink-0 text-muted-foreground">{source.available ? source.status : t("todoBoard.unavailable")}</span></li>)}
      </ul> : <Badge tone="muted">{t("todoBoard.manual")}</Badge>}
      {item.dueAt ? <p className="text-xs text-muted-foreground">{t("todoBoard.due", { date: formatAppDateTime(item.dueAt, i18n.language, { dateStyle: "short" }) })}</p> : null}
      <div className="flex flex-wrap items-center gap-1 border-t border-border pt-2">
        {!item.archived ? <>
          <Button aria-label={t("todoBoard.movePrevious")} disabled={stageIndex === 0} onClick={() => onMove(workItemStages[stageIndex - 1])} size="icon" type="button" variant="ghost"><ArrowUp aria-hidden="true" /></Button>
          <select aria-label={t("todoBoard.stage")} className="ucd-input min-w-0 flex-1 rounded px-2 py-1 text-xs" onChange={(event) => onMove(event.target.value as WorkItemStage)} value={item.stage}>{workItemStages.map((stage) => <option key={stage} value={stage}>{t(`todoBoard.stage.${stage}`)}</option>)}</select>
          <Button aria-label={t("todoBoard.moveNext")} disabled={stageIndex === workItemStages.length - 1} onClick={() => onMove(workItemStages[stageIndex + 1])} size="icon" type="button" variant="ghost"><ArrowDown aria-hidden="true" /></Button>
          <Button aria-label={t("todoBoard.edit")} onClick={onEdit} size="icon" type="button" variant="ghost"><Pencil aria-hidden="true" /></Button>
          <Button aria-label={t("todoBoard.archive")} onClick={onArchive} size="icon" type="button" variant="ghost"><Archive aria-hidden="true" /></Button>
        </> : <><Button onClick={onRestore} size="sm" type="button" variant="outline"><RotateCcw aria-hidden="true" />{t("todoBoard.restore")}</Button><Button onClick={onDelete} size="sm" type="button" variant="outline"><Trash2 aria-hidden="true" />{t("todoBoard.delete")}</Button></>}
      </div>
    </article>
  );
}
