import { ChevronLeft, ChevronRight, Loader2, Play } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { cn } from "../lib/utils";
import type { AgentRegistryEntry } from "../types/agent";
import type { EvaluationTask } from "../types/evaluation";
import { Sheet } from "../ui/sheet/Sheet";
import { MAX_EVALUATION_AGENTS } from "./evaluation-agent-filters";
import { EvaluationAgentSelector } from "./evaluation-agent-selector";
import { EvaluationReviewStep } from "./evaluation-review-step";
import { EVALUATION_WIZARD_STEP_COUNT, useEvaluationWizardSteps, type EvaluationWizardStep } from "./evaluation-wizard-steps";

export interface EvaluationRunWizardProps {
  tasks: EvaluationTask[];
  agents: AgentRegistryEntry[];
  /** The page's own last-committed task/Agent selection (`evaluation-center.tsx`'s `taskId`/
   *  `agentIds`), used only to seed this wizard's own draft when it mounts -- never written to
   *  directly. See `evaluation-wizard-steps.ts`'s own doc comment: a fresh mount per open means a
   *  plain `useState` initializer here already reseeds from the latest committed values every time. */
  initialTaskId: string;
  initialAgentIds: string[];
  running: boolean;
  /** The page's own `error` (load/run/cancel share one slot) -- shown here too because the Sheet
   *  overlay sits above the page's own error banner while open; `EvaluationRunControls` clears it
   *  when the wizard opens so a stale, unrelated cancel-error can't bleed into a fresh attempt. */
  error: string | null;
  onClose: () => void;
  /** Only called from the Review step's own Run action -- receives the draft's final values
   *  directly rather than reading page state, so there is no stale-closure race with the page's own
   *  `setTaskId`/`setAgentIds` commit (`evaluation-center.tsx`'s `start`). */
  onRun: (taskId: string, agentIds: string[]) => void;
}

/**
 * 18.4: the guided wizard/Sheet-with-Review that replaces the old inline header controls --
 * task, then Agent selection (18.5's own `EvaluationAgentSelector`), then Review. Mounted the same
 * way Goal Center/Work Board mount their own create/edit Sheets (`placement="right"`), not
 * `CreateSessionDialogContent`'s dual Dialog/Sheet-by-breakpoint shape: this wizard lives inside an
 * already-routed page panel, not a top-level app-wide dialog, so one consistent Sheet mount is
 * enough.
 */
export function EvaluationRunWizard({
  agents, error, initialAgentIds, initialTaskId, onClose, onRun, running, tasks,
}: EvaluationRunWizardProps) {
  const { t } = useTranslation();
  const wizard = useEvaluationWizardSteps();
  const [taskId, setTaskId] = useState(initialTaskId);
  const [agentIds, setAgentIds] = useState(initialAgentIds);
  const activeTask = tasks.find((task) => task.id === taskId);
  const validAgentCount = agentIds.length > 0 && agentIds.length <= MAX_EVALUATION_AGENTS;
  const canRun = activeTask !== undefined && validAgentCount && !running;

  function canAdvance(step: EvaluationWizardStep): boolean {
    if (step === 1) return activeTask !== undefined;
    if (step === 2) return validAgentCount;
    return true;
  }

  function toggleAgent(agentId: string) {
    setAgentIds((current) => (current.includes(agentId) ? current.filter((id) => id !== agentId) : [...current, agentId]));
  }

  const footer = (
    <div className="flex items-center justify-between gap-3">
      {wizard.isFirstStep ? (
        <Button disabled={running} onClick={onClose} type="button" variant="outline">{t("evaluation.wizard.cancel")}</Button>
      ) : (
        <Button disabled={running} onClick={wizard.goBack} type="button" variant="outline">
          <ChevronLeft aria-hidden="true" className="h-3.5 w-3.5" />{t("evaluation.wizard.back")}
        </Button>
      )}
      {wizard.isLastStep ? (
        <Button data-testid="evaluation-run" disabled={!canRun} onClick={() => onRun(taskId, agentIds)} type="button">
          {running ? <Loader2 aria-hidden="true" className="h-3.5 w-3.5 animate-spin" /> : <Play aria-hidden="true" className="h-3.5 w-3.5" />}
          {running ? t("evaluation.running") : t("evaluation.run")}
        </Button>
      ) : (
        <Button disabled={!canAdvance(wizard.step)} onClick={wizard.goNext} type="button">
          {t("evaluation.wizard.next")}<ChevronRight aria-hidden="true" className="h-3.5 w-3.5" />
        </Button>
      )}
    </div>
  );

  return (
    <Sheet
      closeDisabled={running}
      description={t("evaluation.wizard.step", { current: wizard.step, total: EVALUATION_WIZARD_STEP_COUNT })}
      footer={footer}
      onClose={onClose}
      placement="right"
      title={t("evaluation.configure")}
      widthClassName="w-full sm:w-[30rem]"
    >
      <div className="grid gap-4">
        {wizard.step === 1 ? (
          <div className="grid gap-2">
            <h3 className="text-sm font-semibold">{t("evaluation.task")}</h3>
            <ul aria-label={t("evaluation.task")} className="grid gap-2">
              {tasks.map((task) => (
                <li key={`${task.id}-v${task.version}`}>
                  <button
                    aria-pressed={task.id === taskId}
                    className={cn(
                      "w-full rounded-md border border-border p-2.5 text-left text-sm transition-colors hover:bg-muted/40",
                      task.id === taskId && "border-primary bg-[hsl(var(--nav-active-soft))] shadow-[0_0_0_1px_hsl(var(--primary))]",
                    )}
                    data-testid={`evaluation-task-${task.id}`}
                    onClick={() => setTaskId(task.id)}
                    type="button"
                  >
                    <span className="flex items-center justify-between gap-2">
                      <span className="font-medium">{task.id} v{task.version}</span>
                      <span className="rounded bg-muted px-1.5 py-0.5 text-[0.6875rem] text-muted-foreground">{task.category}</span>
                    </span>
                    <span className="mt-1 block truncate text-xs text-muted-foreground">{task.prompt}</span>
                  </button>
                </li>
              ))}
            </ul>
          </div>
        ) : null}
        {wizard.step === 2 ? (
          <EvaluationAgentSelector agents={agents} onSelectVisible={setAgentIds} onToggle={toggleAgent} selectedIds={agentIds} />
        ) : null}
        {wizard.step === 3 ? <EvaluationReviewStep agentIds={agentIds} agents={agents} error={error} onEditStep={wizard.goToStep} task={activeTask} /> : null}
      </div>
    </Sheet>
  );
}
