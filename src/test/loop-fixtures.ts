import type { LoopDefinition, LoopEvidence, LoopExecutionAssessment, LoopIteration, LoopRun, LoopRunScope, LoopRunStatus } from "../types/loop";

export function loopDefinitionFixture(overrides: Partial<LoopDefinition> = {}): LoopDefinition {
  return {
    id: "definition-1", name: "Fixture Loop", enabled: true, projectPath: "D:/project", baseBranch: "main",
    goal: "Complete the requested change", acceptanceCriteria: ["Tests pass"], allowedPaths: ["src"], protectedPaths: [".git"],
    workerAgentId: "onepiece", verifierAgentId: "onepiece",
    verificationCommands: [{ id: "tests", kind: "process", program: "npm", args: ["test"], workingDirectory: null, timeoutSeconds: 60, required: true }],
    limits: { maxIterations: 3, stepTimeoutSeconds: 60, totalTimeoutSeconds: 600, maxConsecutiveRuntimeErrors: 2, maxConsecutiveNoProgress: 2 },
    version: 1, createdAt: "2026-08-21T00:00:00Z", updatedAt: "2026-08-21T00:00:00Z",
    scopeSchemaVersion: 1, requestedMode: "preventive-required", scopeState: "verified", ...overrides,
  };
}

export function loopAssessmentFixture(overrides: Partial<LoopExecutionAssessment> = {}): LoopExecutionAssessment {
  return {
    requestedMode: "preventive-required",
    surfaces: [
      { surface: "worker", coverage: "complete-enforcement", detail: "Native Worker session offers only host-mediated file tools.", blocking: false },
      { surface: "verifier", coverage: "complete-enforcement", detail: "Native Verifier session offers only read-only tools.", blocking: false },
      { surface: "verification:tests", coverage: "artifact-validation-only", detail: "Process check tests runs npm with no containment of its descendants.", blocking: false },
      { surface: "artifact-validation", coverage: "complete-enforcement", detail: "Complete worktree manifests are captured at every phase boundary.", blocking: false },
    ],
    blockers: [],
    limitations: ["verification:tests: artifact-validation-only — Process check tests runs npm with no containment of its descendants."],
    satisfiesRequestedMode: true,
    acknowledgementRequired: false,
    witnessDigest: "sha256:witness",
    assessedAt: "2026-08-21T00:00:00Z",
    simulated: false,
    ...overrides,
  };
}

export function loopRunScopeFixture(overrides: Partial<LoopRunScope> = {}): LoopRunScope {
  return {
    requestedMode: "preventive-required", scopeDigest: "sha256:scope", bindingStatus: "bound", assessment: loopAssessmentFixture(),
    sealedEvidenceId: null, acceptanceOperationId: null, baselineDigest: "sha256:baseline", allowedPaths: ["src"], protectedPaths: [".git"], ...overrides,
  };
}

export function loopEvidenceFixture(overrides: Partial<LoopEvidence> = {}): LoopEvidence {
  return {
    id: "evidence-1", runId: "run-1", iterationId: "iteration-1", kind: "worker", status: "passed",
    summary: "Worker completed.", operationId: "operation-1", commandId: null, exitCode: null, durationMs: 1_000,
    details: { changedFiles: 2, additions: 12, deletions: 3 }, createdAt: "2026-08-21T00:01:00Z", ...overrides,
  };
}

export function loopIterationFixture(overrides: Partial<LoopIteration> = {}): LoopIteration {
  return {
    id: "iteration-1", runId: "run-1", sequence: 1, status: "awaiting-acceptance",
    workerSessionId: "worker-1", verifierSessionId: "verifier-1", workerSummary: "Implemented the change.",
    verifierRecommendation: "pass", verifierFindings: [], decisionReason: "Ready for acceptance.", diffFingerprint: "diff-1",
    checkFailureFingerprint: null, userFeedback: null, evidence: [loopEvidenceFixture()],
    startedAt: "2026-08-21T00:00:30Z", completedAt: "2026-08-21T00:02:00Z", ...overrides,
  };
}

export function loopRunFixture(status: LoopRunStatus = "awaiting-acceptance", overrides: Partial<LoopRun> = {}): LoopRun {
  const definition = loopDefinitionFixture();
  return {
    id: "run-1", definitionId: definition.id, definitionSnapshot: definition, status, phase: "finalizing", terminalReason: null,
    currentIteration: 1, consecutiveRuntimeErrors: 0, consecutiveNoProgress: 0, pauseRequested: false,
    projectPath: definition.projectPath, worktreePath: "D:/project-loop", worktreeName: "loop", worktreeBranch: "vanehub/loop",
    activeOperationId: null, iterations: [loopIterationFixture({ status })], simulated: false,
    createdAt: "2026-08-21T00:00:00Z", startedAt: "2026-08-21T00:00:30Z", updatedAt: "2026-08-21T00:02:00Z",
    completedAt: ["succeeded", "failed", "cancelled"].includes(status) ? "2026-08-21T00:02:00Z" : null,
    revision: 1, scope: loopRunScopeFixture(), ...overrides,
  };
}

export const loopFixtureCases = {
  enabled: () => loopDefinitionFixture(),
  disabled: () => loopDefinitionFixture({ enabled: false }),
  legacy: () => loopDefinitionFixture({ scopeSchemaVersion: null, requestedMode: null, scopeState: "legacy-unverified" }),
  unavailableSelection: () => loopDefinitionFixture({ projectPath: "D:/missing", baseBranch: "deleted-branch" }),
  activeRun: () => loopRunFixture("running", { phase: "acting", activeOperationId: "worker-operation" }),
  multiIteration: () => loopRunFixture("awaiting-acceptance", { currentIteration: 2, iterations: [loopIterationFixture(), loopIterationFixture({ id: "iteration-2", sequence: 2, diffFingerprint: "diff-2" })] }),
  recoveryRequired: () => loopRunFixture("paused", { terminalReason: "recovery-required", phase: "verifying" }),
  scopeBindingMissing: () => loopRunFixture("paused", { terminalReason: "scope-binding-missing", phase: "acting", scope: loopRunScopeFixture({ bindingStatus: "missing", assessment: null, scopeDigest: null }) }),
  awaitingAcceptance: () => loopRunFixture("awaiting-acceptance"),
};
