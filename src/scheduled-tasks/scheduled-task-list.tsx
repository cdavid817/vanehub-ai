import { Loader2, Plus } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import type { MutationState } from "../ui/async/mutation-state";
import type { AgentRegistryEntry, ScheduledTask } from "../types/agent";
import { ScheduledTaskRow } from "./scheduled-task-row";

export interface ScheduledTaskListProps {
  tasks: ScheduledTask[];
  agents: AgentRegistryEntry[];
  loading: boolean;
  selectedId: string | null;
  weekdayNames: string[];
  language: string;
  getMutation: (taskId: string) => MutationState | undefined;
  onSelect: (taskId: string) => void;
  onSetEnabled: (task: ScheduledTask, enabled: boolean) => void;
  onNew: () => void;
  onEdit: (task: ScheduledTask) => void;
  onDuplicate: (task: ScheduledTask) => void;
  onDelete: (task: ScheduledTask) => void;
  onDismissError: (taskId: string) => void;
  /** 19.4: distinguishes "no tasks exist yet" from "a real filter/search narrowed this to zero" --
   *  the two are different facts and the pre-existing `scheduledTasks.empty` copy ("No scheduled
   *  tasks yet") would be actively misleading for the second case, mirroring
   *  `WorkBoardList`'s own `filtersActive`-gated empty copy. */
  filtersActive: boolean;
}

/**
 * 19.3 structural extraction, extended by 19.7/19.16: the list half of what used to be one
 * 265-line `scheduled-tasks-panel.tsx`. `onNew` replaces the old always-visible inline create
 * form (moved into `ScheduledTaskEditorSheet`, 19.7) with a single trigger button here, matching
 * `GoalCenter`'s own `PageHeader primaryAction` "New" button precedent -- this list has no
 * `PageHeader` of its own, so the trigger sits in this header row instead.
 */
export function ScheduledTaskList({
  agents, filtersActive, getMutation, language, loading, onDelete, onDismissError, onDuplicate, onEdit, onNew, onSelect, onSetEnabled, selectedId, tasks, weekdayNames,
}: ScheduledTaskListProps) {
  const { t } = useTranslation();
  const zone = Intl.DateTimeFormat().resolvedOptions().timeZone;
  return (
    <section className="min-h-0">
      <div className="mb-3 flex items-center justify-between gap-2">
        <span className="flex items-center gap-2">
          <h4 className="text-xs font-semibold uppercase text-muted-foreground">{t("scheduledTasks.listTitle")}</h4>
          {loading ? <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" aria-hidden="true" /> : null}
        </span>
        <Button className="h-7 px-2 text-xs" onClick={onNew} size="sm" type="button">
          <Plus className="h-3.5 w-3.5" aria-hidden="true" />{t("scheduledTasks.createTitle")}
        </Button>
      </div>
      {/* 19.5: a shared, once-per-list fact -- not a per-row column. Every task's `nextRunAt` is
          this same device's own OS-local clock (confirmed absent as a per-task field, see
          `ScheduledTaskExecutionNotice`'s own doc comment); repeating it on every row would
          misleadingly imply a per-task setting that does not exist. */}
      {tasks.length > 0 ? <p className="mb-2 text-[11px] text-muted-foreground">{t("scheduledTasks.listTimezoneCaption", { zone })}</p> : null}
      <ul aria-label={t("scheduledTasks.listTitle")} className="grid gap-2">
        {tasks.length === 0 && !loading ? (
          <li className="rounded-lg border border-dashed border-border p-6 text-center text-sm text-muted-foreground">
            {filtersActive ? t("scheduledTasks.emptyFiltered") : t("scheduledTasks.empty")}
          </li>
        ) : null}
        {tasks.map((task) => (
          <ScheduledTaskRow
            agent={agents.find((candidate) => candidate.id === task.agentId)}
            key={task.id}
            language={language}
            mutation={getMutation(task.id)}
            onDelete={onDelete}
            onDismissError={onDismissError}
            onDuplicate={onDuplicate}
            onEdit={onEdit}
            onSelect={onSelect}
            onSetEnabled={onSetEnabled}
            selected={task.id === selectedId}
            task={task}
            weekdayNames={weekdayNames}
          />
        ))}
      </ul>
    </section>
  );
}
