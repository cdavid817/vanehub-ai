import { beforeEach, describe, expect, it, vi } from "vitest";
import type { PlanDraft } from "../types/plan";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import { tauriPlanClient } from "./tauri-plan-client";

const draft = {
  id: "plan-1", versionId: "version-1", version: 1, goal: "Ship", projectPath: "D:/app",
  baseRef: "main", plannerProfileId: null, subtasks: [], dependencies: [],
  discovery: { status: "not_started", limitations: [] },
  executionPolicy: { maxAttemptsPerSubtask: 3, repairEligibleClasses: ["verification_failed"], finalValidationCommands: [] },
} satisfies PlanDraft;

describe("Tauri Plan adapter", () => {
  beforeEach(() => invokeMock.mockReset().mockResolvedValue({}));

  it("maps the complete Plan service contract to declared commands", async () => {
    const generation = { planId: null, version: 1, goal: "Ship", projectPath: "D:/app", baseRef: "main", availableTools: [] };
    await tauriPlanClient.generatePlanDraft(generation);
    await tauriPlanClient.validatePlanDraft(draft);
    await tauriPlanClient.savePlanDraft(draft);
    await tauriPlanClient.getPlanDraft("plan-1");
    await tauriPlanClient.listPlanVersions("plan-1");
    await tauriPlanClient.deletePlanDraft("draft-plan");
    await tauriPlanClient.approvePlan("plan-1");
    await tauriPlanClient.startPlanRun("run-1");
    await tauriPlanClient.executeNextAttempt("run-1");
    await tauriPlanClient.listPlanRuns("cursor-1");
    await tauriPlanClient.getPlanRun("run-1");
    await tauriPlanClient.getPlanAttemptEvidence("attempt-1");
    await tauriPlanClient.requestPlanControl("run-1", "pause");
    await tauriPlanClient.retryPlanSubtask("run-1", "task-run-1");
    await tauriPlanClient.recoverPlanRun("run-1");
    await tauriPlanClient.acceptPlanRun("run-1");

    expect(invokeMock.mock.calls).toEqual([
      ["generate_plan_draft", { input: generation }], ["validate_plan_draft", { input: draft }],
      ["save_plan_draft", { input: draft }], ["get_plan_draft", { planId: "plan-1" }],
      ["list_plan_versions", { planId: "plan-1" }], ["delete_plan_draft", { planId: "draft-plan" }],
      ["approve_plan", { planId: "plan-1", originatingSessionId: null }],
      ["start_plan_run", { runId: "run-1" }], ["execute_next_plan_attempt", { runId: "run-1" }],
      ["list_plan_runs", { cursor: "cursor-1" }],
      ["get_plan_run_detail", { runId: "run-1" }],
      ["get_plan_attempt_evidence", { attemptId: "attempt-1" }],
      ["request_plan_control", { runId: "run-1", kind: "pause" }],
      ["retry_plan_subtask", { runId: "run-1", subtaskRunId: "task-run-1" }],
      ["recover_plan_run", { runId: "run-1" }], ["accept_plan_run", { runId: "run-1" }],
    ]);
  });
});
