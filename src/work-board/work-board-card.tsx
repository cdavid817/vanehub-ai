import { Archive, ArrowDown, ArrowUp, CalendarDays, FolderOpen, Pencil, RotateCcw, Trash2 } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { formatAppDateTime } from "../i18n/format";
import { normalizeDisplayPath } from "../lib/session-path";
import { cn } from "../lib/utils";
import type { WorkItem, WorkItemStage } from "../types/work-board";
import { workItemStages } from "../types/work-board";

/** The action row's buttons share the select's 32px height, so the row reads as one control strip. */
const iconButtonClass = "h-8 w-8 shrink-0 rounded-md text-muted-foreground hover:text-foreground";

const priorityAccent: Record<WorkItem["priority"], string> = {
  urgent: "bg-[hsl(var(--danger))]",
  high: "bg-[hsl(var(--danger))]",
  medium: "bg-[hsl(var(--warning))]",
  low: "bg-[hsl(var(--success))]",
  none: "bg-transparent",
};

function MetaChip({ children, icon, title }: { children: ReactNode; icon: ReactNode; title?: string }) {
  return (
    <span className="inline-flex min-w-0 max-w-full items-center gap-1 rounded border border-border/70 bg-muted/30 px-1.5 py-0.5 text-[11px] text-muted-foreground" title={title}>
      {icon}
      <span className="min-w-0 truncate">{children}</span>
    </span>
  );
}

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
  // Stored paths keep the Windows extended-length prefix; every display surface strips it.
  const projectPath = item.projectPath ? normalizeDisplayPath(item.projectPath) : null;
  return (
    <article
      // No overflow clip on the card and no line clamp on the description: in WebKitGTK a
      // `-webkit-box` child contributes zero height to the grid tracks, the card sized itself to
      // the title row alone, and the clip then hid the description and the whole action row.
      className="ucd-card relative grid gap-2 rounded-lg p-3 pl-3.5"
      data-testid={`work-item-${item.id}`}
      draggable={!item.archived}
      onDragStart={(event) => event.dataTransfer.setData("text/work-item", item.id)}
    >
      <span aria-hidden="true" className={cn("absolute inset-y-0 left-0 w-1 rounded-l-lg", priorityAccent[item.priority])} />
      <div className="flex items-start justify-between gap-2">
        <h3 className="min-w-0 flex-1 text-sm font-semibold leading-5">{item.title}</h3>
        {item.priority === "none" ? null : (
          <Badge tone={item.priority === "urgent" || item.priority === "high" ? "danger" : item.priority === "medium" ? "warning" : "muted"}>
            {t(`todoBoard.priority.${item.priority}`)}
          </Badge>
        )}
      </div>
      {item.description ? <p className="max-h-15 overflow-hidden text-xs leading-5 text-muted-foreground">{item.description}</p> : null}
      {projectPath || item.dueAt ? (
        <div className="flex min-w-0 flex-wrap items-center gap-1.5">
          {projectPath ? <MetaChip icon={<FolderOpen aria-hidden="true" className="h-3 w-3 shrink-0" />} title={projectPath}>{projectPath}</MetaChip> : null}
          {item.dueAt ? (
            <MetaChip icon={<CalendarDays aria-hidden="true" className="h-3 w-3 shrink-0" />}>
              {t("todoBoard.due", { date: formatAppDateTime(item.dueAt, i18n.language, { dateStyle: "short" }) })}
            </MetaChip>
          ) : null}
        </div>
      ) : null}
      {item.sources.length ? (
        <ul aria-label={t("todoBoard.sources")} className="grid gap-1 border-t border-border/60 pt-2">
          {item.sources.map((source) => (
            <li className="flex min-w-0 items-center gap-2 text-xs" key={`${source.sourceKind}:${source.sourceId}`}>
              <Badge tone={source.available ? "default" : "danger"}>{t(`todoBoard.source.${source.sourceKind}`)}</Badge>
              <span className="min-w-0 truncate">{source.title}</span>
              <span className="ml-auto shrink-0 text-muted-foreground">{source.available ? source.status : t("todoBoard.unavailable")}</span>
            </li>
          ))}
        </ul>
      ) : (
        <div><Badge tone="muted">{t("todoBoard.manual")}</Badge></div>
      )}
      {/* Two groups on one row: moving the card (arrows around the stage select, which stretches
          to whatever width is left) and editing it (right-aligned). In a narrow column the second
          group wraps to its own line and stays right-aligned instead of scattering. */}
      <div className="flex flex-wrap items-center gap-1.5 border-t border-border pt-2">
        {!item.archived ? <>
          <div className="flex min-w-0 flex-1 items-center gap-1">
            <Button aria-label={t("todoBoard.movePrevious")} className={iconButtonClass} disabled={stageIndex === 0} onClick={() => onMove(workItemStages[stageIndex - 1])} size="icon" type="button" variant="ghost"><ArrowUp aria-hidden="true" className="h-4 w-4" /></Button>
            <select aria-label={t("todoBoard.stage")} className="ucd-input h-8 min-w-0 flex-1 rounded-md px-2 text-xs" onChange={(event) => onMove(event.target.value as WorkItemStage)} value={item.stage}>{workItemStages.map((stage) => <option key={stage} value={stage}>{t(`todoBoard.stage.${stage}`)}</option>)}</select>
            <Button aria-label={t("todoBoard.moveNext")} className={iconButtonClass} disabled={stageIndex === workItemStages.length - 1} onClick={() => onMove(workItemStages[stageIndex + 1])} size="icon" type="button" variant="ghost"><ArrowDown aria-hidden="true" className="h-4 w-4" /></Button>
          </div>
          <div className="ml-auto flex shrink-0 items-center gap-1">
            <Button aria-label={t("todoBoard.edit")} className={iconButtonClass} onClick={onEdit} size="icon" type="button" variant="ghost"><Pencil aria-hidden="true" className="h-4 w-4" /></Button>
            <Button aria-label={t("todoBoard.archive")} className={iconButtonClass} onClick={onArchive} size="icon" type="button" variant="ghost"><Archive aria-hidden="true" className="h-4 w-4" /></Button>
          </div>
        </> : <><Button onClick={onRestore} size="sm" type="button" variant="outline"><RotateCcw aria-hidden="true" />{t("todoBoard.restore")}</Button><Button onClick={onDelete} size="sm" type="button" variant="outline"><Trash2 aria-hidden="true" />{t("todoBoard.delete")}</Button></>}
      </div>
    </article>
  );
}
