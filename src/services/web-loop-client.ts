import { i18n } from "../i18n";
import type {
  ContinueLoopInput,
  LoopDefinition,
  LoopEvent,
  SaveLoopDefinitionInput,
  StartLoopResult,
} from "../types/loop";
import type { LoopRun } from "../types/loop";
import type { LoopService } from "./loop-service";
import { mockAgents } from "./mock-agent-data";
import { nowIso } from "./web-mock-clock";
import { prependWebAgentRun, projectWebOwnerRun, setWebAgentRunEvents } from "./web-agent-run-state";
import { createWebLoopRoleSession, scheduleWebLoopPhase } from "./web-loop-scheduler";
import {
  clearWebLoopTimer,
  cloneLoopValue,
  createWebLoopIteration,
  emitLoopEvent,
  findLoopDefinition,
  findLoopRun,
  getWebLoopTimer,
  listWebLoopDefinitions,
  listWebLoopRuns,
  nextWebLoopDefinitionSequence,
  nextWebLoopRunSequence,
  peekWebLoopRunSequence,
  prependWebLoopRun,
  replaceWebLoopDefinitions,
  subscribeWebLoopEvents,
} from "./web-loop-state";

function validateLoopDefinitionInput(input: SaveLoopDefinitionInput) {
  const name = input.name.trim();
  const projectPath = input.projectPath.trim();
  const baseBranch = input.baseBranch.trim();
  const goal = input.goal.trim();
  if (!name || !projectPath || !baseBranch || !goal) throw new Error(i18n.t("loops.editor.error.scope"));
  if (!mockAgents.some((agent) => agent.id === input.workerAgentId)) throw new Error(i18n.t("loops.web.error.unsupportedWorker", { agentId: input.workerAgentId }));
  if (!mockAgents.some((agent) => agent.id === input.verifierAgentId)) throw new Error(i18n.t("loops.web.error.unsupportedVerifier", { agentId: input.verifierAgentId }));
  if (input.acceptanceCriteria.every((criterion) => !criterion.trim())) throw new Error(i18n.t("loops.editor.error.acceptance"));
  if (input.verificationCommands.length === 0) throw new Error(i18n.t("loops.editor.error.verificationRequired"));
  for (const command of input.verificationCommands) {
    if (!command.id.trim() || !command.program.trim() || command.timeoutSeconds < 1) throw new Error(i18n.t("loops.web.error.invalidCommand"));
    const workingDirectory = command.workingDirectory?.trim() ?? null;
    if (workingDirectory && (/^(?:[a-zA-Z]:[\\/]|[\\/])/.test(workingDirectory) || workingDirectory.split(/[\\/]+/).includes(".."))) {
      throw new Error(i18n.t("loops.editor.error.verificationDirectory"));
    }
  }
  const { limits } = input;
  if (
    limits.maxIterations < 1 || limits.maxIterations > 20 ||
    limits.stepTimeoutSeconds < 1 || limits.totalTimeoutSeconds < limits.stepTimeoutSeconds ||
    limits.maxConsecutiveRuntimeErrors < 1 || limits.maxConsecutiveNoProgress < 1
  ) throw new Error(i18n.t("loops.editor.error.limits"));
  return {
    ...input,
    name,
    projectPath,
    baseBranch,
    goal,
    acceptanceCriteria: input.acceptanceCriteria.map((value) => value.trim()).filter(Boolean),
    allowedPaths: input.allowedPaths.map((value) => value.trim()).filter(Boolean),
    protectedPaths: input.protectedPaths.map((value) => value.trim()).filter(Boolean),
    verificationCommands: input.verificationCommands.map((command) => ({
      ...command,
      id: command.id.trim(),
      program: command.program.trim(),
      args: command.args.map((value) => value.trim()).filter(Boolean),
      workingDirectory: command.workingDirectory?.trim() || null,
    })),
    limits: { ...input.limits },
  };
}

export const webLoopClient: LoopService = {
  async listLoopDefinitions() {
    return cloneLoopValue([...listWebLoopDefinitions()].sort((left, right) => right.updatedAt.localeCompare(left.updatedAt)));
  },

  async createLoopDefinition(input: SaveLoopDefinitionInput) {
    const validated = validateLoopDefinitionInput(input);
    const timestamp = nowIso();
    const definition: LoopDefinition = {
      ...validated,
      id: `web-loop-${nextWebLoopDefinitionSequence()}`,
      version: 1,
      createdAt: timestamp,
      updatedAt: timestamp,
    };
    replaceWebLoopDefinitions([definition, ...listWebLoopDefinitions()]);
    return cloneLoopValue(definition);
  },

  async updateLoopDefinition(definitionId: string, input: SaveLoopDefinitionInput) {
    const current = findLoopDefinition(definitionId);
    if (input.expectedVersion != null && input.expectedVersion !== current.version) throw new Error(i18n.t("loops.web.error.versionConflict"));
    const validated = validateLoopDefinitionInput(input);
    const updated: LoopDefinition = {
      ...validated,
      id: current.id,
      version: current.version + 1,
      createdAt: current.createdAt,
      updatedAt: nowIso(),
    };
    replaceWebLoopDefinitions(listWebLoopDefinitions().map((candidate) => candidate.id === definitionId ? updated : candidate));
    return cloneLoopValue(updated);
  },

  async deleteLoopDefinition(definitionId: string) {
    findLoopDefinition(definitionId);
    if (listWebLoopRuns().some((run) => run.definitionId === definitionId && ["queued", "running", "paused", "awaiting-acceptance"].includes(run.status))) {
      throw new Error(i18n.t("loops.web.error.activeRunDelete"));
    }
    replaceWebLoopDefinitions(listWebLoopDefinitions().filter((candidate) => candidate.id !== definitionId));
  },

  async listLoopRuns(definitionId?: string) {
    const runs = definitionId ? listWebLoopRuns().filter((run) => run.definitionId === definitionId) : listWebLoopRuns();
    return cloneLoopValue([...runs].sort((left, right) => right.createdAt.localeCompare(left.createdAt)));
  },

  async getLoopRun(runId: string) {
    return cloneLoopValue(findLoopRun(runId));
  },

  async startLoop(definitionId: string): Promise<StartLoopResult> {
    const definition = findLoopDefinition(definitionId);
    if (!definition.enabled) throw new Error(i18n.t("loops.web.error.definitionDisabled"));
    if (listWebLoopRuns().some((run) => run.definitionId === definitionId && ["queued", "running", "paused", "awaiting-acceptance"].includes(run.status))) {
      throw new Error(i18n.t("loops.web.error.activeRunExists"));
    }
    const timestamp = nowIso();
    const runId = `web-loop-run-${nextWebLoopRunSequence()}`;
    const operationId = `web-loop-prepare-${runId}`;
    const run: LoopRun = {
      id: runId,
      definitionId,
      definitionSnapshot: cloneLoopValue(definition),
      status: "queued",
      phase: "preparing",
      terminalReason: null,
      currentIteration: 1,
      consecutiveRuntimeErrors: 0,
      consecutiveNoProgress: 0,
      pauseRequested: false,
      projectPath: definition.projectPath,
      worktreePath: null,
      worktreeName: null,
      worktreeBranch: null,
      activeOperationId: operationId,
      iterations: [],
      simulated: true,
      createdAt: timestamp,
      startedAt: null,
      updatedAt: timestamp,
      completedAt: null,
    };
    prependWebLoopRun(run);
    const canonicalId = `018f0f17-4d6a-7e20-b41d-66c5271a${String(peekWebLoopRunSequence()).padStart(4, "0")}`;
    prependWebAgentRun({
      id: canonicalId,
      owner: { ownerType: "loop_run", ownerId: runId },
      links: [{ linkType: "loop_definition", linkId: definitionId }],
      parentRunId: null,
      state: "preparing",
      recoveryPolicy: "owner_reconciles",
      retryCount: 0,
      maxRetries: 3,
      reasonCode: null,
      createdAt: timestamp,
      updatedAt: timestamp,
      version: 2,
      lastWitness: `web-loop-prepare:${runId}`,
    });
    setWebAgentRunEvents(canonicalId, []);
    emitLoopEvent(run);
    scheduleWebLoopPhase(run);
    return { run: cloneLoopValue(run), operationId };
  },

  async pauseLoop(runId: string) {
    const run = findLoopRun(runId);
    if (run.status !== "queued" && run.status !== "running") throw new Error(i18n.t("loops.web.error.pauseState"));
    run.pauseRequested = true;
    emitLoopEvent(run);
    return cloneLoopValue(run);
  },

  async resumeLoop(runId: string) {
    const run = findLoopRun(runId);
    if (run.status !== "paused") throw new Error(i18n.t("loops.web.error.resumeState"));
    run.status = run.iterations.length === 0 ? "queued" : "running";
    projectWebOwnerRun(run.id, "running");
    run.terminalReason = null;
    run.pauseRequested = false;
    emitLoopEvent(run);
    scheduleWebLoopPhase(run);
    return cloneLoopValue(run);
  },

  async cancelLoop(runId: string) {
    const run = findLoopRun(runId);
    if (["succeeded", "failed", "cancelled"].includes(run.status)) return cloneLoopValue(run);
    const timer = getWebLoopTimer(run.id);
    if (timer) clearTimeout(timer);
    clearWebLoopTimer(run.id);
    run.status = "cancelled";
    projectWebOwnerRun(run.id, "cancelled");
    run.terminalReason = "user-stopped";
    run.completedAt = nowIso();
    run.pauseRequested = false;
    emitLoopEvent(run);
    return cloneLoopValue(run);
  },

  async acceptLoop(runId: string) {
    const run = findLoopRun(runId);
    if (run.status !== "awaiting-acceptance") throw new Error(i18n.t("loops.web.error.acceptanceState"));
    run.status = "succeeded";
    projectWebOwnerRun(run.id, "completed");
    run.terminalReason = "goal-met";
    run.completedAt = nowIso();
    emitLoopEvent(run);
    return cloneLoopValue(run);
  },

  async continueLoop(input: ContinueLoopInput) {
    const run = findLoopRun(input.runId);
    const feedback = input.feedback.trim();
    if (run.status !== "awaiting-acceptance") throw new Error(i18n.t("loops.web.error.acceptanceState"));
    if (!feedback) throw new Error(i18n.t("loops.web.error.feedbackRequired"));
    if (run.currentIteration >= run.definitionSnapshot.limits.maxIterations) throw new Error(i18n.t("loops.web.error.maxIterations"));
    run.currentIteration += 1;
    const iteration = createWebLoopIteration(run.id, run.currentIteration, feedback);
    run.iterations.push(iteration);
    createWebLoopRoleSession(run, iteration, "worker");
    run.status = "running";
    projectWebOwnerRun(run.id, "running");
    run.phase = "acting";
    run.terminalReason = null;
    emitLoopEvent(run, "iteration-updated");
    scheduleWebLoopPhase(run);
    return cloneLoopValue(run);
  },

  async rejectLoop(runId: string) {
    const run = findLoopRun(runId);
    if (run.status !== "awaiting-acceptance") throw new Error(i18n.t("loops.web.error.acceptanceState"));
    run.status = "cancelled";
    projectWebOwnerRun(run.id, "cancelled");
    run.terminalReason = "user-rejected";
    run.completedAt = nowIso();
    emitLoopEvent(run);
    return cloneLoopValue(run);
  },

  async subscribeLoopEvents(runId: string, handler: (event: LoopEvent) => void) {
    return subscribeWebLoopEvents(runId, handler);
  },
};
