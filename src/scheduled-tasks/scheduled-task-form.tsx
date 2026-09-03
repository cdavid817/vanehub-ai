import { useId } from "react";
import { useTranslation } from "react-i18next";
import type { AgentRegistryEntry, ScheduledTaskFrequency } from "../types/agent";
import type { ScheduledTaskDraft, ScheduledTaskDraftIssue } from "./scheduled-task-draft";
import { frequencyKinds, initialFrequency, type FrequencyKind } from "./scheduled-task-presentation";

export interface ScheduledTaskFormProps {
  agents: AgentRegistryEntry[];
  draft: ScheduledTaskDraft;
  onChange: (draft: ScheduledTaskDraft) => void;
  weekdayNames: string[];
  /** 19.7: adjacent (inline, next-to-field) validation -- `null` once the draft is fully valid.
   *  Computed once by the caller (`scheduled-task-editor-sheet.tsx`, via
   *  `validateScheduledTaskDraft`) rather than re-derived here, so the Save button's own
   *  disabled state and these hints can never disagree about which field is the problem. */
  issue: ScheduledTaskDraftIssue | null;
}

/**
 * 19.7: the fields-only half of the editor -- Create and Edit both render this over a
 * `ScheduledTaskDraft`, extracted from the former create-only, panel-owned-state version (19.3)
 * so the same markup, validation, and test ids serve both modes instead of a second copy. The
 * Save/Cancel controls and the Review restatement live in `scheduled-task-editor-sheet.tsx`,
 * which is the only thing that changes between the two modes.
 */
export function ScheduledTaskForm({ agents, draft, issue, onChange, weekdayNames }: ScheduledTaskFormProps) {
  const { t } = useTranslation();
  const nameErrorId = useId();
  const contentErrorId = useId();
  const agentErrorId = useId();
  return (
    <section className="grid content-start gap-3">
      {/* The inline error is a sibling of the `<label>`, not nested inside it: nesting would fold
          its text into the input's own computed accessible NAME (via implicit label
          association) instead of staying a separate DESCRIPTION, confusing both screen readers
          and any `getByLabelText`-style lookup. `aria-describedby` links them correctly instead. */}
      <div className="grid gap-1">
        <label className="grid gap-1">
          <span className="text-xs font-medium text-muted-foreground">{t("scheduledTasks.name")}</span>
          <input aria-describedby={issue === "name" ? nameErrorId : undefined} className="ucd-input h-9 rounded px-2 text-sm" onChange={(event) => onChange({ ...draft, name: event.target.value })} placeholder={t("scheduledTasks.namePlaceholder")} value={draft.name} />
        </label>
        {issue === "name" ? <span className="text-xs text-destructive" id={nameErrorId} role="alert">{t("scheduledTasks.validation.name")}</span> : null}
      </div>
      <div className="grid gap-1">
        <label className="grid gap-1">
          <span className="text-xs font-medium text-muted-foreground">{t("scheduledTasks.content")}</span>
          <textarea aria-describedby={issue === "content" ? contentErrorId : undefined} className="ucd-input min-h-24 rounded p-2 text-sm" onChange={(event) => onChange({ ...draft, content: event.target.value })} placeholder={t("scheduledTasks.contentPlaceholder")} value={draft.content} />
        </label>
        {issue === "content" ? <span className="text-xs text-destructive" id={contentErrorId} role="alert">{t("scheduledTasks.validation.content")}</span> : null}
      </div>
      <div className="grid gap-1">
        <label className="grid gap-1">
          <span className="text-xs font-medium text-muted-foreground">{t("scheduledTasks.agent")}</span>
          <select aria-describedby={issue === "agent" ? agentErrorId : undefined} className="ucd-input h-9 rounded px-2 text-sm" onChange={(event) => onChange({ ...draft, agentId: event.target.value })} value={draft.agentId}>
            {agents.map((agent) => <option key={agent.id} value={agent.id}>{agent.displayName}</option>)}
          </select>
        </label>
        {issue === "agent" ? <span className="text-xs text-destructive" id={agentErrorId} role="alert">{t("scheduledTasks.validation.agent")}</span> : null}
      </div>
      <FrequencyControls frequency={draft.frequency} onChange={(frequency) => onChange({ ...draft, frequency })} weekdayNames={weekdayNames} />
      {issue === "frequency" ? <span className="text-xs text-destructive" role="alert">{t("scheduledTasks.validation.frequency")}</span> : null}
      <p className="text-xs text-muted-foreground">{t("scheduledTasks.runtimeHint")}</p>
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
