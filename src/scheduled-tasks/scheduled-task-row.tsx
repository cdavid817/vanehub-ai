import { Copy, Pencil, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { formatScheduledTaskFrequency } from "../lib/scheduled-task-recurrence";
import { ActionMenu, type ActionMenuItem } from "../ui/actions/ActionMenu";
import { MutationStatus } from "../ui/async/MutationStatus";
import type { MutationState } from "../ui/async/mutation-state";
import type { AgentRegistryEntry, ScheduledTask } from "../types/agent";
import { formatDateTime, frequencySummaryParams, statusClass } from "./scheduled-task-presentation";

export interface ScheduledTaskRowProps {
  task: ScheduledTask;
  agent?: AgentRegistryEntry;
  selected: boolean;
  weekdayNames: string[];
  language: string;
  /** This row's own in-flight Enable/Disable, Delete, Run now, or Edit save, if any -- see
   *  `use-scheduled-tasks-actions.ts`'s own doc comment for why all four share one slot per task
   *  id rather than each other's own. */
  mutation?: MutationState;
  onSelect: (taskId: string) => void;
  onSetEnabled: (task: ScheduledTask, enabled: boolean) => void;
  onEdit: (task: ScheduledTask) => void;
  onDuplicate: (task: ScheduledTask) => void;
  onDelete: (task: ScheduledTask) => void;
  onDismissError: (taskId: string) => void;
}

/**
 * 19.16: Delete moves from this row's own bespoke inline Trash2-button + Cancel/Confirm-button
 * pair into `ActionMenu`'s built-in `confirmation`, alongside Edit (19.7) and Duplicate (19.9) --
 * one `More` menu per row rather than a growing set of always-visible icon buttons, matching
 * `work-board-card.tsx`'s own per-card `ActionMenu` precedent (this list is "many independently
 * actionable rows," the same shape, not Goal Center's single always-selected detail pane).
 * Enable/Disable and the row's own select button stay directly visible -- routine, frequent
 * actions, not "consequence-aware confirmation" candidates the way Delete is.
 */
export function ScheduledTaskRow({
  agent, language, mutation, onDelete, onDismissError, onDuplicate, onEdit, onSelect, onSetEnabled, selected, task, weekdayNames,
}: ScheduledTaskRowProps) {
  const { t } = useTranslation();
  const frequencyLabel = formatScheduledTaskFrequency(task.frequency);
  const pending = mutation?.pending ?? false;

  const moreItems: ActionMenuItem[] = [
    { disabled: pending, icon: Pencil, id: "edit", label: t("scheduledTasks.edit"), onSelect: () => onEdit(task) },
    { disabled: pending, icon: Copy, id: "duplicate", label: t("scheduledTasks.duplicate"), onSelect: () => onDuplicate(task) },
    {
      confirmation: {
        confirmLabel: t("scheduledTasks.confirmDeleteAction"),
        description: t("scheduledTasks.deleteConsequence"),
        title: t("scheduledTasks.confirmDelete", { name: task.name }),
      },
      disabled: pending,
      icon: Trash2,
      id: "delete",
      label: t("scheduledTasks.delete"),
      onSelect: () => onDelete(task),
      tone: "destructive",
    },
  ];

  return (
    <li className="ucd-list-row grid gap-2 rounded-lg p-3">
      <div className="flex items-start justify-between gap-3">
        <button
          aria-current={selected}
          className="min-w-0 rounded text-left outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
          data-testid={`scheduled-task-select-${task.id}`}
          onClick={() => onSelect(task.id)}
          type="button"
        >
          <div className="truncate text-sm font-medium">{task.name}</div>
          <div className="mt-1 text-xs text-muted-foreground">{agent?.displayName ?? task.agentId}</div>
        </button>
        <div className="flex shrink-0 items-center gap-2">
          <label className="flex items-center gap-1 text-xs text-muted-foreground">
            <input checked={task.enabled} disabled={pending} onChange={(event) => onSetEnabled(task, event.target.checked)} type="checkbox" />
            {task.enabled ? t("scheduledTasks.enabled") : t("scheduledTasks.disabled")}
          </label>
          <ActionMenu items={moreItems} triggerLabel={t("workbenchUi.pageHeader.moreActions")} />
        </div>
      </div>
      <div className="grid gap-2 text-xs text-muted-foreground sm:grid-cols-3">
        <span>{t(frequencyLabel.key, frequencySummaryParams(frequencyLabel, weekdayNames))}</span>
        <span>{t("scheduledTasks.nextRun", { time: formatDateTime(task.nextRunAt, language) })}</span>
        <span className={statusClass(task.latestStatus)}>
          {t(`scheduledTasks.status.${task.latestStatus}`)}
        </span>
      </div>
      <MutationStatus onDismiss={() => onDismissError(task.id)} state={mutation} />
    </li>
  );
}
