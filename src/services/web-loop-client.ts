import { i18n } from "../i18n";
import type {
  ContinueLoopInput,
  LoopDefinition,
  LoopEvent,
  SaveLoopDefinitionInput,
  StartLoopResult,
} from "../types/loop";
import { validateLoopDefinitionInput } from "./web-loop-definition-validation";
import type { LoopRun } from "../types/loop";
import type { LoopWorkbenchService } from "./loop-service";
import { nowIso } from "./web-mock-clock";
import { prependWebAgentRun, projectWebOwnerRun, setWebAgentRunEvents } from "./web-agent-run-state";
import { createWebLoopRoleSession, scheduleWebLoopPhase } from "./web-loop-scheduler";
import { consumeWebLoopAudit, requireWebLoopRevision, simulateLoopAssessment, simulateLoopRunScope } from "./web-loop-scope";
import {
  addLoopEvidence,
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
import { activeLoopStatuses, webLoopReadinessClient } from "./web-loop-readiness-client";

export const webLoopClient: LoopWorkbenchService = {
  ...webLoopReadinessClient,

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
    if (listWebLoopRuns().some((run) => run.definitionId === definitionId && activeLoopStatuses.includes(run.status))) {
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

  async startLoop(definitionId: string, envelope): Promise<StartLoopResult> {
    const definition = findLoopDefinition(definitionId);
    if (!definition.enabled) throw new Error(i18n.t("loops.web.error.definitionDisabled"));
    requireWebLoopRevision(envelope, definition.version);
    if (definition.scopeState !== "verified") throw new Error(i18n.t("loops.web.error.legacyScope"));
    const assessment = simulateLoopAssessment(definition);
    if (!assessment.satisfiesRequestedMode) throw new Error(i18n.t("loops.web.error.coverage", { mode: assessment.requestedMode, blockers: assessment.blockers.join("; ") }));
    consumeWebLoopAudit(envelope, "start", definition.id, definition.version, definition);
    if (listWebLoopRuns().some((run) => run.definitionId === definitionId && activeLoopStatuses.includes(run.status))) {
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
      revision: 1,
      scope: simulateLoopRunScope(definition, assessment),
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

  async resumeLoop(runId: string, envelope) {
    const run = findLoopRun(runId);
    if (run.status !== "paused") throw new Error(i18n.t("loops.web.error.resumeState"));
    requireWebLoopRevision(envelope, run.revision);
    if (run.terminalReason === "scope-binding-missing" || run.scope?.bindingStatus !== "bound") throw new Error(i18n.t("loops.web.error.bindingMissing"));
    consumeWebLoopAudit(envelope, "resume", run.id, run.revision, run.definitionSnapshot);
    run.status = run.iterations.length === 0 ? "queued" : run.phase === "finalizing" ? "awaiting-acceptance" : "running";
    projectWebOwnerRun(run.id, run.status === "awaiting-acceptance" ? "verifying" : "running");
    run.terminalReason = null;
    run.pauseRequested = false;
    emitLoopEvent(run);
    if (run.status !== "awaiting-acceptance") scheduleWebLoopPhase(run);
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
    const result = await this.requestLoopAcceptance({ runId });
    return result.run;
  },

  async requestLoopAcceptance(input) {
    const run = findLoopRun(input.runId);
    if (run.status !== "awaiting-acceptance") throw new Error(i18n.t("loops.web.error.acceptanceState"));
    requireWebLoopRevision({ expectedRevision: input.expectedRevision }, run.revision);
    if (input.expectedScopeDigest && input.expectedScopeDigest !== run.scope?.scopeDigest) throw new Error(i18n.t("loops.web.error.staleRevision"));
    if (run.scope?.acceptanceOperationId) throw new Error(i18n.t("loops.web.error.acceptanceInProgress"));
    // The simulated seal and commit happen inline: no real worktree exists to rescan, so an
    // asynchronous operation would only pretend to wait.
    const operationId = `web-loop-accept-${run.id}-${run.revision}`;
    const iteration = run.iterations.at(-1) ?? null;
    const sealedEvidenceId = `web-loop-sealed-${run.id}-${run.revision}`;
    if (run.scope) {
      run.scope.sealedEvidenceId = sealedEvidenceId;
      run.scope.acceptanceOperationId = null;
    }
    addLoopEvidence(run, iteration, {
      kind: "scope-evidence",
      status: "passed",
      summary: i18n.t("loops.web.evidence.sealed"),
      operationId,
      commandId: null,
      exitCode: null,
      durationMs: 0,
      details: { simulated: true, sealedEvidenceId, scopeDigest: run.scope?.scopeDigest ?? null },
    });
    run.revision += 1;
    run.status = "succeeded";
    run.phase = "finalizing";
    run.terminalReason = "goal-met";
    run.completedAt = nowIso();
    if (iteration) { iteration.status = "succeeded"; iteration.completedAt = run.completedAt; }
    projectWebOwnerRun(run.id, "completed");
    addLoopEvidence(run, iteration, {
      kind: "acceptance",
      status: "passed",
      summary: i18n.t("loops.web.evidence.accepted"),
      operationId,
      commandId: null,
      exitCode: null,
      durationMs: 0,
      details: { simulated: true, sealedEvidenceId },
    });
    emitLoopEvent(run);
    return { run: cloneLoopValue(run), operationId };
  },

  async continueLoop(input: ContinueLoopInput) {
    const run = findLoopRun(input.runId);
    const feedback = input.feedback.trim();
    if (run.status !== "awaiting-acceptance") throw new Error(i18n.t("loops.web.error.acceptanceState"));
    if (!feedback) throw new Error(i18n.t("loops.web.error.feedbackRequired"));
    if (run.currentIteration >= run.definitionSnapshot.limits.maxIterations) throw new Error(i18n.t("loops.web.error.maxIterations"));
    requireWebLoopRevision(input.envelope, run.revision);
    consumeWebLoopAudit(input.envelope, "continue", run.id, run.revision, run.definitionSnapshot);
    run.revision += 1;
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
