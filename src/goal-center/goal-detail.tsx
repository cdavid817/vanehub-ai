import { AlertTriangle, Ban, Check, Pencil, Play, RotateCcw, Trash2 } from "lucide-react";
import { type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { normalizeDisplayPath } from "../lib/session-path";
import { ActionMenu, type ActionMenuItem } from "../ui/actions/ActionMenu";
import { MutationStatus } from "../ui/async/MutationStatus";
import type { MutationState } from "../ui/async/mutation-state";
import type { Goal, GoalLinkTarget } from "../contracts/goal";
import { ExecutionTargetPicker } from "./execution-target-picker";
import { GoalRelationshipSections } from "./goal-relationship-sections";
import {
  blockingReason, canAccept, progressLabel, statusTone, unresolvableLinks,
} from "./goal-presentation";

export interface GoalDetailProps {
  goal: Goal;
  /** This goal's own in-flight activate/accept/reopen/abandon/edit/delete/link/unlink, if any --
   *  shared across all of those actions (they all mutate the same goal and would race each
   *  other), so the detail pane disables only its own goal's controls while pending rather than
   *  the whole page. Matches work-board-card.tsx's own `busy: boolean` -> `mutation?: MutationState`
   *  change. */
  mutation?: MutationState;
  onActivate: () => void;
  onAccept: () => void;
  onReopen: () => void;
  onAbandon: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onDismissError: () => void;
  onLink: (targetKind: GoalLinkTarget, targetId: string) => void;
  onUnlink: (targetKind: GoalLinkTarget, targetId: string) => void;
}

export function GoalDetail(props: GoalDetailProps) {
  const { goal, mutation, onAbandon, onAccept, onActivate, onDelete, onDismissError, onEdit, onLink, onReopen, onUnlink } = props;
  const { t } = useTranslation();
  const reason = blockingReason(goal);
  const stranded = unresolvableLinks(goal);
  const pending = mutation?.pending ?? false;

  /**
   * 15.3: exactly one of Activate/Accept/Reopen ever applies to a given `status` -- each is
   * guarded below by the same disjoint `status` checks the old always-visible row used, so there
   * is never a second candidate to weigh, only whichever one already applies. Accept keeps its
   * pre-existing disabled/title behavior verbatim (visible but disabled with a reason until
   * `canAccept`) -- this only changes how the applicable action is presented, not whether it is
   * available.
   */
  let primaryAction: ReactNode = null;
  if (goal.status === "draft" || goal.status === "abandoned") {
    primaryAction = <Button disabled={pending} onClick={onActivate} size="sm" type="button"><Play aria-hidden="true" />{t("goals.actions.activate")}</Button>;
  } else if (goal.status === "active") {
    primaryAction = (
      <Button disabled={pending || !canAccept(goal)} onClick={onAccept} size="sm" title={canAccept(goal) ? undefined : t(`goals.blocked.${reason}`)} type="button">
        <Check aria-hidden="true" />{t("goals.actions.accept")}
      </Button>
    );
  } else if (goal.status === "achieved") {
    primaryAction = <Button disabled={pending} onClick={onReopen} size="sm" type="button" variant="outline"><RotateCcw aria-hidden="true" />{t("goals.actions.reopen")}</Button>;
  }

  // Everything else permitted for this status -- Abandon (unless already abandoned), Edit,
  // Delete -- same visibility/disabled conditions as the row this replaces, just grouped into
  // "More" instead of their own always-visible buttons.
  const moreItems: ActionMenuItem[] = [];
  if (goal.status !== "abandoned") {
    moreItems.push({ disabled: pending, icon: Ban, id: "abandon", label: t("goals.actions.abandon"), onSelect: onAbandon });
  }
  moreItems.push(
    { disabled: pending, icon: Pencil, id: "edit", label: t("goals.actions.edit"), onSelect: onEdit },
    { disabled: pending, icon: Trash2, id: "delete", label: t("goals.actions.delete"), onSelect: onDelete, tone: "destructive" },
  );

  return <section aria-labelledby="goal-detail-title" className="grid content-start gap-4 overflow-y-auto p-4">
    <header className="grid gap-2 rounded-md border border-border bg-muted/10 p-3">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <h2 className="truncate text-lg font-semibold" id="goal-detail-title">{goal.title}</h2>
          {goal.projectPath ? <p className="truncate text-xs text-muted-foreground" title={normalizeDisplayPath(goal.projectPath)}>{normalizeDisplayPath(goal.projectPath)}</p> : null}
        </div>
        <span className={`shrink-0 rounded px-2 py-1 text-xs font-medium ${statusTone(goal.derivedStatus)}`}>{t(`goals.status.${goal.derivedStatus}`)}</span>
      </div>
      {goal.description ? <p className="whitespace-pre-wrap text-sm text-muted-foreground">{goal.description}</p> : null}
    </header>

    <div aria-label={t("goals.actionsLabel")} className="flex flex-wrap items-center gap-2 rounded-md border border-border bg-muted/20 p-2" role="group">
      {primaryAction}
      <ActionMenu items={moreItems} triggerLabel={t("workbenchUi.pageHeader.moreActions")} />
    </div>
    <MutationStatus onDismiss={onDismissError} state={mutation} />

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
      <GoalRelationshipSections links={goal.links} onUnlink={onUnlink} pending={pending} />
      <ExecutionTargetPicker onLink={onLink} pending={pending} />
    </div>
  </section>;
}
