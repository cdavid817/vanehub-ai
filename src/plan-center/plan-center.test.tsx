// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { PlanDraft, PlanRunDetail } from "../types/plan";
import { activateAppLanguage } from "../i18n";

const { service } = vi.hoisted(() => ({ service: {
  generatePlanDraft: vi.fn(), savePlanDraft: vi.fn(), approvePlan: vi.fn(), startPlanRun: vi.fn(),
  getPlanRun: vi.fn(), executeNextAttempt: vi.fn(), requestPlanControl: vi.fn(), retryPlanSubtask: vi.fn(),
  recoverPlanRun: vi.fn(), acceptPlanRun: vi.fn(), getPlanAttemptEvidence: vi.fn(),
} }));
vi.mock("../services/runtime-plan-client", () => ({ planService: service }));

import { PlanCenter } from "./plan-center";
import { PlanRunView } from "./plan-run-view";

const draft: PlanDraft = {
  id: "plan-1", versionId: "version-1", version: 1, goal: "Ship", projectPath: "D:/app",
  baseRef: "main", plannerProfileId: "profile-1", dependencies: [], subtasks: [{
    id: "task-1", title: "Implement", description: "Implement safely", acceptanceCriteria: ["Tests pass"],
    ordinal: 0, assignedRole: "worker", limits: { tokenBudget: 1000, toolCallLimit: 10, timeoutSeconds: 60 },
    validationCommands: [],
  }],
};

const run: PlanRunDetail = {
  id: "run-1", planId: "plan-1", status: "awaiting_acceptance", completedTasks: 1, totalTasks: 1,
  simulated: false, createdAt: "2026-08-08T00:00:00Z", updatedAt: "2026-08-08T00:01:00Z",
  projectPath: "D:/app", baseRef: "main", baseOid: "abc", worktreePath: "D:/app-plan",
  worktreeName: "plan", worktreeBranch: "vanehub/plan", availableControls: ["accept"], tasks: [{
    id: "task-run-1", subtaskId: "task-1", title: "Implement", status: "succeeded", topologicalRank: 0,
    ordinal: 0, resultSummary: "Done", changedFiles: ["src/app.ts"], verificationSummary: "Tests passed", attempts: [],
  }],
};

describe("Plan Center", () => {
  beforeEach(async () => { await activateAppLanguage("en"); vi.clearAllMocks(); service.generatePlanDraft.mockResolvedValue(structuredClone(draft)); });

  it("collects a goal and enters editable human review after OnePiece generation", async () => {
    render(<PlanCenter />);
    fireEvent.change(screen.getByRole("textbox", { name: "Goal" }), { target: { value: "Ship safely" } });
    fireEvent.change(screen.getByRole("textbox", { name: "Project path" }), { target: { value: "D:/app" } });
    fireEvent.click(screen.getByRole("button", { name: "Generate Plan" }));
    expect(await screen.findByText("Review task graph")).toBeTruthy();
    expect(service.generatePlanDraft).toHaveBeenCalledWith(expect.objectContaining({ goal: "Ship safely", projectPath: "D:/app" }));
  });

  it.each(["OnePiece profile is not ready", "Planner output is invalid"])("presents actionable generation failure: %s", async (message) => {
    service.generatePlanDraft.mockRejectedValueOnce(new Error(message));
    render(<PlanCenter />);
    fireEvent.change(screen.getByRole("textbox", { name: "Goal" }), { target: { value: "Ship safely" } });
    fireEvent.change(screen.getByRole("textbox", { name: "Project path" }), { target: { value: "D:/app" } });
    fireEvent.click(screen.getByRole("button", { name: "Generate Plan" }));
    expect((await screen.findByRole("alert")).textContent).toContain(message);
  });

  it("edits a generated task and blocks approval with complete-draft feedback", async () => {
    render(<PlanCenter />);
    fireEvent.change(screen.getByRole("textbox", { name: "Goal" }), { target: { value: "Ship safely" } });
    fireEvent.change(screen.getByRole("textbox", { name: "Project path" }), { target: { value: "D:/app" } });
    fireEvent.click(screen.getByRole("button", { name: "Generate Plan" }));
    const title = await screen.findByRole("textbox", { name: "Task 1 title" });
    fireEvent.change(title, { target: { value: "Implement safely" } });
    expect((title as HTMLInputElement).value).toBe("Implement safely");
    fireEvent.change(screen.getByLabelText("Acceptance criteria (one per line, up to three)"), { target: { value: "" } });
    fireEvent.click(screen.getByRole("button", { name: "Approve and start" }));
    expect((await screen.findByRole("alert")).textContent).toContain("one to three acceptance criteria");
    expect(service.savePlanDraft).not.toHaveBeenCalled();
  });

  it("shows verified progress, explicit acceptance, and retained-worktree safety copy", async () => {
    const accept = vi.fn();
    render(<PlanRunView busy={false} onAccept={accept} onControl={vi.fn()} onExecute={vi.fn()} onInspectEvidence={service.getPlanAttemptEvidence} onRetry={vi.fn()} run={run} />);
    expect(screen.getByText("D:/app-plan")).toBeTruthy();
    expect(screen.getByText(/does not automatically commit/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Accept result" }));
    await waitFor(() => expect(accept).toHaveBeenCalledOnce());
  });

  it("routes controls and loads bounded attempt evidence only after explicit inspection", async () => {
    const execute = vi.fn();
    const control = vi.fn();
    const retry = vi.fn();
    const active: PlanRunDetail = {
      ...run,
      status: "running",
      availableControls: ["pause", "cancel"],
      tasks: [{
        ...run.tasks[0]!, status: "failed", attempts: [{
          id: "attempt-1", sequence: 1, status: "failed", sessionId: "session-1", profileId: "profile-1",
          executionRunId: null, operationId: "operation-1", tokenUsage: 42, toolCallCount: 3,
          errorClass: "verification_failed", startedAt: "2026-08-08T00:00:00Z", completedAt: "2026-08-08T00:00:10Z",
        }],
      }],
    };
    service.getPlanAttemptEvidence.mockResolvedValue([{ id: "evidence-1", commandId: "tests", status: "failed", exitCode: 1, durationMs: 25, outputSummary: "bounded failure output", createdAt: "2026-08-08T00:00:10Z" }]);
    const view = render(<PlanRunView busy={false} onAccept={vi.fn()} onControl={control} onExecute={execute} onInspectEvidence={service.getPlanAttemptEvidence} onRetry={retry} run={active} />);
    expect(service.getPlanAttemptEvidence).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Execute next task" }));
    fireEvent.click(screen.getByRole("button", { name: "Pause" }));
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    fireEvent.click(screen.getByText(/Attempt 1/));
    expect(await screen.findByText("bounded failure output")).toBeTruthy();
    expect(service.getPlanAttemptEvidence).toHaveBeenCalledWith("attempt-1");
    view.rerender(<PlanRunView busy={false} onAccept={vi.fn()} onControl={control} onExecute={execute} onInspectEvidence={service.getPlanAttemptEvidence} onRetry={retry} run={{ ...active, status: "recovery_required", availableControls: ["recover"] }} />);
    fireEvent.click(screen.getByRole("button", { name: "Recover" }));
    view.rerender(<PlanRunView busy={false} onAccept={vi.fn()} onControl={control} onExecute={execute} onInspectEvidence={service.getPlanAttemptEvidence} onRetry={retry} run={{ ...active, status: "paused", availableControls: ["resume"] }} />);
    fireEvent.click(screen.getByRole("button", { name: "Resume" }));
    expect(execute).toHaveBeenCalledOnce();
    expect(control.mock.calls).toEqual([["pause"], ["cancel"], ["recover"], ["resume"]]);
    expect(retry).toHaveBeenCalledWith("task-run-1");
    expect(screen.getByText(/session-1/)).toBeTruthy();
  });
});
