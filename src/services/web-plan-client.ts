import type { PlanService } from "./plan-service";
import type { GeneratePlanDraftInput, PlanAttemptEvidence, PlanControlKind, PlanControlResult, PlanDependency, PlanDraft, PlanRunDetail, PlanRunStatus, PlanSubTask, PlanSubTaskRun } from "../types/plan";
const drafts = new Map<string, PlanDraft>(); const versions = new Map<string, PlanDraft[]>();
const runs = new Map<string, PlanRunDetail>(); const attemptEvidence = new Map<string, PlanAttemptEvidence[]>();
const runDependencies = new Map<string, PlanDependency[]>(); const driverTimers = new Map<string, ReturnType<typeof setTimeout>>();
let sequence = 0;
export function markFirstWebPlanRunRecoveryRequired(): void {
  const run = [...runs.values()].at(-1);
  if (!run || !["running", "paused"].includes(run.status)) throw new Error("No recoverable Web/mock PlanRun exists.");
  run.status = "recovery_required"; run.updatedAt = now(); run.availableControls = availableControls(run);
}
export function projectLatestWebPlanRunForTest(status: PlanRunStatus): void {
  const run = [...runs.values()].at(-1);
  if (!run) throw new Error("No Web/mock PlanRun exists.");
  run.status = status; run.updatedAt = now(); run.availableControls = availableControls(run);
}
export function triggerLatestWebPlanRepairForTest(): void {
  const run = [...runs.values()].at(-1); const active = run?.tasks.find((taskRun) => taskRun.status === "running");
  if (!run || !active) throw new Error("No active Web/mock Plan task exists.");
  const existing = driverTimers.get(run.id); if (existing) clearTimeout(existing); driverTimers.delete(run.id); const failedId = nextId("attempt");
  active.attempts.push({ id: failedId, sequence: active.attempts.length + 1, status: "failed", sessionId: null, profileId: null, executionRunId: null, operationId: null, tokenUsage: 0, toolCallCount: 0, errorClass: "verification_failed", startedAt: now(), completedAt: now() });
  attemptEvidence.set(failedId, [{ id: nextId("evidence"), commandId: "simulated-tests", status: "failed", exitCode: 1, durationMs: 0, outputSummary: "Simulated verification failure retained for repair.", createdAt: now() }]);
  run.status = "repairing"; run.updatedAt = now(); run.availableControls = availableControls(run);
  const timer = setTimeout(() => { driverTimers.delete(run.id); run.status = "running"; advance(run); scheduleDriver(run); }, 2_500); driverTimers.set(run.id, timer);
}
function nextId(prefix: string): string { sequence += 1; return `${prefix}-mock-${sequence}`; }
function now(): string { return new Date(Date.UTC(2026, 0, 1, 0, 0, sequence)).toISOString(); }
function copy<T>(value: T): T { return structuredClone(value); }
function persistDraft(draft: PlanDraft): void {
  drafts.set(draft.id, copy(draft));
  const history = versions.get(draft.id) ?? []; const retained = history.filter((candidate) => candidate.versionId !== draft.versionId);
  versions.set(draft.id, [...retained, copy(draft)].sort((left, right) => right.version - left.version));
}
function task(id: string, ordinal: number, title: string, description: string): PlanSubTask {
  const commandId = `verify-${id}`;
  return {
    id, title, description,
    acceptanceCriteria: [`${title} is complete and verifiable`], criterionEvidence: [{ criterionIndex: 0, kind: "automated", commandId }],
    ordinal, assignedRole: "worker",
    limits: { tokenBudget: 8_000, toolCallLimit: 30, timeoutSeconds: 900 }, validationCommands: [{ id: commandId, program: "npm", args: ["run", "test"], workingDirectory: null, timeoutSeconds: 600, required: true }],
  };
}
function validateDraft(draft: PlanDraft): void {
  if (draft.subtasks.length < 1 || draft.subtasks.length > 10) throw new Error("A Plan requires 1-10 SubTasks.");
  const ids = new Set(draft.subtasks.map((subtask) => subtask.id));
  if (ids.size !== draft.subtasks.length) throw new Error("SubTask IDs must be unique.");
  for (const subtask of draft.subtasks) {
    if (!subtask.title.trim() || !subtask.description.trim()) throw new Error(`SubTask ${subtask.id} requires a title and description.`);
    if (subtask.acceptanceCriteria.length < 1 || subtask.acceptanceCriteria.length > 3) {
      throw new Error(`SubTask ${subtask.id} requires 1-3 acceptance criteria.`);
    }
    const requiredCommands = new Set(subtask.validationCommands.filter((command) => command.required).map((command) => command.id));
    if (requiredCommands.size === 0) throw new Error(`SubTask ${subtask.id} requires a required validation command.`);
    if (subtask.criterionEvidence.length !== subtask.acceptanceCriteria.length) throw new Error(`SubTask ${subtask.id} has unresolved criterion evidence.`);
    subtask.criterionEvidence.forEach((binding, index) => {
      if (binding.criterionIndex !== index) throw new Error(`SubTask ${subtask.id} has invalid criterion evidence indexes.`);
      if (binding.kind === "automated" && (!binding.commandId || !requiredCommands.has(binding.commandId))) throw new Error(`SubTask ${subtask.id} references an unknown required command.`);
      if (binding.kind === "manual" && binding.commandId !== null) throw new Error(`SubTask ${subtask.id} manual evidence cannot reference a command.`);
    });
  }
  if (draft.executionPolicy.maxAttemptsPerSubtask < 1 || draft.executionPolicy.maxAttemptsPerSubtask > 5) throw new Error("Maximum attempts must be between one and five.");
  if (draft.executionPolicy.repairEligibleClasses.length === 0) throw new Error("At least one repair class is required.");
  if (!draft.executionPolicy.finalValidationCommands.some((command) => command.required)) throw new Error("A required final validation command is required.");
  const successors = new Map<string, string[]>();
  const edges = new Set<string>();
  for (const edge of draft.dependencies) {
    if (!ids.has(edge.predecessorId) || !ids.has(edge.successorId) || edge.predecessorId === edge.successorId) {
      throw new Error("Plan dependency endpoints must reference distinct SubTasks.");
    }
    const identity = `${edge.predecessorId}\u0000${edge.successorId}`;
    if (edges.has(identity)) throw new Error("Plan dependency edges must be unique.");
    edges.add(identity);
    successors.set(edge.predecessorId, [...(successors.get(edge.predecessorId) ?? []), edge.successorId]);
  }
  const visiting = new Set<string>();
  const visited = new Set<string>();
  const cyclic = (id: string): boolean => {
    if (visiting.has(id)) return true;
    if (visited.has(id)) return false;
    visiting.add(id);
    const found = (successors.get(id) ?? []).some(cyclic);
    visiting.delete(id);
    visited.add(id);
    return found;
  };
  if ([...ids].some(cyclic)) throw new Error("Plan dependencies must form an acyclic graph.");
}
function toRunTask(subtask: PlanSubTask, rank: number): PlanSubTaskRun {
  return {
    id: nextId("subtask-run"),
    subtaskId: subtask.id,
    title: subtask.title,
    status: "pending",
    topologicalRank: rank,
    ordinal: subtask.ordinal,
    resultSummary: null,
    changedFiles: [],
    verificationSummary: null,
    attempts: [],
  };
}
function topologicalRanks(draft: PlanDraft): Map<string, number> {
  const tasks = new Map(draft.subtasks.map((subtask) => [subtask.id, subtask]));
  const indegree = new Map(draft.subtasks.map((subtask) => [subtask.id, 0]));
  const successors = new Map<string, string[]>();
  const ranks = new Map(draft.subtasks.map((subtask) => [subtask.id, 0]));
  for (const edge of draft.dependencies) {
    indegree.set(edge.successorId, (indegree.get(edge.successorId) ?? 0) + 1);
    successors.set(edge.predecessorId, [...(successors.get(edge.predecessorId) ?? []), edge.successorId]);
  }
  const compare = (left: string, right: string) => (tasks.get(left)?.ordinal ?? 0) - (tasks.get(right)?.ordinal ?? 0) || left.localeCompare(right);
  const ready = [...indegree].filter(([, degree]) => degree === 0).map(([id]) => id).sort(compare);
  while (ready.length > 0) {
    const current = ready.shift()!;
    for (const successor of (successors.get(current) ?? []).sort(compare)) {
      ranks.set(successor, Math.max(ranks.get(successor) ?? 0, (ranks.get(current) ?? 0) + 1));
      const remaining = (indegree.get(successor) ?? 0) - 1;
      indegree.set(successor, remaining);
      if (remaining === 0) { ready.push(successor); ready.sort(compare); }
    }
  }
  return ranks;
}
function nextEligibleTask(run: PlanRunDetail): PlanSubTaskRun | undefined {
  const predecessors = new Map<string, string[]>();
  for (const edge of runDependencies.get(run.id) ?? []) {
    predecessors.set(edge.successorId, [...(predecessors.get(edge.successorId) ?? []), edge.predecessorId]);
  }
  const bySubtask = new Map(run.tasks.map((task) => [task.subtaskId, task]));
  return run.tasks
    .filter((task) => task.status === "pending" && (predecessors.get(task.subtaskId) ?? []).every((id) => bySubtask.get(id)?.status === "succeeded"))
    .sort((left, right) => left.topologicalRank - right.topologicalRank || left.ordinal - right.ordinal || left.subtaskId.localeCompare(right.subtaskId))[0];
}
function availableControls(run: PlanRunDetail): PlanControlKind[] {
  switch (run.status) {
    case "running": return ["pause", "cancel"];
    case "paused": return ["resume", "cancel"];
    case "recovery_required": return ["recover", "cancel"];
    case "action_required": return ["retry", "cancel"];
    case "awaiting_acceptance": return ["accept"];
    default: return [];
  }
}
function scheduleDriver(run: PlanRunDetail): void {
  if (driverTimers.has(run.id) || run.status !== "running") return;
  const timer = setTimeout(() => {
    driverTimers.delete(run.id);
    if (run.status !== "running") return;
    advance(run);
    scheduleDriver(run);
  }, 2_000);
  driverTimers.set(run.id, timer);
}
function advance(run: PlanRunDetail): void {
  if (run.status !== "running") return;
  const active = run.tasks.find((candidate) => candidate.status === "running");
  if (active) {
    const completedAt = now();
    active.status = "succeeded";
    active.resultSummary = "Simulated Web/mock task completed.";
    active.verificationSummary = "Simulated acceptance checks passed.";
    const attemptId = nextId("attempt");
    active.attempts.push({
      id: attemptId, sequence: active.attempts.length + 1, status: "succeeded",
      sessionId: null, profileId: null, executionRunId: null, operationId: null,
      tokenUsage: 0, toolCallCount: 0, errorClass: null, startedAt: completedAt,
      completedAt,
    });
    attemptEvidence.set(attemptId, []);
  }
  const next = nextEligibleTask(run);
  if (next) next.status = "running";
  else if (run.tasks.every((candidate) => candidate.status === "succeeded")) {
    const createdAt = now();
    const sequence = (run.finalization?.sequence ?? 0) + 1;
    run.status = "awaiting_acceptance";
    run.finalization = {
      id: nextId("finalization"), sequence, status: "succeeded",
      evidence: [{ id: nextId("final-evidence"), commandId: "final-tests", status: "passed", exitCode: 0, durationMs: 0, outputSummary: "Simulated final verification passed.", createdAt }],
      repairAttempts: [],
    };
  }
  run.completedTasks = run.tasks.filter((candidate) => candidate.status === "succeeded").length;
  run.updatedAt = now();
  run.availableControls = availableControls(run);
}
function control(runId: string, kind: PlanControlKind): PlanControlResult {
  const run = runs.get(runId);
  if (!run) throw new Error("PlanRun was not found.");
  if (kind === "pause" && run.status === "running") run.status = "paused";
  else if (kind === "resume" && run.status === "paused") { run.status = "running"; scheduleDriver(run); }
  else if (kind === "cancel" && ["running", "paused", "recovery_required", "action_required"].includes(run.status)) run.status = "cancelled";
  else if (kind === "recover" && run.status === "recovery_required") run.status = "paused";
  else if (kind === "retry" && run.status === "action_required" && run.finalization?.status === "failed") {
    run.status = "running";
    advance(run);
    scheduleDriver(run);
  }
  else if (kind === "accept" && run.status === "awaiting_acceptance") run.status = "completed";
  else throw new Error(`Control ${kind} is invalid while PlanRun is ${run.status}.`);
  run.updatedAt = now();
  run.availableControls = availableControls(run);
  return { requestId: nextId("control"), run: copy(run) };
}
export const webPlanClient: PlanService = {
  async generatePlanDraft(input: GeneratePlanDraftInput) {
    if (!input.goal.trim() || !input.projectPath.trim() || !input.baseRef.trim()) throw new Error("Goal, project path, and base ref are required.");
    const id = input.planId ?? nextId("plan");
    const firstId = nextId("task");
    const secondId = nextId("task");
    const draft: PlanDraft = {
      id, versionId: nextId("version"), version: input.version, goal: input.goal.trim(),
      projectPath: input.projectPath.trim(), baseRef: input.baseRef.trim(), plannerProfileId: "web-mock-profile",
      discovery: { status: "complete", limitations: ["Web/mock discovery is deterministic and simulated."] },
      executionPolicy: {
        maxAttemptsPerSubtask: 3,
        repairEligibleClasses: ["verification_failed"],
        finalValidationCommands: [{ id: "final-tests", program: "npm", args: ["run", "test"], workingDirectory: null, timeoutSeconds: 600, required: true }],
      },
      subtasks: [task(firstId, 0, "Analyze the change", input.goal), task(secondId, 1, "Implement and verify", input.goal)],
      dependencies: [{ predecessorId: firstId, successorId: secondId }],
    };
    persistDraft(draft);
    return copy(draft);
  },
  async validatePlanDraft(input) { validateDraft(input); },
  async savePlanDraft(input) { validateDraft(input); persistDraft(input); return copy(input); },
  async getPlanDraft(planId) { return copy(drafts.get(planId) ?? null); },
  async listPlanVersions(planId) { return copy(versions.get(planId) ?? []); },
  async deletePlanDraft(planId) {
    if ([...runs.values()].some((run) => run.planId === planId)) throw new Error("An executing or approved Plan cannot be deleted.");
    drafts.delete(planId);
    versions.delete(planId);
  },
  async approvePlan(planId, originatingSessionId) {
    const draft = drafts.get(planId);
    if (!draft) throw new Error("Plan was not found.");
    validateDraft(draft);
    const runId = nextId("run");
    const createdAt = now();
    const ranks = topologicalRanks(draft);
    runs.set(runId, {
      id: runId, planId, status: "queued", completedTasks: 0, totalTasks: draft.subtasks.length,
      simulated: true, createdAt, updatedAt: createdAt, projectPath: draft.projectPath, baseRef: draft.baseRef,
      baseOid: null, worktreePath: null, worktreeName: null, worktreeBranch: null,
      originatingSessionId: originatingSessionId ?? null,
      tasks: draft.subtasks.map((subtask) => toRunTask(subtask, ranks.get(subtask.id) ?? 0)), availableControls: [],
      finalization: null,
    });
    runDependencies.set(runId, copy(draft.dependencies));
    return { runId, summary: { projectPath: draft.projectPath, taskCount: draft.subtasks.length, retainedWorktree: true, automaticGitOperations: false } };
  },
  async startPlanRun(runId) {
    const run = runs.get(runId);
    if (!run || run.status !== "queued") throw new Error("Queued PlanRun was not found.");
    run.status = "running";
    run.baseOid = "web-mock-no-git";
    run.worktreeName = `plan-${runId}`;
    run.worktreeBranch = `web-mock/${runId}`;
    run.worktreePath = `web-mock://retained/${runId}`;
    advance(run);
    scheduleDriver(run);
    return { runId, status: "running", projectPath: run.projectPath, baseOid: run.baseOid, worktreePath: run.worktreePath, worktreeName: run.worktreeName, worktreeBranch: run.worktreeBranch };
  },
  async executeNextAttempt(runId) {
    const run = runs.get(runId);
    if (!run || run.status !== "running") return null;
    advance(run);
    const latest = run.tasks.flatMap((candidate) => candidate.attempts).at(-1);
    return latest ? {
      attemptId: latest.id, sessionId: latest.sessionId ?? "web-mock-session",
      status: latest.status === "succeeded" ? "succeeded" as const : "failed" as const,
      contextTruncated: false,
    } : null;
  },
  async listPlanRuns() {
    const items = [...runs.values()].map((run) => ({
      id: run.id, planId: run.planId, status: run.status, completedTasks: run.completedTasks,
      totalTasks: run.totalTasks, simulated: run.simulated, createdAt: run.createdAt, updatedAt: run.updatedAt,
    }));
    return { items: copy(items), nextCursor: null };
  },
  async getPlanRun(runId) { const run = runs.get(runId); if (!run) throw new Error("PlanRun was not found."); return copy(run); },
  async getPlanRunForSession(sessionId) {
    const run = [...runs.values()].filter((candidate) => candidate.originatingSessionId === sessionId).at(-1);
    return run ? copy({ id: run.id, planId: run.planId, status: run.status, completedTasks: run.completedTasks, totalTasks: run.totalTasks, simulated: run.simulated, createdAt: run.createdAt, updatedAt: run.updatedAt }) : null;
  },
  async getPlanAttemptEvidence(attemptId) {
    return copy(attemptEvidence.get(attemptId) ?? []);
  },
  async requestPlanControl(runId, kind) { return control(runId, kind); },
  async retryPlanSubtask(runId, subtaskRunId) {
    const run = runs.get(runId); const subtask = run?.tasks.find((candidate) => candidate.id === subtaskRunId);
    if (!run || !subtask || !["failed", "interrupted"].includes(subtask.status)) throw new Error("SubTask cannot be retried.");
    subtask.status = "pending"; run.status = "running"; advance(run); scheduleDriver(run);
    return { requestId: nextId("retry"), run: copy(run) };
  },
  async recoverPlanRun(runId) { return control(runId, "recover"); },
  async acceptPlanRun(runId) { return control(runId, "accept"); },
};
