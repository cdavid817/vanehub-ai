import {
  AlertTriangle,
  CalendarClock,
  CheckCircle2,
  CircleSlash2,
  Clock3,
  Loader2,
  Trash2,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { formatScheduledTaskFrequency } from "../lib/scheduled-task-recurrence";
import type { AgentRegistryEntry, ScheduledTask } from "../types/agent";

export interface ScheduledTaskMutation {
  action: "delete" | "disable" | "enable";
  taskId: string;
}

interface ScheduledTaskListProps {
  agents: AgentRegistryEntry[];
  confirmingDeleteId: string | null;
  loading: boolean;
  mutation: ScheduledTaskMutation | null;
  onCancelDelete: () => void;
  onConfirmDelete: (task: ScheduledTask) => void;
  onRequestDelete: (taskId: string) => void;
  onSetEnabled: (task: ScheduledTask, enabled: boolean) => void;
  tasks: ScheduledTask[];
}

function formatDateTime(value: string | null, language: string) {
  if (!value) return "-";
  return new Intl.DateTimeFormat(language, {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(value));
}

function statusPresentation(status: ScheduledTask["latestStatus"]) {
  switch (status) {
    case "failed":
      return { icon: AlertTriangle, tone: "danger" as const };
    case "succeeded":
      return { icon: CheckCircle2, tone: "success" as const };
    case "running":
      return { icon: Loader2, tone: "default" as const };
    case "skipped":
      return { icon: CircleSlash2, tone: "warning" as const };
    case "never-run":
      return { icon: Clock3, tone: "muted" as const };
  }
}

export function ScheduledTaskList({
  agents,
  confirmingDeleteId,
  loading,
  mutation,
  onCancelDelete,
  onConfirmDelete,
  onRequestDelete,
  onSetEnabled,
  tasks,
}: ScheduledTaskListProps) {
  const { i18n, t } = useTranslation();
  const enabledCount = tasks.filter((task) => task.enabled).length;
  const weekdays = Array.from({ length: 7 }, (_, index) => t(`scheduledTasks.weekday.${index}`));

  return (
    <section className="order-last grid min-h-0 content-start gap-3 lg:order-first" aria-labelledby="scheduled-task-list-title">
      <div className="flex min-h-8 items-center justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2">
          <h4 className="text-sm font-semibold" id="scheduled-task-list-title">{t("scheduledTasks.listTitle")}</h4>
          <Badge tone="muted">{t("scheduledTasks.taskCount", { count: tasks.length })}</Badge>
          {tasks.length > 0 ? <span className="text-xs text-muted-foreground">{t("scheduledTasks.enabledCount", { count: enabledCount })}</span> : null}
        </div>
        {loading ? (
          <span className="flex shrink-0 items-center gap-1.5 text-xs text-muted-foreground" role="status">
            <Loader2 className="h-3.5 w-3.5 animate-spin" aria-hidden="true" />
            {tasks.length > 0 ? t("scheduledTasks.refreshing") : t("scheduledTasks.loading")}
          </span>
        ) : null}
      </div>

      {tasks.length === 0 && !loading ? (
        <div className="grid min-h-48 place-items-center rounded-md border border-dashed border-border bg-muted/20 p-6 text-center">
          <div>
            <span className="mx-auto flex h-10 w-10 items-center justify-center rounded-md bg-muted text-muted-foreground">
              <CalendarClock className="h-5 w-5" aria-hidden="true" />
            </span>
            <p className="mt-3 text-sm font-medium">{t("scheduledTasks.empty")}</p>
            <p className="mt-1 text-xs leading-5 text-muted-foreground">{t("scheduledTasks.emptyHint")}</p>
          </div>
        </div>
      ) : null}

      <div className="grid gap-2 lg:max-h-[56vh] lg:overflow-y-auto lg:pr-1">
        {tasks.map((task) => (
          <ScheduledTaskRow
            agentName={agents.find((agent) => agent.id === task.agentId)?.displayName ?? task.agentId}
            confirmingDelete={confirmingDeleteId === task.id}
            frequency={formatScheduledTaskFrequency(task.frequency, (key, values) => t(key, values), weekdays)}
            key={task.id}
            language={i18n.language}
            mutation={mutation?.taskId === task.id ? mutation : null}
            onCancelDelete={onCancelDelete}
            onConfirmDelete={() => onConfirmDelete(task)}
            onRequestDelete={() => onRequestDelete(task.id)}
            onSetEnabled={(enabled) => onSetEnabled(task, enabled)}
            task={task}
          />
        ))}
      </div>
    </section>
  );
}

function ScheduledTaskRow({
  agentName,
  confirmingDelete,
  frequency,
  language,
  mutation,
  onCancelDelete,
  onConfirmDelete,
  onRequestDelete,
  onSetEnabled,
  task,
}: {
  agentName: string;
  confirmingDelete: boolean;
  frequency: string;
  language: string;
  mutation: ScheduledTaskMutation | null;
  onCancelDelete: () => void;
  onConfirmDelete: () => void;
  onRequestDelete: () => void;
  onSetEnabled: (enabled: boolean) => void;
  task: ScheduledTask;
}) {
  const { t } = useTranslation();
  const status = statusPresentation(task.latestStatus);
  const StatusIcon = status.icon;
  const pending = mutation !== null;

  return (
    <article className="ucd-list-row grid gap-3 rounded-md p-3" data-scheduled-task-id={task.id}>
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <h5 className="truncate text-sm font-semibold" title={task.name}>{task.name}</h5>
          <p className="mt-1 line-clamp-2 text-xs leading-5 text-muted-foreground">{task.content}</p>
        </div>
        <div className="flex shrink-0 items-center gap-1.5">
          <button
            aria-checked={task.enabled}
            aria-label={t(task.enabled ? "scheduledTasks.disableTask" : "scheduledTasks.enableTask", { name: task.name })}
            className="inline-flex h-8 items-center gap-1.5 rounded-md border border-border bg-background px-2 text-xs font-medium outline-hidden transition-colors hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50"
            disabled={pending}
            onClick={() => onSetEnabled(!task.enabled)}
            role="switch"
            type="button"
          >
            {pending && mutation.action !== "delete" ? <Loader2 className="h-3 w-3 animate-spin" aria-hidden="true" /> : <span className={task.enabled ? "h-1.5 w-1.5 rounded-full bg-[hsl(var(--success))]" : "h-1.5 w-1.5 rounded-full bg-muted-foreground"} />}
            {task.enabled ? t("scheduledTasks.enabled") : t("scheduledTasks.disabled")}
          </button>
          {confirmingDelete ? (
            <span className="flex items-center gap-1">
              <Button className="h-8 px-2 text-xs" disabled={pending} onClick={onCancelDelete} size="sm" variant="outline">{t("scheduledTasks.cancelDelete")}</Button>
              <Button className="h-8 bg-destructive px-2 text-xs text-destructive-foreground" disabled={pending} onClick={onConfirmDelete} size="sm">
                {pending ? <Loader2 className="animate-spin" aria-hidden="true" /> : null}
                {t("scheduledTasks.confirmDeleteAction")}
              </Button>
            </span>
          ) : (
            <Button aria-label={t("scheduledTasks.confirmDelete", { name: task.name })} className="h-8 w-8 px-0" disabled={pending} onClick={onRequestDelete} title={t("scheduledTasks.delete")} variant="outline">
              {pending && mutation.action === "delete" ? <Loader2 className="animate-spin" aria-hidden="true" /> : <Trash2 aria-hidden="true" />}
            </Button>
          )}
        </div>
      </div>

      <div className="flex flex-wrap items-center gap-1.5 text-xs text-muted-foreground">
        <Badge tone="muted">{agentName}</Badge>
        <span className="rounded-sm bg-muted px-2 py-1">{frequency}</span>
        <span className="rounded-sm bg-muted px-2 py-1">{t("scheduledTasks.nextRun", { time: formatDateTime(task.nextRunAt, language) })}</span>
        <Badge tone={status.tone}>
          <StatusIcon className={task.latestStatus === "running" ? "mr-1 h-3 w-3 animate-spin" : "mr-1 h-3 w-3"} aria-hidden="true" />
          {t(`scheduledTasks.status.${task.latestStatus}`)}
        </Badge>
      </div>
      {task.latestError ? <p className="truncate text-xs text-destructive" title={task.latestError}>{task.latestError}</p> : null}
    </article>
  );
}
