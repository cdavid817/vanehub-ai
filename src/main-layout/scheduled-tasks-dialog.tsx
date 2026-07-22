import { useEffect, useMemo, useState } from "react";
import { Loader2, Plus, Trash2, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { formatScheduledTaskFrequency } from "../lib/scheduled-task-recurrence";
import { agentService } from "../services/runtime-agent-client";
import type { AgentRegistryEntry, ScheduledTask, ScheduledTaskFrequency } from "../types/agent";

type FrequencyKind = ScheduledTaskFrequency["kind"];

const frequencyKinds: FrequencyKind[] = ["minutes", "hours", "daily", "weekly", "monthly"];
const weekdayLabels = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

function initialFrequency(kind: FrequencyKind): ScheduledTaskFrequency {
  switch (kind) {
    case "minutes":
      return { kind, interval: 30 };
    case "hours":
      return { kind, interval: 1 };
    case "daily":
      return { kind, timeOfDay: "09:00" };
    case "weekly":
      return { kind, weekday: 1, timeOfDay: "09:00" };
    case "monthly":
      return { kind, dayOfMonth: 1, timeOfDay: "09:00" };
  }
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

function statusClass(status: ScheduledTask["latestStatus"]) {
  if (status === "failed") return "text-destructive";
  if (status === "succeeded") return "text-[hsl(var(--success))]";
  if (status === "running") return "text-primary";
  return "text-muted-foreground";
}

export function ScheduledTasksDialog({
  agents,
  onClose,
  open,
}: {
  agents: AgentRegistryEntry[];
  onClose: () => void;
  open: boolean;
}) {
  const { i18n, t } = useTranslation();
  const [tasks, setTasks] = useState<ScheduledTask[]>([]);
  const [name, setName] = useState("");
  const [content, setContent] = useState("");
  const [agentId, setAgentId] = useState("");
  const [frequency, setFrequency] = useState<ScheduledTaskFrequency>(initialFrequency("daily"));
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const selectableAgents = useMemo(
    () => agents.filter((agent) => agent.supportedInteractionModes.includes("cli")),
    [agents],
  );

  async function loadTasks() {
    setLoading(true);
    setError(null);
    try {
      setTasks(await agentService.listScheduledTasks());
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    if (!open) return;
    setName("");
    setContent("");
    setFrequency(initialFrequency("daily"));
    setAgentId(selectableAgents[0]?.id ?? "");
    void loadTasks();
  }, [open, selectableAgents]);

  async function createTask() {
    if (!name.trim() || !content.trim() || !agentId) return;
    setSaving(true);
    setError(null);
    try {
      await agentService.createScheduledTask({ name, content, agentId, frequency });
      setName("");
      setContent("");
      setFrequency(initialFrequency("daily"));
      await loadTasks();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setSaving(false);
    }
  }

  async function setEnabled(task: ScheduledTask, enabled: boolean) {
    setError(null);
    try {
      const updated = await agentService.setScheduledTaskEnabled({ taskId: task.id, enabled });
      setTasks((current) => current.map((candidate) => (candidate.id === task.id ? updated : candidate)));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  async function deleteTask(task: ScheduledTask) {
    if (!window.confirm(t("scheduledTasks.confirmDelete", { name: task.name }))) return;
    setError(null);
    try {
      await agentService.deleteScheduledTask(task.id);
      setTasks((current) => current.filter((candidate) => candidate.id !== task.id));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-background/70 p-4">
      <div className="ucd-panel grid max-h-[90vh] w-full max-w-5xl grid-rows-[auto_minmax(0,1fr)_auto] overflow-hidden rounded-lg shadow-xl">
        <div className="flex items-center justify-between border-b border-border p-4">
          <div>
            <h3 className="text-sm font-semibold">{t("scheduledTasks.title")}</h3>
            <p className="mt-1 text-xs text-muted-foreground">{t("scheduledTasks.description")}</p>
          </div>
          <Button className="h-8 w-8 px-0" onClick={onClose} title={t("scheduledTasks.close")} variant="outline">
            <X className="h-4 w-4" aria-hidden="true" />
          </Button>
        </div>

        <div className="grid min-h-0 gap-4 overflow-y-auto p-4 lg:grid-cols-[minmax(0,1fr)_320px]">
          <section className="min-h-0">
            <div className="mb-3 flex items-center justify-between">
              <h4 className="text-xs font-semibold uppercase text-muted-foreground">{t("scheduledTasks.listTitle")}</h4>
              {loading ? <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" aria-hidden="true" /> : null}
            </div>
            <div className="grid gap-2">
              {tasks.length === 0 && !loading ? (
                <div className="rounded-lg border border-dashed border-border p-6 text-center text-sm text-muted-foreground">
                  {t("scheduledTasks.empty")}
                </div>
              ) : null}
              {tasks.map((task) => {
                const agent = agents.find((candidate) => candidate.id === task.agentId);
                return (
                  <div className="ucd-list-row grid gap-2 rounded-lg p-3" key={task.id}>
                    <div className="flex items-start justify-between gap-3">
                      <div className="min-w-0">
                        <div className="truncate text-sm font-medium">{task.name}</div>
                        <div className="mt-1 text-xs text-muted-foreground">{agent?.displayName ?? task.agentId}</div>
                      </div>
                      <div className="flex shrink-0 items-center gap-2">
                        <label className="flex items-center gap-1 text-xs text-muted-foreground">
                          <input checked={task.enabled} onChange={(event) => void setEnabled(task, event.target.checked)} type="checkbox" />
                          {task.enabled ? t("scheduledTasks.enabled") : t("scheduledTasks.disabled")}
                        </label>
                        <Button className="h-8 w-8 px-0" onClick={() => void deleteTask(task)} title={t("scheduledTasks.delete")} variant="outline">
                          <Trash2 className="h-3.5 w-3.5" aria-hidden="true" />
                        </Button>
                      </div>
                    </div>
                    <div className="grid gap-2 text-xs text-muted-foreground sm:grid-cols-3">
                      <span>{formatScheduledTaskFrequency(task.frequency, weekdayLabels)}</span>
                      <span>{t("scheduledTasks.nextRun", { time: formatDateTime(task.nextRunAt, i18n.language) })}</span>
                      <span className={statusClass(task.latestStatus)}>
                        {t(`scheduledTasks.status.${task.latestStatus}`)}
                      </span>
                    </div>
                  </div>
                );
              })}
            </div>
          </section>

          <section className="grid content-start gap-3 rounded-lg border border-border p-3">
            <h4 className="text-xs font-semibold uppercase text-muted-foreground">{t("scheduledTasks.createTitle")}</h4>
            <label className="grid gap-1">
              <span className="text-xs font-medium text-muted-foreground">{t("scheduledTasks.name")}</span>
              <input className="ucd-input h-9 rounded px-2 text-sm" onChange={(event) => setName(event.target.value)} placeholder={t("scheduledTasks.namePlaceholder")} value={name} />
            </label>
            <label className="grid gap-1">
              <span className="text-xs font-medium text-muted-foreground">{t("scheduledTasks.content")}</span>
              <textarea className="ucd-input min-h-24 rounded p-2 text-sm" onChange={(event) => setContent(event.target.value)} placeholder={t("scheduledTasks.contentPlaceholder")} value={content} />
            </label>
            <label className="grid gap-1">
              <span className="text-xs font-medium text-muted-foreground">{t("scheduledTasks.agent")}</span>
              <select className="ucd-input h-9 rounded px-2 text-sm" onChange={(event) => setAgentId(event.target.value)} value={agentId}>
                {selectableAgents.map((agent) => <option key={agent.id} value={agent.id}>{agent.displayName}</option>)}
              </select>
            </label>
            <FrequencyControls frequency={frequency} onChange={setFrequency} t={t} />
            <p className="text-xs text-muted-foreground">{t("scheduledTasks.runtimeHint")}</p>
          </section>
        </div>

        <div className="flex items-center justify-between gap-3 border-t border-border p-4">
          <span className="min-w-0 truncate text-xs text-destructive">{error}</span>
          <Button className="h-8 px-3 text-xs" disabled={!name.trim() || !content.trim() || !agentId || saving} onClick={() => void createTask()} type="button">
            {saving ? <Loader2 className="h-3.5 w-3.5 animate-spin" aria-hidden="true" /> : <Plus className="h-3.5 w-3.5" aria-hidden="true" />}
            {t("scheduledTasks.create")}
          </Button>
        </div>
      </div>
    </div>
  );
}

function FrequencyControls({
  frequency,
  onChange,
  t,
}: {
  frequency: ScheduledTaskFrequency;
  onChange: (frequency: ScheduledTaskFrequency) => void;
  t: (key: string) => string;
}) {
  return (
    <div className="grid gap-2">
      <label className="grid gap-1">
        <span className="text-xs font-medium text-muted-foreground">{t("scheduledTasks.frequency")}</span>
        <select className="ucd-input h-9 rounded px-2 text-sm" onChange={(event) => onChange(initialFrequency(event.target.value as FrequencyKind))} value={frequency.kind}>
          {frequencyKinds.map((kind) => <option key={kind} value={kind}>{t(`scheduledTasks.frequency.${kind}`)}</option>)}
        </select>
      </label>
      {(frequency.kind === "minutes" || frequency.kind === "hours") ? (
        <input className="ucd-input h-9 rounded px-2 text-sm" min={1} onChange={(event) => onChange({ ...frequency, interval: Number(event.target.value) })} type="number" value={frequency.interval} />
      ) : null}
      {frequency.kind === "daily" ? (
        <input className="ucd-input h-9 rounded px-2 text-sm" onChange={(event) => onChange({ ...frequency, timeOfDay: event.target.value })} type="time" value={frequency.timeOfDay} />
      ) : null}
      {frequency.kind === "weekly" ? (
        <div className="grid grid-cols-2 gap-2">
          <select className="ucd-input h-9 rounded px-2 text-sm" onChange={(event) => onChange({ ...frequency, weekday: Number(event.target.value) })} value={frequency.weekday}>
            {weekdayLabels.map((label, index) => <option key={label} value={index}>{label}</option>)}
          </select>
          <input className="ucd-input h-9 rounded px-2 text-sm" onChange={(event) => onChange({ ...frequency, timeOfDay: event.target.value })} type="time" value={frequency.timeOfDay} />
        </div>
      ) : null}
      {frequency.kind === "monthly" ? (
        <div className="grid grid-cols-2 gap-2">
          <input className="ucd-input h-9 rounded px-2 text-sm" max={31} min={1} onChange={(event) => onChange({ ...frequency, dayOfMonth: Number(event.target.value) })} type="number" value={frequency.dayOfMonth} />
          <input className="ucd-input h-9 rounded px-2 text-sm" onChange={(event) => onChange({ ...frequency, timeOfDay: event.target.value })} type="time" value={frequency.timeOfDay} />
        </div>
      ) : null}
    </div>
  );
}
