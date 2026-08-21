import type { LoopDefinition, LoopEvidence, LoopIteration, LoopRun, LoopRunStatus } from "../types/loop";

export function loopDefinitionFixture(overrides: Partial<LoopDefinition> = {}): LoopDefinition {
  return {
    id: "definition-1", name: "Fixture Loop", enabled: true, projectPath: "D:/project", baseBranch: "main",
    goal: "Complete the requested change", acceptanceCriteria: ["Tests pass"], allowedPaths: ["src"], protectedPaths: [".git"],
    workerAgentId: "codex-cli", verifierAgentId: "claude-code",
    verificationCommands: [{ id: "tests", program: "npm", args: ["test"], workingDirectory: null, timeoutSeconds: 60, required: true }],
    limits: { maxIterations: 3, stepTimeoutSeconds: 60, totalTimeoutSeconds: 600, maxConsecutiveRuntimeErrors: 2, maxConsecutiveNoProgress: 2 },
    version: 1, createdAt: "2026-08-21T00:00:00Z", updatedAt: "2026-08-21T00:00:00Z", ...overrides,
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
    completedAt: ["succeeded", "failed", "cancelled"].includes(status) ? "2026-08-21T00:02:00Z" : null, ...overrides,
  };
}

export const loopFixtureCases = {
  enabled: () => loopDefinitionFixture(),
  disabled: () => loopDefinitionFixture({ enabled: false }),
  unavailableSelection: () => loopDefinitionFixture({ projectPath: "D:/missing", baseBranch: "deleted-branch" }),
  activeRun: () => loopRunFixture("running", { phase: "acting", activeOperationId: "worker-operation" }),
  multiIteration: () => loopRunFixture("awaiting-acceptance", { currentIteration: 2, iterations: [loopIterationFixture(), loopIterationFixture({ id: "iteration-2", sequence: 2, diffFingerprint: "diff-2" })] }),
  recoveryRequired: () => loopRunFixture("paused", { terminalReason: "recovery-required", phase: "verifying" }),
  awaitingAcceptance: () => loopRunFixture("awaiting-acceptance"),
};
