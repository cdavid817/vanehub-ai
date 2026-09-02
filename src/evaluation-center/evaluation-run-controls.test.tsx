// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import type { AgentRegistryEntry } from "../types/agent";
import type { EvaluationTask } from "../types/evaluation";
import { EvaluationRunControls } from "./evaluation-run-controls";

function buildTask(id: string, version = 1): EvaluationTask {
  return { id, version, category: "bugfix", prompt: "fix it", timeoutSeconds: 600, verifierProfiles: ["default"] };
}

function buildAgent(id: string, displayName: string): AgentRegistryEntry {
  return {
    id, displayName, provider: id, launch: { kind: "cli" }, supportedInteractionModes: ["cli"],
    availabilityState: "available", capabilityTags: [], agentOrigin: "builtin",
  };
}

const TASK_A = buildTask("fix-null-auth-token");
const TASK_B = buildTask("add-retry-policy", 2);
const AGENT_A = buildAgent("claude-code", "Claude Code");
const AGENT_B = buildAgent("codex-cli", "Codex CLI");

describe("EvaluationRunControls", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  it("renders every task as a select option and reports a change without owning taskId itself", () => {
    const onTaskIdChange = vi.fn();
    render(
      <EvaluationRunControls
        agentIds={[]}
        agents={[]}
        disabled={false}
        onRun={vi.fn()}
        onTaskIdChange={onTaskIdChange}
        onToggleAgent={vi.fn()}
        running={false}
        taskId={TASK_A.id}
        tasks={[TASK_A, TASK_B]}
      />,
    );
    const select = screen.getByTestId("evaluation-task") as HTMLSelectElement;
    expect(select.value).toBe(TASK_A.id);
    expect(Array.from(select.options).map((option) => option.textContent)).toEqual([
      `${TASK_A.id} v${TASK_A.version}`, `${TASK_B.id} v${TASK_B.version}`,
    ]);
    fireEvent.change(select, { target: { value: TASK_B.id } });
    expect(onTaskIdChange).toHaveBeenCalledWith(TASK_B.id);
  });

  it("renders one checkbox per Agent reflecting the given selection and reports toggles by id", () => {
    const onToggleAgent = vi.fn();
    render(
      <EvaluationRunControls
        agentIds={[AGENT_A.id]}
        agents={[AGENT_A, AGENT_B]}
        disabled={false}
        onRun={vi.fn()}
        onTaskIdChange={vi.fn()}
        onToggleAgent={onToggleAgent}
        running={false}
        taskId={TASK_A.id}
        tasks={[TASK_A]}
      />,
    );
    const checkboxA = screen.getByTestId(`evaluation-agent-${AGENT_A.id}`) as HTMLInputElement;
    const checkboxB = screen.getByTestId(`evaluation-agent-${AGENT_B.id}`) as HTMLInputElement;
    expect(checkboxA.checked).toBe(true);
    expect(checkboxB.checked).toBe(false);
    expect(screen.getByText(AGENT_A.displayName)).toBeTruthy();
    fireEvent.click(checkboxB);
    expect(onToggleAgent).toHaveBeenCalledWith(AGENT_B.id);
  });

  it("disables the run button exactly as told and fires onRun on click", () => {
    const onRun = vi.fn();
    const { rerender } = render(
      <EvaluationRunControls
        agentIds={[AGENT_A.id]}
        agents={[AGENT_A]}
        disabled
        onRun={onRun}
        onTaskIdChange={vi.fn()}
        onToggleAgent={vi.fn()}
        running={false}
        taskId={TASK_A.id}
        tasks={[TASK_A]}
      />,
    );
    const button = screen.getByTestId("evaluation-run") as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    rerender(
      <EvaluationRunControls
        agentIds={[AGENT_A.id]}
        agents={[AGENT_A]}
        disabled={false}
        onRun={onRun}
        onTaskIdChange={vi.fn()}
        onToggleAgent={vi.fn()}
        running={false}
        taskId={TASK_A.id}
        tasks={[TASK_A]}
      />,
    );
    expect(button.disabled).toBe(false);
    fireEvent.click(button);
    expect(onRun).toHaveBeenCalledTimes(1);
  });

  it("shows the running label instead of the run label while running", () => {
    render(
      <EvaluationRunControls
        agentIds={[]}
        agents={[]}
        disabled
        onRun={vi.fn()}
        onTaskIdChange={vi.fn()}
        onToggleAgent={vi.fn()}
        running
        taskId=""
        tasks={[]}
      />,
    );
    expect(screen.getByTestId("evaluation-run").textContent).toContain("Running");
  });
});
