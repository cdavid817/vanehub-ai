import { Pencil, TriangleAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { AgentRegistryEntry } from "../types/agent";
import type { EvaluationTask } from "../types/evaluation";
import { isEvaluationAgentIncompatible, MAX_EVALUATION_AGENTS } from "./evaluation-agent-filters";
import type { EvaluationWizardStep } from "./evaluation-wizard-steps";

function SummaryRow({ editLabel, label, onEdit, value }: { editLabel: string; label: string; onEdit: () => void; value: string }) {
  return (
    <div className="grid grid-cols-[minmax(6rem,0.35fr)_1fr_auto] items-start gap-2 text-xs">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="min-w-0 wrap-break-word font-medium text-foreground">{value}</dd>
      <button aria-label={editLabel} className="text-muted-foreground hover:text-foreground" onClick={onEdit} type="button">
        <Pencil aria-hidden="true" className="h-3.5 w-3.5" />
      </button>
    </div>
  );
}

/**
 * Step 3 / Review (18.4): the final restatement of Steps 1-2's own choices before Run actually
 * dispatches -- no field here is new, matching `CreateSessionStep4`'s own "nothing new to compute"
 * shape for its runtime/participant/workspace rows. Each row's own edit affordance calls
 * `onEditStep` (`useEvaluationWizardSteps`'s `goToStep`) to jump straight back to the step that
 * owns that field, rather than forcing a full Back/Back walk through the whole wizard. Re-surfaces
 * which selected Agents are flagged incompatible one more time here too: a reader who picked one
 * back in Step 2 and then changed the task in Step 1 should still see the same real reason at the
 * point they actually commit, not just during Step 2's own moment.
 */
export function EvaluationReviewStep({
  agentIds, agents, error, onEditStep, task,
}: {
  agentIds: string[];
  agents: AgentRegistryEntry[];
  error: string | null;
  onEditStep: (step: EvaluationWizardStep) => void;
  task: EvaluationTask | undefined;
}) {
  const { t } = useTranslation();
  const selectedAgents = agents.filter((agent) => agentIds.includes(agent.id));
  const flagged = selectedAgents.filter(isEvaluationAgentIncompatible);
  const overCapacity = agentIds.length > MAX_EVALUATION_AGENTS;
  const taskLabel = t("evaluation.task");
  const agentsLabel = t("evaluation.agents");

  return (
    <div className="grid gap-3">
      <h3 className="text-sm font-semibold">{t("evaluation.wizard.review")}</h3>
      <dl className="grid gap-2">
        <SummaryRow
          editLabel={t("evaluation.wizard.editField", { field: taskLabel })}
          label={taskLabel}
          onEdit={() => onEditStep(1)}
          value={task ? `${task.id} v${task.version} · ${task.category}` : t("evaluation.unavailable")}
        />
        <SummaryRow
          editLabel={t("evaluation.wizard.editField", { field: agentsLabel })}
          label={agentsLabel}
          onEdit={() => onEditStep(2)}
          value={t("evaluation.selectedCount", { count: selectedAgents.length })}
        />
      </dl>
      {selectedAgents.length > 0 ? (
        <ul className="grid gap-1 text-xs text-muted-foreground">
          {selectedAgents.map((agent) => <li key={agent.id}>{agent.displayName}</li>)}
        </ul>
      ) : null}
      {flagged.length > 0 ? (
        <div className="grid gap-1 rounded-md border border-[hsl(var(--warning))]/40 bg-[hsl(var(--warning))]/5 p-2.5">
          {flagged.map((agent) => (
            <p className="flex items-start gap-1.5 text-xs text-[hsl(var(--warning))]" key={agent.id}>
              <TriangleAlert aria-hidden="true" className="mt-0.5 h-3.5 w-3.5 shrink-0" />
              <span>{agent.displayName}: {agent.unavailableReason ?? t(`evaluation.agentStatus.${agent.availabilityState}`)}</span>
            </p>
          ))}
        </div>
      ) : null}
      {overCapacity ? <p className="text-xs text-destructive" role="alert">{t("evaluation.agentSelection.maxAgentsExceeded")}</p> : null}
      {error ? <p className="text-xs text-destructive" role="alert">{error}</p> : null}
    </div>
  );
}
