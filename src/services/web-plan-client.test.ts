import { describe, expect, it } from "vitest";
import { webPlanClient } from "./web-plan-client";

describe("Web/mock Plan adapter", () => {
  it("dispatches a predecessor before a reordered successor", async () => {
    const draft = await webPlanClient.generatePlanDraft({
      planId: null, version: 1, goal: "Reordered DAG", projectPath: "D:/app", baseRef: "main", availableTools: [],
    });
    const predecessorId = draft.subtasks[0]!.id;
    const successorId = draft.subtasks[1]!.id;
    draft.subtasks = [draft.subtasks[1]!, draft.subtasks[0]!].map((task, ordinal) => ({ ...task, ordinal }));
    await webPlanClient.savePlanDraft(draft);
    const { runId } = await webPlanClient.approvePlan(draft.id);
    await webPlanClient.startPlanRun(runId);
    const run = await webPlanClient.getPlanRun(runId);
    expect(run.tasks.find((task) => task.status === "running")?.subtaskId).toBe(predecessorId);
    expect(run.tasks.find((task) => task.subtaskId === successorId)?.topologicalRank).toBe(1);
  });

  it("computes multi-level ranks when dependency edges are out of order", async () => {
    const draft = await webPlanClient.generatePlanDraft({
      planId: null, version: 1, goal: "Multi-level DAG", projectPath: "D:/app", baseRef: "main", availableTools: [],
    });
    const [first, second] = draft.subtasks;
    const third = { ...structuredClone(second!), id: `${second!.id}-third`, title: "Third", ordinal: 2 };
    draft.subtasks = [third, second!, first!].map((task, ordinal) => ({ ...task, ordinal }));
    draft.dependencies = [
      { predecessorId: second!.id, successorId: third.id },
      { predecessorId: first!.id, successorId: second!.id },
    ];
    await webPlanClient.savePlanDraft(draft);
    const { runId } = await webPlanClient.approvePlan(draft.id);
    await webPlanClient.startPlanRun(runId);
    const ranks = new Map((await webPlanClient.getPlanRun(runId)).tasks.map((task) => [task.subtaskId, task.topologicalRank]));
    expect([ranks.get(first!.id), ranks.get(second!.id), ranks.get(third.id)]).toEqual([0, 1, 2]);
  });

  it("selects the deterministic ordinal-first task between independent branches", async () => {
    const draft = await webPlanClient.generatePlanDraft({
      planId: null, version: 1, goal: "Independent DAG", projectPath: "D:/app", baseRef: "main", availableTools: [],
    });
    const independent = { ...structuredClone(draft.subtasks[0]!), id: `${draft.subtasks[0]!.id}-independent`, title: "Independent", ordinal: 0 };
    draft.subtasks = [
      { ...draft.subtasks[0]!, ordinal: 1 },
      { ...draft.subtasks[1]!, ordinal: 2 },
      independent,
    ];
    await webPlanClient.savePlanDraft(draft);
    const { runId } = await webPlanClient.approvePlan(draft.id);
    await webPlanClient.startPlanRun(runId);
    expect((await webPlanClient.getPlanRun(runId)).tasks.find((task) => task.status === "running")?.subtaskId).toBe(independent.id);
  });

  it("supports independent validation, version inspection, and draft deletion", async () => {
    const first = await webPlanClient.generatePlanDraft({
      planId: null, version: 1, goal: "Version one", projectPath: "D:/app", baseRef: "main", availableTools: [],
    });
    await webPlanClient.validatePlanDraft(first);
    const second = await webPlanClient.generatePlanDraft({
      planId: first.id, version: 2, goal: "Version two", projectPath: "D:/app", baseRef: "main", availableTools: [],
    });
    expect((await webPlanClient.listPlanVersions(first.id)).map((version) => version.version)).toEqual([2, 1]);
    await webPlanClient.deletePlanDraft(first.id);
    expect(await webPlanClient.getPlanDraft(first.id)).toBeNull();
    expect(await webPlanClient.listPlanVersions(first.id)).toEqual([]);

    second.dependencies.push({ predecessorId: second.subtasks[1]!.id, successorId: second.subtasks[0]!.id });
    await expect(webPlanClient.validatePlanDraft(second)).rejects.toThrow(/acyclic/i);
  });

  it("generates, approves, and advances a simulated serial Plan without native claims", async () => {
    const draft = await webPlanClient.generatePlanDraft({
      planId: null, version: 1, goal: "Implement Plan execution", projectPath: "D:/app",
      baseRef: "main", availableTools: ["shell"],
    });
    expect(draft.subtasks).toHaveLength(2);
    expect(draft.dependencies).toEqual([{ predecessorId: draft.subtasks[0]?.id, successorId: draft.subtasks[1]?.id }]);
    const { runId } = await webPlanClient.approvePlan(draft.id);
    const prepared = await webPlanClient.startPlanRun(runId);
    expect(prepared.worktreePath).toBe(`web-mock://retained/${runId}`);

    const first = await webPlanClient.getPlanRun(runId);
    expect(first.simulated).toBe(true);
    expect(first.tasks.filter((task) => task.status === "running")).toHaveLength(1);
    await webPlanClient.executeNextAttempt(runId);
    expect((await webPlanClient.getPlanRun(runId)).status).toBe("running");
    await webPlanClient.executeNextAttempt(runId);
    const complete = await webPlanClient.getPlanRun(runId);
    expect(complete.status).toBe("awaiting_acceptance");
    expect((await webPlanClient.acceptPlanRun(runId)).run.status).toBe("completed");
  });

  it("validates edited graphs and enforces control transitions", async () => {
    const draft = await webPlanClient.generatePlanDraft({
      planId: null, version: 1, goal: "Control", projectPath: "D:/app", baseRef: "main", availableTools: [],
    });
    draft.subtasks[0]!.acceptanceCriteria = [];
    await expect(webPlanClient.savePlanDraft(draft)).rejects.toThrow("1-3 acceptance criteria");
    draft.subtasks[0]!.acceptanceCriteria = ["ok"];
    await webPlanClient.savePlanDraft(draft);
    const { runId } = await webPlanClient.approvePlan(draft.id);
    await expect(webPlanClient.requestPlanControl(runId, "pause")).rejects.toThrow("invalid");
    await webPlanClient.startPlanRun(runId);
    expect((await webPlanClient.requestPlanControl(runId, "pause")).run.status).toBe("paused");
    expect((await webPlanClient.requestPlanControl(runId, "resume")).run.status).toBe("running");
    expect((await webPlanClient.requestPlanControl(runId, "cancel")).run.status).toBe("cancelled");
  });
});
