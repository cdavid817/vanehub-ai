import { useTranslation } from "react-i18next";
import { formatScheduledTaskFrequency } from "../lib/scheduled-task-recurrence";
import type { AgentRegistryEntry } from "../types/agent";
import type { ScheduledTaskDraft } from "./scheduled-task-draft";
import { frequencySummaryParams } from "./scheduled-task-presentation";
import { ScheduledTaskExecutionNotice } from "./scheduled-task-execution-notice";

function SummaryRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[minmax(6rem,0.35fr)_1fr] gap-2 text-xs">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="min-w-0 wrap-break-word whitespace-pre-line font-medium text-foreground">{value}</dd>
    </div>
  );
}

/**
 * 19.7's final Review: a plain restatement of the fields `ScheduledTaskForm` (rendered directly
 * above this, in the same scrollable Sheet) already collected -- nothing new to compute, matching
 * `EvaluationReviewStep`'s and `CreateSessionStep4`'s own "nothing new to compute" shape for
 * this exact kind of pre-commit summary. No per-row "jump to step" affordance the way
 * `EvaluationReviewStep`'s `SummaryRow` has one: this editor is single-page-with-sections, not a
 * multi-step wizard, so every field is already directly editable in the section above -- there is
 * no separate step to jump back to.
 */
export function ScheduledTaskReview({ agent, draft, weekdayNames }: {
  agent: AgentRegistryEntry | undefined;
  draft: ScheduledTaskDraft;
  weekdayNames: string[];
}) {
  const { t } = useTranslation();
  const frequencyLabel = formatScheduledTaskFrequency(draft.frequency);
  return (
    <section aria-label={t("scheduledTasks.editor.reviewTitle")} className="grid gap-2 rounded-lg border border-border bg-muted/10 p-3">
      <h4 className="text-xs font-semibold uppercase text-muted-foreground">{t("scheduledTasks.editor.reviewTitle")}</h4>
      <dl className="grid gap-2">
        <SummaryRow label={t("scheduledTasks.name")} value={draft.name.trim() || "—"} />
        <SummaryRow label={t("scheduledTasks.agent")} value={agent?.displayName ?? (draft.agentId || "—")} />
        <SummaryRow label={t("scheduledTasks.frequency")} value={t(frequencyLabel.key, frequencySummaryParams(frequencyLabel, weekdayNames))} />
        <SummaryRow label={t("scheduledTasks.content")} value={draft.content.trim() || "—"} />
      </dl>
      <ScheduledTaskExecutionNotice />
    </section>
  );
}
