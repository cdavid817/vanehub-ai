import { Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { AgentRegistryEntry, ScheduledTask } from "../types/agent";
import { ScheduledTaskRow } from "./scheduled-task-row";

export interface ScheduledTaskListProps {
  tasks: ScheduledTask[];
  agents: AgentRegistryEntry[];
  loading: boolean;
  selectedId: string | null;
  weekdayNames: string[];
  language: string;
  confirmingDeleteId: string | null;
  onSelect: (taskId: string) => void;
  onSetEnabled: (task: ScheduledTask, enabled: boolean) => void;
  onRequestDelete: (taskId: string | null) => void;
  onConfirmDelete: (task: ScheduledTask) => void;
}

/**
 * 19.3 structural extraction: the list half of what used to be one 265-line
 * `scheduled-tasks-panel.tsx` (list, create form, and `FrequencyControls` all inline in one file).
 * Row markup moved verbatim into `ScheduledTaskRow`; this component keeps only the heading,
 * loading spinner, and empty state that used to sit directly around the `.map()`.
 */
export function ScheduledTaskList({
  agents, confirmingDeleteId, language, loading, onConfirmDelete, onRequestDelete, onSelect, onSetEnabled, selectedId, tasks, weekdayNames,
}: ScheduledTaskListProps) {
  const { t } = useTranslation();
  return (
    <section className="min-h-0">
      <div className="mb-3 flex items-center justify-between">
        <h4 className="text-xs font-semibold uppercase text-muted-foreground">{t("scheduledTasks.listTitle")}</h4>
        {loading ? <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" aria-hidden="true" /> : null}
      </div>
      <ul aria-label={t("scheduledTasks.listTitle")} className="grid gap-2">
        {tasks.length === 0 && !loading ? (
          <li className="rounded-lg border border-dashed border-border p-6 text-center text-sm text-muted-foreground">
            {t("scheduledTasks.empty")}
          </li>
        ) : null}
        {tasks.map((task) => (
          <ScheduledTaskRow
            agent={agents.find((candidate) => candidate.id === task.agentId)}
            confirmingDelete={confirmingDeleteId === task.id}
            key={task.id}
            language={language}
            onConfirmDelete={onConfirmDelete}
            onRequestDelete={onRequestDelete}
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
