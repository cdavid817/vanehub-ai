// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { ComponentProps } from "react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import type { AgentRegistryEntry } from "../types/agent";
import type { EvaluationTask } from "../types/evaluation";
import { EvaluationRunWizard } from "./evaluation-run-wizard";

function buildTask(id: string, version = 1, category: EvaluationTask["category"] = "bugfix"): EvaluationTask {
  return { id, version, category, prompt: `Prompt for ${id}`, timeoutSeconds: 600, verifierProfiles: ["default"] };
}

function buildAgent(id: string, displayName: string, overrides: Partial<AgentRegistryEntry> = {}): AgentRegistryEntry {
  return {
    id, displayName, provider: id, launch: { kind: "cli" }, supportedInteractionModes: ["cli"],
    availabilityState: "available", capabilityTags: [], agentOrigin: "builtin",
    ...overrides,
  };
}

const TASK_A = buildTask("fix-null-auth-token");
const TASK_B = buildTask("add-retry-policy", 2, "feature");
const AGENT_A = buildAgent("claude-code", "Claude Code");
const AGENT_B = buildAgent("codex-cli", "Codex CLI");

function renderWizard(overrides: Partial<ComponentProps<typeof EvaluationRunWizard>> = {}) {
  const onClose = vi.fn();
  const onRun = vi.fn();
  render(
    <EvaluationRunWizard
      agents={[AGENT_A, AGENT_B]}
      error={null}
      initialAgentIds={[AGENT_A.id]}
      initialTaskId={TASK_A.id}
      onClose={onClose}
      onRun={onRun}
      running={false}
      tasks={[TASK_A, TASK_B]}
      {...overrides}
    />,
  );
  return { onClose, onRun };
}

describe("EvaluationRunWizard", () => {
  beforeAll(async () => { await activateAppLanguage("en"); });
  afterEach(() => { cleanup(); });

  it("opens on the task step with the seeded task selected, and Next enabled", () => {
    renderWizard();
    expect(screen.getByTestId(`evaluation-task-${TASK_A.id}`).getAttribute("aria-pressed")).toBe("true");
    expect((screen.getByRole("button", { name: "Next" }) as HTMLButtonElement).disabled).toBe(false);
  });

  it("does not call onRun before Review's own Run action is clicked", () => {
    const { onRun } = renderWizard();
    fireEvent.click(screen.getByTestId(`evaluation-task-${TASK_B.id}`));
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(screen.getByText("Review")).toBeTruthy();
    expect(onRun).not.toHaveBeenCalled();
  });

  it("blocks Next on the Agent step until at least one Agent is selected", () => {
    renderWizard({ initialAgentIds: [] });
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect((screen.getByRole("button", { name: "Next" }) as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByTestId(`evaluation-agent-${AGENT_A.id}`));
    expect((screen.getByRole("button", { name: "Next" }) as HTMLButtonElement).disabled).toBe(false);
  });

  it("reaches Review with the chosen task and Agents reflected, then commits the wizard's own draft values on Run", () => {
    const { onRun } = renderWizard();
    fireEvent.click(screen.getByTestId(`evaluation-task-${TASK_B.id}`));
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    fireEvent.click(screen.getByTestId(`evaluation-agent-${AGENT_B.id}`));
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(screen.getByText(`${TASK_B.id} v${TASK_B.version} · ${TASK_B.category}`)).toBeTruthy();
    fireEvent.click(screen.getByTestId("evaluation-run"));
    expect(onRun).toHaveBeenCalledWith(TASK_B.id, [AGENT_A.id, AGENT_B.id]);
  });

  it("navigates back from Review to the Agent step without losing the draft", () => {
    renderWizard();
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(screen.getByText("Review")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    expect((screen.getByTestId(`evaluation-agent-${AGENT_A.id}`) as HTMLInputElement).checked).toBe(true);
  });

  it("jumps from Review straight back to the step that owns the clicked field", () => {
    renderWizard();
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(screen.getByText("Review")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Edit Agents" }));
    expect(screen.getByTestId(`evaluation-agent-${AGENT_A.id}`)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    fireEvent.click(screen.getByRole("button", { name: "Edit Benchmark task" }));
    expect(screen.getByTestId(`evaluation-task-${TASK_A.id}`)).toBeTruthy();
  });

  it("closes via Cancel on the first step", () => {
    const { onClose } = renderWizard();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onClose).toHaveBeenCalled();
  });

  it("shows the page-level error inside Review while the wizard stays open", () => {
    renderWizard({ error: "Could not start the benchmark." });
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(screen.getByRole("alert").textContent).toBe("Could not start the benchmark.");
  });

  it("disables the footer's close/back action while a run is in flight", () => {
    renderWizard({ running: true });
    expect((screen.getByRole("button", { name: "Cancel" }) as HTMLButtonElement).disabled).toBe(true);
  });
});
