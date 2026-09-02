import { Loader2, Plus } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import type { AgentRegistryEntry, ScheduledTaskFrequency } from "../types/agent";
import { frequencyKinds, initialFrequency, type FrequencyKind } from "./scheduled-task-presentation";

export interface ScheduledTaskFormProps {
  agents: AgentRegistryEntry[];
  name: string;
  onNameChange: (value: string) => void;
  content: string;
  onContentChange: (value: string) => void;
  agentId: string;
  onAgentIdChange: (value: string) => void;
  frequency: ScheduledTaskFrequency;
  onFrequencyChange: (value: ScheduledTaskFrequency) => void;
  weekdayNames: string[];
  error: string | null;
  saving: boolean;
  onSubmit: () => void;
}

/**
 * 19.3 structural extraction only, matching `EvaluationRunControls`'s own precedent
 * (evaluation-run-controls.tsx, 18.2): moved verbatim out of `scheduled-tasks-panel.tsx`, same
 * fields, same classNames, same validation, same test ids. State ownership does not move -- the
 * container still holds name/content/agentId/frequency, and this stays a controlled,
 * presentation-only view over them.
 */
export function ScheduledTaskForm({
  agentId, agents, content, error, frequency, name, onAgentIdChange, onContentChange, onFrequencyChange, onNameChange, onSubmit, saving, weekdayNames,
}: ScheduledTaskFormProps) {
  const { t } = useTranslation();
  return (
    <section className="grid content-start gap-3 rounded-lg border border-border p-3">
      <h4 className="text-xs font-semibold uppercase text-muted-foreground">{t("scheduledTasks.createTitle")}</h4>
      <label className="grid gap-1">
        <span className="text-xs font-medium text-muted-foreground">{t("scheduledTasks.name")}</span>
        <input className="ucd-input h-9 rounded px-2 text-sm" onChange={(event) => onNameChange(event.target.value)} placeholder={t("scheduledTasks.namePlaceholder")} value={name} />
      </label>
      <label className="grid gap-1">
        <span className="text-xs font-medium text-muted-foreground">{t("scheduledTasks.content")}</span>
        <textarea className="ucd-input min-h-24 rounded p-2 text-sm" onChange={(event) => onContentChange(event.target.value)} placeholder={t("scheduledTasks.contentPlaceholder")} value={content} />
      </label>
      <label className="grid gap-1">
        <span className="text-xs font-medium text-muted-foreground">{t("scheduledTasks.agent")}</span>
        <select className="ucd-input h-9 rounded px-2 text-sm" onChange={(event) => onAgentIdChange(event.target.value)} value={agentId}>
          {agents.map((agent) => <option key={agent.id} value={agent.id}>{agent.displayName}</option>)}
        </select>
      </label>
      <FrequencyControls frequency={frequency} onChange={onFrequencyChange} weekdayNames={weekdayNames} />
      <p className="text-xs text-muted-foreground">{t("scheduledTasks.runtimeHint")}</p>
      <div className="flex items-start justify-between gap-3">
        <p className="min-w-0 flex-1 wrap-break-word text-xs leading-5 text-destructive" role="alert">{error}</p>
        <Button className="h-8 shrink-0 px-3 text-xs" disabled={!name.trim() || !content.trim() || !agentId || saving} onClick={onSubmit} type="button">
          {saving ? <Loader2 className="h-3.5 w-3.5 animate-spin" aria-hidden="true" /> : <Plus className="h-3.5 w-3.5" aria-hidden="true" />}
          {t("scheduledTasks.create")}
        </Button>
      </div>
    </section>
  );
}

function FrequencyControls({
  frequency,
  onChange,
  weekdayNames,
}: {
  frequency: ScheduledTaskFrequency;
  onChange: (frequency: ScheduledTaskFrequency) => void;
  weekdayNames: string[];
}) {
  const { t } = useTranslation();
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
          <select className="ucd-input h-9 rounded px-2 text-sm" data-testid="scheduled-task-weekday" onChange={(event) => onChange({ ...frequency, weekday: Number(event.target.value) })} value={frequency.weekday}>
            {weekdayNames.map((label, index) => <option key={index} value={index}>{label}</option>)}
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
