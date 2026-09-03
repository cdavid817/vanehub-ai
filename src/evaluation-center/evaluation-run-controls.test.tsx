// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
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
const AGENT_A = buildAgent("claude-code", "Claude Code");

describe("EvaluationRunControls", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  afterEach(() => { cleanup(); });

  it("renders a closed trigger and opens the wizard, notifying onOpen, when clicked", () => {
    const onOpen = vi.fn();
    render(
      <EvaluationRunControls
        agentIds={[AGENT_A.id]}
        agents={[AGENT_A]}
        error={null}
        onOpen={onOpen}
        onRun={vi.fn()}
        running={false}
        taskId={TASK_A.id}
        tasks={[TASK_A]}
      />,
    );
    expect(screen.queryByTestId(`evaluation-task-${TASK_A.id}`)).toBeNull();
    fireEvent.click(screen.getByTestId("evaluation-configure"));
    expect(onOpen).toHaveBeenCalledTimes(1);
    expect(screen.getByTestId(`evaluation-task-${TASK_A.id}`)).toBeTruthy();
  });

  it("disables the trigger while no tasks are loaded, and while a run is already in flight", () => {
    const { rerender } = render(
      <EvaluationRunControls agentIds={[]} agents={[]} error={null} onOpen={vi.fn()} onRun={vi.fn()} running={false} taskId="" tasks={[]} />,
    );
    expect((screen.getByTestId("evaluation-configure") as HTMLButtonElement).disabled).toBe(true);
    rerender(
      <EvaluationRunControls
        agentIds={[AGENT_A.id]}
        agents={[AGENT_A]}
        error={null}
        onOpen={vi.fn()}
        onRun={vi.fn()}
        running
        taskId={TASK_A.id}
        tasks={[TASK_A]}
      />,
    );
    expect((screen.getByTestId("evaluation-configure") as HTMLButtonElement).disabled).toBe(true);
  });

  it("closes the wizard once onRun resolves true, delegating the wizard's own committed draft verbatim", async () => {
    const onRun = vi.fn().mockResolvedValue(true);
    render(
      <EvaluationRunControls
        agentIds={[AGENT_A.id]}
        agents={[AGENT_A]}
        error={null}
        onOpen={vi.fn()}
        onRun={onRun}
        running={false}
        taskId={TASK_A.id}
        tasks={[TASK_A]}
      />,
    );
    fireEvent.click(screen.getByTestId("evaluation-configure"));
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    fireEvent.click(screen.getByTestId("evaluation-run"));
    expect(onRun).toHaveBeenCalledWith(TASK_A.id, [AGENT_A.id]);
    await waitFor(() => expect(screen.queryByTestId(`evaluation-task-${TASK_A.id}`)).toBeNull());
  });

  it("keeps the wizard open, with its draft and Review step intact, once onRun resolves false", async () => {
    const onRun = vi.fn().mockResolvedValue(false);
    render(
      <EvaluationRunControls
        agentIds={[AGENT_A.id]}
        agents={[AGENT_A]}
        error={null}
        onOpen={vi.fn()}
        onRun={onRun}
        running={false}
        taskId={TASK_A.id}
        tasks={[TASK_A]}
      />,
    );
    fireEvent.click(screen.getByTestId("evaluation-configure"));
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    fireEvent.click(screen.getByTestId("evaluation-run"));
    await waitFor(() => expect(onRun).toHaveBeenCalledTimes(1));
    expect(screen.getByText("Review")).toBeTruthy();
  });
});
