import { Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { formatScheduledTaskFrequency } from "../lib/scheduled-task-recurrence";
import type { AgentRegistryEntry, ScheduledTask } from "../types/agent";
import { formatDateTime, frequencySummaryParams, statusClass } from "./scheduled-task-presentation";

export interface ScheduledTaskRowProps {
  task: ScheduledTask;
  agent?: AgentRegistryEntry;
  selected: boolean;
  weekdayNames: string[];
  language: string;
  confirmingDelete: boolean;
  onSelect: (taskId: string) => void;
  onSetEnabled: (task: ScheduledTask, enabled: boolean) => void;
  onRequestDelete: (taskId: string | null) => void;
  onConfirmDelete: (task: ScheduledTask) => void;
}

/**
 * 19.3 structural extraction: moved verbatim out of `scheduled-tasks-panel.tsx`'s own `.map()` --
 * same `.ucd-list-row` class the existing Playwright spec locates rows by
 * (workspace-activity-bar.spec.ts), same enable/disable checkbox and inline delete-confirmation,
 * same text and classNames throughout. The only real addition is the name/agent `<button>`: it
 * cannot also wrap the checkbox or delete controls (a `<button>` cannot validly nest another
 * interactive control), so this is the smallest read-only region that makes the row genuinely
 * selectable without restructuring anything else about the row.
 */
export function ScheduledTaskRow({
  agent, confirmingDelete, language, onConfirmDelete, onRequestDelete, onSelect, onSetEnabled, selected, task, weekdayNames,
}: ScheduledTaskRowProps) {
  const { t } = useTranslation();
  const frequencyLabel = formatScheduledTaskFrequency(task.frequency);

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
            <input checked={task.enabled} onChange={(event) => onSetEnabled(task, event.target.checked)} type="checkbox" />
            {task.enabled ? t("scheduledTasks.enabled") : t("scheduledTasks.disabled")}
          </label>
          {confirmingDelete ? (
            <span className="flex items-center gap-1">
              <Button className="h-8 px-2 text-xs" onClick={() => onRequestDelete(null)} size="sm" variant="outline">
                {t("scheduledTasks.cancelDelete")}
              </Button>
              <Button autoFocus className="h-8 bg-destructive px-2 text-xs text-destructive-foreground" onClick={() => onConfirmDelete(task)} size="sm">
                {t("scheduledTasks.confirmDeleteAction")}
              </Button>
            </span>
          ) : (
            <Button aria-label={t("scheduledTasks.confirmDelete", { name: task.name })} className="h-8 w-8 px-0" onClick={() => onRequestDelete(task.id)} title={t("scheduledTasks.delete")} variant="outline">
              <Trash2 className="h-3.5 w-3.5" aria-hidden="true" />
            </Button>
          )}
        </div>
      </div>
      <div className="grid gap-2 text-xs text-muted-foreground sm:grid-cols-3">
        <span>{t(frequencyLabel.key, frequencySummaryParams(frequencyLabel, weekdayNames))}</span>
        <span>{t("scheduledTasks.nextRun", { time: formatDateTime(task.nextRunAt, language) })}</span>
        <span className={statusClass(task.latestStatus)}>
          {t(`scheduledTasks.status.${task.latestStatus}`)}
        </span>
      </div>
    </li>
  );
}
