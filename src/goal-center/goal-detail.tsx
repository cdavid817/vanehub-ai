import { AlertTriangle, Check, Play, Plus, RotateCcw, Trash2, X } from "lucide-react";
import { type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import type { Goal, GoalLinkTarget } from "../contracts/goal";
import { linkableGoalTargets } from "../contracts/goal";
import {
  blockingReason, canAccept, groupLinks, progressLabel, statusTone, unresolvableLinks,
} from "./goal-presentation";

const fieldClass = "ucd-input rounded-md px-3 py-2 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring";

export interface GoalDetailProps {
  goal: Goal;
  busy: boolean;
  onActivate: () => void;
  onAccept: () => void;
  onReopen: () => void;
  onAbandon: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onLink: (targetKind: GoalLinkTarget, targetId: string) => void;
  onUnlink: (targetKind: GoalLinkTarget, targetId: string) => void;
}

export function GoalDetail(props: GoalDetailProps) {
  const { busy, goal, onAbandon, onAccept, onActivate, onDelete, onEdit, onLink, onReopen, onUnlink } = props;
  const { t } = useTranslation();
  const reason = blockingReason(goal);
  const stranded = unresolvableLinks(goal);

  const submitLink = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    const targetId = String(data.get("targetId") ?? "").trim();
    if (!targetId) return;
    onLink(String(data.get("targetKind") ?? "loop") as GoalLinkTarget, targetId);
    event.currentTarget.reset();
  };

  return <section aria-labelledby="goal-detail-title" className="grid content-start gap-4 overflow-y-auto p-4">
    <header className="grid gap-2">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <h2 className="truncate text-lg font-semibold" id="goal-detail-title">{goal.title}</h2>
          {goal.projectPath ? <p className="truncate text-xs text-muted-foreground">{goal.projectPath}</p> : null}
        </div>
        <span className={`shrink-0 rounded px-2 py-1 text-xs font-medium ${statusTone(goal.derivedStatus)}`}>{t(`goals.status.${goal.derivedStatus}`)}</span>
      </div>
      {goal.description ? <p className="whitespace-pre-wrap text-sm text-muted-foreground">{goal.description}</p> : null}
    </header>

    <div className="flex flex-wrap gap-2">
      {goal.status === "draft" || goal.status === "abandoned"
        ? <Button disabled={busy} onClick={onActivate} size="sm" type="button"><Play aria-hidden="true" />{t("goals.actions.activate")}</Button>
        : null}
      {goal.status === "active"
        ? <Button disabled={busy || !canAccept(goal)} onClick={onAccept} size="sm" title={canAccept(goal) ? undefined : t(`goals.blocked.${reason}`)} type="button"><Check aria-hidden="true" />{t("goals.actions.accept")}</Button>
        : null}
      {goal.status === "achieved"
        ? <Button disabled={busy} onClick={onReopen} size="sm" type="button" variant="outline"><RotateCcw aria-hidden="true" />{t("goals.actions.reopen")}</Button>
        : null}
      {goal.status === "abandoned"
        ? null
        : <Button disabled={busy} onClick={onAbandon} size="sm" type="button" variant="outline">{t("goals.actions.abandon")}</Button>}
      <Button disabled={busy} onClick={onEdit} size="sm" type="button" variant="outline">{t("goals.actions.edit")}</Button>
      <Button disabled={busy} onClick={onDelete} size="sm" type="button" variant="outline"><Trash2 aria-hidden="true" />{t("goals.actions.delete")}</Button>
    </div>

    <div className="grid gap-2 rounded-md border border-border bg-muted/10 p-3">
      <div className="flex items-baseline justify-between gap-2">
        <h3 className="text-sm font-semibold">{t("goals.progress.title")}</h3>
        <span className="text-sm tabular-nums">{progressLabel(goal)}</span>
      </div>
      {/* The reason is spelled out rather than left to a bar that simply stops moving. */}
      <p className="text-xs text-muted-foreground">{t(`goals.blocked.${reason}`)}</p>
      {stranded.length
        ? <p className="flex items-start gap-2 text-xs text-amber-600 dark:text-amber-400" role="status">
            <AlertTriangle aria-hidden="true" className="mt-0.5 h-3.5 w-3.5 shrink-0" />
            {t("goals.progress.unresolvable", { count: stranded.length })}
          </p>
        : null}
      {goal.acceptanceNotes
        ? <p className="whitespace-pre-wrap border-t border-border pt-2 text-xs text-muted-foreground">{goal.acceptanceNotes}</p>
        : null}
    </div>

    <div className="grid gap-3">
      <h3 className="text-sm font-semibold">{t("goals.links.title")}</h3>
      {goal.links.length === 0 ? <p className="text-xs text-muted-foreground">{t("goals.links.empty")}</p> : null}
      {groupLinks(goal.links).map((group) => <div className="grid gap-1" key={group.kind}>
        <h4 className="text-xs font-medium text-muted-foreground">{t(`goals.target.${group.kind}`)}</h4>
        {group.links.map((link) => <div className="flex items-center justify-between gap-2 rounded border border-border px-2 py-1" key={`${link.targetKind}:${link.targetId}`}>
          <span className="truncate text-xs">{link.targetId}</span>
          <span className="flex shrink-0 items-center gap-2">
            <span className={`text-xs ${link.progress === "unresolvable" ? "text-amber-600 dark:text-amber-400" : "text-muted-foreground"}`}>
              {t(group.kind === "session" ? "goals.linkProgress.notCounted" : `goals.linkProgress.${link.progress}`)}
            </span>
            <Button aria-label={t("goals.actions.unlink")} disabled={busy} onClick={() => onUnlink(link.targetKind, link.targetId)} size="icon" type="button" variant="ghost"><X aria-hidden="true" /></Button>
          </span>
        </div>)}
      </div>)}

      <form className="flex flex-wrap gap-2" onSubmit={submitLink}>
        <select aria-label={t("goals.fields.targetKind")} className={fieldClass} defaultValue="loop" name="targetKind">
          {linkableGoalTargets.map((kind) => <option key={kind} value={kind}>{t(`goals.target.${kind}`)}</option>)}
        </select>
        <input aria-label={t("goals.fields.targetId")} className={`${fieldClass} min-w-0 flex-1`} name="targetId" placeholder={t("goals.fields.targetId")} />
        <Button disabled={busy} size="sm" type="submit"><Plus aria-hidden="true" />{t("goals.actions.link")}</Button>
      </form>
    </div>
  </section>;
}
