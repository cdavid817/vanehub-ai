import { beforeEach, describe, expect, it, vi } from "vitest";
import type { PlanDraft, PlanRunDetail, PlanRunStatus } from "../types/plan";
import type { PlanService } from "./plan-service";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import { tauriPlanClient } from "./tauri-plan-client";
import { projectLatestWebPlanRunForTest, webPlanClient } from "./web-plan-client";

let setNativeProjectionStatus: (status: PlanRunStatus) => void = () => undefined;

const methodNames = [
  "generatePlanDraft", "validatePlanDraft", "savePlanDraft", "getPlanDraft", "listPlanVersions",
  "deletePlanDraft", "approvePlan", "startPlanRun", "executeNextAttempt", "listPlanRuns",
  "getPlanRun", "getPlanAttemptEvidence", "requestPlanControl",
  "getPlanRunForSession",
  "retryPlanSubtask", "recoverPlanRun", "acceptPlanRun",
] satisfies Array<keyof PlanService>;

function nativeDraft(): PlanDraft {
  return {
    id: "native-plan", versionId: "native-version", version: 1, goal: "Ship", projectPath: "D:/app",
    baseRef: "main", plannerProfileId: "profile-1",
    discovery: { status: "complete", limitations: [] },
    executionPolicy: { maxAttemptsPerSubtask: 3, repairEligibleClasses: ["verification_failed"], finalValidationCommands: [{ id: "final", program: "npm", args: ["test"], workingDirectory: null, timeoutSeconds: 60, required: true }] },
    subtasks: [
      { id: "a", title: "Analyze", description: "Analyze", acceptanceCriteria: ["Analyzed"], criterionEvidence: [{ criterionIndex: 0, kind: "automated", commandId: "test-a" }], ordinal: 0, assignedRole: "worker", limits: { tokenBudget: 1000, toolCallLimit: 10, timeoutSeconds: 60 }, validationCommands: [{ id: "test-a", program: "npm", args: ["test"], workingDirectory: null, timeoutSeconds: 60, required: true }] },
      { id: "b", title: "Build", description: "Build", acceptanceCriteria: ["Built"], criterionEvidence: [{ criterionIndex: 0, kind: "automated", commandId: "test-b" }], ordinal: 1, assignedRole: "worker", limits: { tokenBudget: 1000, toolCallLimit: 10, timeoutSeconds: 60 }, validationCommands: [{ id: "test-b", program: "npm", args: ["test"], workingDirectory: null, timeoutSeconds: 60, required: true }] },
    ], dependencies: [{ predecessorId: "a", successorId: "b" }],
  };
}

function setupNativeAdapter(): void {
  let draft = nativeDraft();
  let status: PlanRunStatus = "running";
  setNativeProjectionStatus = (next) => { status = next; };
  const deletedPlans = new Set<string>();
  const detail = (): PlanRunDetail => ({
    id: "native-run", planId: draft.id, status, completedTasks: status === "awaiting_acceptance" || status === "completed" ? 2 : 0,
    totalTasks: 2, simulated: false, createdAt: "2026-08-08T00:00:00Z", updatedAt: "2026-08-08T00:00:01Z",
    projectPath: draft.projectPath, baseRef: draft.baseRef, baseOid: "abc", worktreePath: "D:/app-plan",
    worktreeName: "plan", worktreeBranch: "vanehub/plan", originatingSessionId: "origin-session", availableControls: status === "running" ? ["pause", "cancel"] : status === "paused" ? ["resume", "cancel"] : status === "awaiting_acceptance" ? ["accept"] : [],
    finalization: null,
    tasks: draft.subtasks.map((task) => ({ id: `run-${task.id}`, subtaskId: task.id, title: task.title, status: status === "awaiting_acceptance" || status === "completed" ? "succeeded" : "pending", topologicalRank: task.ordinal, ordinal: task.ordinal, resultSummary: null, changedFiles: [], verificationSummary: null, attempts: [] })),
  });
  invokeMock.mockReset().mockImplementation(async (command: string, args: Record<string, unknown>) => {
    if (command === "generate_plan_draft") {
      const input = args.input as { goal: string };
      return input.goal === "Delete" ? { ...structuredClone(draft), id: "native-disposable", versionId: "native-disposable-v1" } : structuredClone(draft);
    }
    if (command === "validate_plan_draft") {
      const candidate = args.input as PlanDraft;
      if (candidate.dependencies.length > 1) throw new Error("validation error: dependency cycle");
      return undefined;
    }
    if (command === "save_plan_draft") {
      const candidate = args.input as PlanDraft;
      if (candidate.dependencies.length > 1) throw new Error("dependency cycle");
      draft = structuredClone(candidate);
      return structuredClone(draft);
    }
    if (command === "get_plan_draft") return deletedPlans.has(args.planId as string) ? null : structuredClone(draft);
    if (command === "list_plan_versions") return [structuredClone(draft)];
    if (command === "delete_plan_draft") { deletedPlans.add(args.planId as string); return undefined; }
    if (command === "approve_plan") return { runId: "native-run", summary: { projectPath: draft.projectPath, taskCount: draft.subtasks.length, retainedWorktree: true, automaticGitOperations: false } };
    if (command === "get_plan_run_for_session") return args.sessionId === "origin-session" ? { id: "native-run", planId: draft.id, status, completedTasks: 0, totalTasks: 2, simulated: false, createdAt: "2026-08-08T00:00:00Z", updatedAt: "2026-08-08T00:00:01Z" } : null;
    if (command === "start_plan_run") return { runId: "native-run", status: "running", projectPath: draft.projectPath, baseOid: "abc", worktreePath: "D:/app-plan", worktreeName: "plan", worktreeBranch: "vanehub/plan" };
    if (command === "list_plan_runs") return { items: [{ id: "native-run", planId: draft.id, status, completedTasks: 0, totalTasks: 2, simulated: false, createdAt: "2026-08-08T00:00:00Z", updatedAt: "2026-08-08T00:00:01Z" }], nextCursor: null };
    if (command === "get_plan_run_detail") return detail();
    if (command === "get_plan_attempt_evidence") return [];
    if (command === "request_plan_control") {
      const kind = args.kind;
      if (kind === "pause" && status === "running") status = "paused";
      else if (kind === "resume" && status === "paused") status = "running";
      else throw new Error("invalid control transition");
      return { requestId: `native-${kind}`, run: detail() };
    }
    if (command === "execute_next_plan_attempt") {
      status = "awaiting_acceptance";
      return { attemptId: "attempt-1", sessionId: "session-1", status: "succeeded", contextTruncated: false };
    }
    if (command === "accept_plan_run") {
      status = "completed";
      return { requestId: "native-accept", run: detail() };
    }
    throw new Error(`Unexpected command: ${command}`);
  });
}

const projectedStatuses: PlanRunStatus[] = [
  "running", "verifying", "repairing", "paused", "action_required",
  "final_verifying", "awaiting_acceptance", "completed",
];

async function expectProjectionConformance(
  service: PlanService,
  runId: string,
  project: (status: PlanRunStatus) => void,
): Promise<void> {
  for (const status of projectedStatuses) {
    project(status);
    const projection = await service.getPlanRun(runId);
    expect(projection.status).toBe(status);
    expect(projection.originatingSessionId).toBe("origin-session");
  }
}

async function expectConformance(service: PlanService, label: string): Promise<void> {
  for (const name of methodNames) expect(typeof service[name], name).toBe("function");
  const draft = await service.generatePlanDraft({ planId: null, version: 1, goal: `Ship ${label}`, projectPath: "D:/app", baseRef: "main", availableTools: ["shell"] });
  expect(draft.subtasks).toHaveLength(2);
  expect(draft.dependencies[0]).toEqual({ predecessorId: draft.subtasks[0]!.id, successorId: draft.subtasks[1]!.id });
  const invalid = structuredClone(draft);
  invalid.dependencies.push({ predecessorId: invalid.subtasks[1]!.id, successorId: invalid.subtasks[0]!.id });
  await expect(service.validatePlanDraft(invalid)).rejects.toThrow(/cycle|acyclic/i);
  await expect(service.savePlanDraft(invalid)).rejects.toThrow(/cycle|acyclic/i);
  await service.validatePlanDraft(draft);
  await service.savePlanDraft(draft);
  expect((await service.getPlanDraft(draft.id))?.versionId).toBe(draft.versionId);
  expect(await service.listPlanVersions(draft.id)).toEqual([draft]);
  const disposable = await service.generatePlanDraft({ planId: null, version: 1, goal: "Delete", projectPath: "D:/app", baseRef: "main", availableTools: [] });
  await service.deletePlanDraft(disposable.id);
  expect(await service.getPlanDraft(disposable.id)).toBeNull();
  const { runId, summary } = await service.approvePlan(draft.id, "origin-session");
  expect(summary).toEqual({ projectPath: "D:/app", taskCount: 2, retainedWorktree: true, automaticGitOperations: false });
  const prepared = await service.startPlanRun(runId);
  expect(prepared).toEqual(expect.objectContaining({ runId, status: "running", projectPath: "D:/app" }));
  const page = await service.listPlanRuns();
  expect(page.nextCursor).toBeNull();
  expect(page.items.some((item) => item.id === runId && typeof item.simulated === "boolean")).toBe(true);
  const running = await service.getPlanRun(runId);
  expect(running.originatingSessionId).toBe("origin-session");
  expect((await service.getPlanRunForSession("origin-session"))?.id).toBe(runId);
  expect(await service.getPlanRunForSession("unrelated-session")).toBeNull();
  expect(running.tasks.every((task) => Array.isArray(task.attempts))).toBe(true);
  expect(await service.getPlanAttemptEvidence("missing-attempt")).toEqual([]);
  expect((await service.requestPlanControl(runId, "pause")).run.status).toBe("paused");
  expect((await service.requestPlanControl(runId, "resume")).run.status).toBe("running");
  expect((await service.executeNextAttempt(runId))?.status).toBe("succeeded");
  if ((await service.getPlanRun(runId)).status === "running") {
    expect((await service.executeNextAttempt(runId))?.status).toBe("succeeded");
  }
  expect((await service.acceptPlanRun(runId)).run.status).toBe("completed");
  await expect(service.requestPlanControl(runId, "pause")).rejects.toThrow(/invalid/i);
}

describe("Plan adapter shared conformance", () => {
  beforeEach(setupNativeAdapter);
  it("keeps the Tauri adapter on the shared Plan contract", async () => expectConformance(tauriPlanClient, "native"));
  it("keeps the Web/mock adapter on the shared Plan contract", async () => expectConformance(webPlanClient, "web"));

  it("projects every durable native PlanRun phase without inference", async () => {
    await expectProjectionConformance(tauriPlanClient, "native-run", setNativeProjectionStatus);
  });

  it("projects every simulated Web/mock PlanRun phase with the same shape", async () => {
    const draft = await webPlanClient.generatePlanDraft({ planId: null, version: 1, goal: "Projection", projectPath: "D:/app", baseRef: "main", availableTools: [] });
    const { runId } = await webPlanClient.approvePlan(draft.id, "origin-session");
    await webPlanClient.startPlanRun(runId);
    await expectProjectionConformance(webPlanClient, runId, projectLatestWebPlanRunForTest);
  });
});
