import { Settings2 } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { AgentRegistryEntry } from "../types/agent";
import type { EvaluationTask } from "../types/evaluation";
import { EvaluationRunWizard } from "./evaluation-run-wizard";

export interface EvaluationRunControlsProps {
  tasks: EvaluationTask[];
  agents: AgentRegistryEntry[];
  /** The page's own last-committed selection -- read-only from here, only ever written back by
   *  the page itself once `onRun` succeeds. See `EvaluationRunWizard`'s own doc comment. */
  taskId: string;
  agentIds: string[];
  running: boolean;
  error: string | null;
  /** Fires the moment the wizard opens, before any draft state exists -- lets the page clear a
   *  stale `error` left over from an unrelated prior cancel/load failure so it can't bleed into a
   *  fresh configuration attempt (see `EvaluationRunWizard`'s own `error` doc comment). */
  onOpen: () => void;
  /** Resolves to whether the run actually started. `EvaluationRunControls` only closes the wizard
   *  on `true`, so a failed attempt leaves the draft (and the error) exactly where the reader can
   *  still see and retry it. */
  onRun: (taskId: string, agentIds: string[]) => Promise<boolean>;
}

/**
 * 18.4/18.5: the header's own entry point into the guided wizard/Sheet-with-Review
 * (`EvaluationRunWizard`) -- task and Agent configuration no longer live inline here. State
 * ownership split: this component and the page above it only ever hold the *committed* selection
 * (`taskId`/`agentIds`, unchanged until a run actually starts); the wizard owns its own draft of
 * that same selection while it is open, exactly like `GoalForm`'s Sheet-mounted draft never
 * touching `GoalCenter`'s real state until submit.
 */
export function EvaluationRunControls({
  agentIds, agents, error, onOpen, onRun, running, taskId, tasks,
}: EvaluationRunControlsProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  async function handleRun(nextTaskId: string, nextAgentIds: string[]) {
    const succeeded = await onRun(nextTaskId, nextAgentIds);
    if (succeeded) setOpen(false);
  }

  return (
    <>
      <button
        className="ucd-button-primary flex h-9 items-center gap-2 rounded-md px-3 text-sm disabled:cursor-not-allowed disabled:opacity-50"
        data-testid="evaluation-configure"
        disabled={running || tasks.length === 0}
        onClick={() => { onOpen(); setOpen(true); }}
        type="button"
      >
        <Settings2 aria-hidden="true" className="h-4 w-4" />
        {t("evaluation.configure")}
      </button>
      {open ? (
        <EvaluationRunWizard
          agents={agents}
          error={error}
          initialAgentIds={agentIds}
          initialTaskId={taskId}
          onClose={() => setOpen(false)}
          onRun={(nextTaskId, nextAgentIds) => { void handleRun(nextTaskId, nextAgentIds); }}
          running={running}
          tasks={tasks}
        />
      ) : null}
    </>
  );
}
