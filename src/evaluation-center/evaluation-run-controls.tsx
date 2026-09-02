import { Play } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { AgentRegistryEntry } from "../types/agent";
import type { EvaluationTask } from "../types/evaluation";

export interface EvaluationRunControlsProps {
  tasks: EvaluationTask[];
  taskId: string;
  onTaskIdChange: (taskId: string) => void;
  agents: AgentRegistryEntry[];
  agentIds: string[];
  onToggleAgent: (agentId: string) => void;
  running: boolean;
  disabled: boolean;
  onRun: () => void;
}

/**
 * 18.2 structural extraction only: the task `<select>` and Agent `<fieldset>` of checkboxes that
 * used to sit directly in `evaluation-center.tsx`'s own `<header>`, moved verbatim here -- same
 * test ids, same classNames, same interaction. State ownership does not move: the page still holds
 * `taskId`/`agentIds`/`running`, and this component is a controlled, presentation-only view over
 * them.
 *
 * This is NOT the guided wizard/Sheet-with-Review 18.4 asks for, and NOT the searchable
 * status/capability-filtered Agent selector with select-visible, selected summary, and
 * incompatibility reasons 18.5 asks for -- both remain real, separate, unstarted feature work.
 * This pass only gives the page's `<header>` room to grow into that later without also being the
 * page's own task/Agent markup.
 */
export function EvaluationRunControls({
  tasks, taskId, onTaskIdChange, agents, agentIds, onToggleAgent, running, disabled, onRun,
}: EvaluationRunControlsProps) {
  const { t } = useTranslation();
  return (
    <>
      <select aria-label={t("evaluation.task")} className="h-9 rounded-md border border-input bg-background px-2 text-sm" data-testid="evaluation-task" onChange={(event) => onTaskIdChange(event.target.value)} value={taskId}>
        {tasks.map((task) => <option key={task.id} value={task.id}>{task.id} v{task.version}</option>)}
      </select>
      <fieldset className="flex min-h-9 flex-wrap items-center gap-2 rounded-md border border-input px-2">
        <legend className="sr-only">{t("evaluation.agents")}</legend>
        {agents.map((agent) => (
          <label className="flex items-center gap-1 text-xs" key={agent.id}>
            <input checked={agentIds.includes(agent.id)} data-testid={`evaluation-agent-${agent.id}`} onChange={() => onToggleAgent(agent.id)} type="checkbox" />
            {agent.displayName}
          </label>
        ))}
      </fieldset>
      <button className="ucd-button-primary flex h-9 items-center gap-2 rounded-md px-3 text-sm" data-testid="evaluation-run" disabled={disabled} onClick={onRun} type="button">
        <Play aria-hidden="true" className="h-4 w-4" />
        {running ? t("evaluation.running") : t("evaluation.run")}
      </button>
    </>
  );
}
