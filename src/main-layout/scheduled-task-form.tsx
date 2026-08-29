import { CalendarClock, Info } from "lucide-react";
import { useId, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { AgentRegistryEntry, ScheduledTaskFrequency } from "../types/agent";
import {
  initialFrequency,
  isValidScheduledTaskFrequency,
  type FrequencyKind,
  type ScheduledTaskDraft,
} from "./scheduled-task-model";

const frequencyKinds: FrequencyKind[] = ["minutes", "hours", "daily", "weekly", "monthly"];

export function ScheduledTaskForm({
  agents,
  disabled,
  draft,
  onChange,
}: {
  agents: AgentRegistryEntry[];
  disabled: boolean;
  draft: ScheduledTaskDraft;
  onChange: (draft: ScheduledTaskDraft) => void;
}) {
  const { t } = useTranslation();
  const nameId = useId();
  const contentId = useId();
  const agentId = useId();

  return (
    <section className="order-first grid content-start gap-4 lg:order-last" aria-labelledby="scheduled-task-create-title">
      <div className="flex items-center gap-2">
        <span className="flex h-8 w-8 items-center justify-center rounded-md bg-primary/10 text-primary">
          <CalendarClock className="h-4 w-4" aria-hidden="true" />
        </span>
        <div>
          <h4 className="text-sm font-semibold" id="scheduled-task-create-title">{t("scheduledTasks.createTitle")}</h4>
          <p className="text-xs text-muted-foreground">{t("scheduledTasks.createSubtitle")}</p>
        </div>
      </div>

      <div className="grid gap-3">
        <Field id={nameId} label={t("scheduledTasks.name")}>
          <input
            aria-invalid={!draft.name.trim()}
            className="ucd-input h-9 rounded-md px-2.5 text-sm"
            data-dialog-autofocus
            disabled={disabled}
            id={nameId}
            onChange={(event) => onChange({ ...draft, name: event.target.value })}
            placeholder={t("scheduledTasks.namePlaceholder")}
            required
            value={draft.name}
          />
        </Field>
        <Field id={contentId} label={t("scheduledTasks.content")}>
          <textarea
            aria-invalid={!draft.content.trim()}
            className="ucd-input min-h-24 resize-y rounded-md p-2.5 text-sm leading-5"
            disabled={disabled}
            id={contentId}
            onChange={(event) => onChange({ ...draft, content: event.target.value })}
            placeholder={t("scheduledTasks.contentPlaceholder")}
            required
            value={draft.content}
          />
        </Field>
        <Field id={agentId} label={t("scheduledTasks.agent")}>
          <select
            aria-invalid={!draft.agentId}
            className="ucd-input h-9 rounded-md px-2.5 text-sm"
            disabled={disabled || agents.length === 0}
            id={agentId}
            onChange={(event) => onChange({ ...draft, agentId: event.target.value })}
            required
            value={draft.agentId}
          >
            {agents.length === 0 ? <option value="">{t("scheduledTasks.noAgents")}</option> : null}
            {agents.map((agent) => <option key={agent.id} value={agent.id}>{agent.displayName}</option>)}
          </select>
        </Field>
        <FrequencyControls disabled={disabled} frequency={draft.frequency} onChange={(frequency) => onChange({ ...draft, frequency })} />
      </div>

      <div className="flex gap-2 rounded-md border border-border bg-muted/50 p-3 text-xs leading-5 text-muted-foreground">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-primary" aria-hidden="true" />
        <p>{t("scheduledTasks.runtimeHint")}</p>
      </div>
    </section>
  );
}

function Field({ children, id, label }: { children: ReactNode; id: string; label: string }) {
  return (
    <div className="grid gap-1.5">
      <label className="text-xs font-medium text-foreground" htmlFor={id}>{label}</label>
      {children}
    </div>
  );
}

function FrequencyControls({
  disabled,
  frequency,
  onChange,
}: {
  disabled: boolean;
  frequency: ScheduledTaskFrequency;
  onChange: (frequency: ScheduledTaskFrequency) => void;
}) {
  const { t } = useTranslation();
  const frequencyId = useId();
  const parameterId = useId();
  const secondaryId = useId();
  const errorId = useId();
  const valid = isValidScheduledTaskFrequency(frequency);

  return (
    <div className="grid gap-2 rounded-md border border-border bg-muted/30 p-3">
      <Field id={frequencyId} label={t("scheduledTasks.frequency")}>
        <select className="ucd-input h-9 rounded-md px-2.5 text-sm" disabled={disabled} id={frequencyId} onChange={(event) => onChange(initialFrequency(event.target.value as FrequencyKind))} value={frequency.kind}>
          {frequencyKinds.map((kind) => <option key={kind} value={kind}>{t(`scheduledTasks.frequency.${kind}`)}</option>)}
        </select>
      </Field>
      {(frequency.kind === "minutes" || frequency.kind === "hours") ? (
        <Field id={parameterId} label={t("scheduledTasks.interval")}>
          <div className="relative">
            <input aria-describedby={!valid ? errorId : undefined} aria-invalid={!valid} className="ucd-input h-9 w-full rounded-md px-2.5 pr-16 text-sm" disabled={disabled} id={parameterId} min={1} onChange={(event) => onChange({ ...frequency, interval: Number(event.target.value) })} required type="number" value={frequency.interval || ""} />
            <span className="pointer-events-none absolute inset-y-0 right-2.5 flex items-center text-xs text-muted-foreground">{t(`scheduledTasks.unit.${frequency.kind}`)}</span>
          </div>
        </Field>
      ) : null}
      {frequency.kind === "daily" ? <TimeField disabled={disabled} id={parameterId} onChange={(timeOfDay) => onChange({ ...frequency, timeOfDay })} value={frequency.timeOfDay} /> : null}
      {frequency.kind === "weekly" ? (
        <div className="grid grid-cols-2 gap-2">
          <Field id={parameterId} label={t("scheduledTasks.weekday")}>
            <select className="ucd-input h-9 rounded-md px-2.5 text-sm" disabled={disabled} id={parameterId} onChange={(event) => onChange({ ...frequency, weekday: Number(event.target.value) })} value={frequency.weekday}>
              {Array.from({ length: 7 }, (_, index) => <option key={index} value={index}>{t(`scheduledTasks.weekday.${index}`)}</option>)}
            </select>
          </Field>
          <TimeField disabled={disabled} id={secondaryId} onChange={(timeOfDay) => onChange({ ...frequency, timeOfDay })} value={frequency.timeOfDay} />
        </div>
      ) : null}
      {frequency.kind === "monthly" ? (
        <div className="grid grid-cols-2 gap-2">
          <Field id={parameterId} label={t("scheduledTasks.dayOfMonth")}>
            <input aria-describedby={!valid ? errorId : undefined} aria-invalid={!valid} className="ucd-input h-9 rounded-md px-2.5 text-sm" disabled={disabled} id={parameterId} max={31} min={1} onChange={(event) => onChange({ ...frequency, dayOfMonth: Number(event.target.value) })} required type="number" value={frequency.dayOfMonth || ""} />
          </Field>
          <TimeField disabled={disabled} id={secondaryId} onChange={(timeOfDay) => onChange({ ...frequency, timeOfDay })} value={frequency.timeOfDay} />
        </div>
      ) : null}
      {!valid ? <p className="text-xs text-destructive" id={errorId} role="alert">{t("scheduledTasks.frequencyError")}</p> : null}
    </div>
  );
}

function TimeField({ disabled, id, onChange, value }: { disabled: boolean; id: string; onChange: (value: string) => void; value: string }) {
  const { t } = useTranslation();
  return (
    <Field id={id} label={t("scheduledTasks.timeOfDay")}>
      <input className="ucd-input h-9 rounded-md px-2.5 text-sm" disabled={disabled} id={id} onChange={(event) => onChange(event.target.value)} required type="time" value={value} />
    </Field>
  );
}
