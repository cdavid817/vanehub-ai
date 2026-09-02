import { useTranslation } from "react-i18next";
import { formatScheduledTaskFrequency } from "../lib/scheduled-task-recurrence";
import type { AgentRegistryEntry, ScheduledTask } from "../types/agent";
import { formatDateTime, frequencySummaryParams, statusClass } from "./scheduled-task-presentation";

export interface ScheduledTaskDetailProps {
  task: ScheduledTask | null;
  agent?: AgentRegistryEntry;
  weekdayNames: string[];
  language: string;
}

/**
 * 19.3: a placeholder detail view over fields the panel already has from `listScheduledTasks` --
 * no new fetch, no occurrence preview, no run history. Those need their own service calls and are
 * design.md task 19.6's own separate, larger scope; this exists only so 19.6 has a component to
 * grow into instead of a from-scratch build. Deliberately reuses `scheduled-task-presentation.ts`
 * and `formatScheduledTaskFrequency` exactly as `ScheduledTaskRow` does, rather than re-deriving
 * the same facts a second way.
 */
export function ScheduledTaskDetail({ agent, language, task, weekdayNames }: ScheduledTaskDetailProps) {
  const { t } = useTranslation();

  if (!task) {
    return (
      <div className="grid content-start gap-3 rounded-lg border border-dashed border-border p-4 text-center text-sm text-muted-foreground" data-testid="scheduled-task-detail">
        {t("scheduledTasks.detailEmpty")}
      </div>
    );
  }

  const frequencyLabel = formatScheduledTaskFrequency(task.frequency);

  return (
    <div className="grid content-start gap-3 rounded-lg border border-border p-3" data-testid="scheduled-task-detail">
      <div className="flex items-start justify-between gap-2">
        <h4 className="text-xs font-semibold uppercase text-muted-foreground">{t("scheduledTasks.detailTitle")}</h4>
        <span className={`shrink-0 text-xs font-medium ${task.enabled ? "text-foreground" : "text-muted-foreground"}`}>
          {task.enabled ? t("scheduledTasks.enabled") : t("scheduledTasks.disabled")}
        </span>
      </div>
      <div className="min-w-0">
        <div className="truncate text-sm font-medium">{task.name}</div>
        <div className="mt-1 text-xs text-muted-foreground">{agent?.displayName ?? task.agentId}</div>
      </div>
      <div className="grid gap-1.5 text-xs text-muted-foreground">
        <span>{t(frequencyLabel.key, frequencySummaryParams(frequencyLabel, weekdayNames))}</span>
        <span>{t("scheduledTasks.nextRun", { time: formatDateTime(task.nextRunAt, language) })}</span>
        <span className={statusClass(task.latestStatus)}>
          {t(`scheduledTasks.status.${task.latestStatus}`)}
        </span>
      </div>
    </div>
  );
}
