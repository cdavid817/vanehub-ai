import { Archive, CalendarDays, FolderOpen, Pencil, RotateCcw, Trash2 } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { formatAppDateTime } from "../i18n/format";
import { normalizeDisplayPath } from "../lib/session-path";
import { cn } from "../lib/utils";
import { MutationStatus } from "../ui/async/MutationStatus";
import type { MutationState } from "../ui/async/mutation-state";
import type { WorkItem, WorkItemStage } from "../types/work-board";
import { WorkItemStageMenu } from "./work-item-stage-menu";

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

export function WorkBoardCard({ item, mutation, onArchive, onDelete, onDismissError, onEdit, onMove, onRestore }: {
  item: WorkItem;
  /** This card's own in-flight move/edit/archive/restore/delete, if any -- shared across all of
   *  those actions (they all mutate the same item and would race each other), so the card
   *  disables its own controls while pending rather than the whole board. */
  mutation?: MutationState;
  onArchive: () => void;
  onDelete: () => void;
  onDismissError: () => void;
  onEdit: () => void;
  onMove: (stage: WorkItemStage) => void;
  onRestore: () => void;
}) {
  const { i18n, t } = useTranslation();
  const pending = mutation?.pending ?? false;
  // Stored paths keep the Windows extended-length prefix; every display surface strips it.
  const projectPath = item.projectPath ? normalizeDisplayPath(item.projectPath) : null;
  return (
    <article
      className="ucd-card relative grid gap-2.5 overflow-hidden rounded-lg p-3 pl-3.5"
      data-testid={`work-item-${item.id}`}
      draggable={!item.archived && !pending}
      onDragStart={(event) => event.dataTransfer.setData("text/work-item", item.id)}
    >
      <span aria-hidden="true" className={cn("absolute inset-y-0 left-0 w-1", priorityAccent[item.priority])} />
      <div className="flex items-start justify-between gap-2">
        <h3 className="min-w-0 flex-1 text-sm font-semibold leading-5">{item.title}</h3>
        {item.priority === "none" ? null : (
          <Badge tone={item.priority === "urgent" || item.priority === "high" ? "danger" : item.priority === "medium" ? "warning" : "muted"}>
            {t(`todoBoard.priority.${item.priority}`)}
          </Badge>
        )}
      </div>
      {item.description ? <p className="line-clamp-3 text-xs leading-5 text-muted-foreground">{item.description}</p> : null}
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
      <div className="flex flex-wrap items-center gap-1 border-t border-border pt-2">
        {!item.archived ? <>
          <WorkItemStageMenu disabled={pending} onMove={onMove} stage={item.stage} />
          <Button aria-label={t("todoBoard.edit")} disabled={pending} onClick={onEdit} size="icon" type="button" variant="ghost"><Pencil aria-hidden="true" /></Button>
          <Button aria-label={t("todoBoard.archive")} disabled={pending} onClick={onArchive} size="icon" type="button" variant="ghost"><Archive aria-hidden="true" /></Button>
        </> : <><Button disabled={pending} onClick={onRestore} size="sm" type="button" variant="outline"><RotateCcw aria-hidden="true" />{t("todoBoard.restore")}</Button><Button disabled={pending} onClick={onDelete} size="sm" type="button" variant="outline"><Trash2 aria-hidden="true" />{t("todoBoard.delete")}</Button></>}
      </div>
      <MutationStatus onDismiss={onDismissError} state={mutation} />
    </article>
  );
}
