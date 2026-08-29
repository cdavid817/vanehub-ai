import type {
  LoopDefinition,
  LoopEvidence,
  LoopEvidenceKind,
  LoopEvidenceStatus,
  LoopIteration,
  LoopRun,
  LoopRunPhase,
  LoopRunStatus,
  LoopTerminalReason,
  LoopVerificationCommand,
} from "../../types/loop";
import {
  DEFAULT_SEED,
  FIXTURE_RANGE_END_MS,
  FIXTURE_RANGE_START_MS,
  type SeededRandom,
  chance,
  createIdFactory,
  createSeededRandom,
  isoTimestamp,
  maybeLong,
  nextInt,
  offsetTimestamp,
  pick,
  pickWeighted,
  title,
  words,
} from "./seeded-random";

const STATUS_WEIGHTS: ReadonlyArray<readonly [LoopRunStatus, number]> = [
  ["succeeded", 35], ["failed", 15], ["cancelled", 8], ["running", 20], ["awaiting-acceptance", 10],
  ["paused", 8], ["queued", 4],
];

const TERMINAL: ReadonlySet<LoopRunStatus> = new Set(["succeeded", "failed", "cancelled"]);

function phaseFor(rng: SeededRandom, status: LoopRunStatus): LoopRunPhase {
  if (TERMINAL.has(status) || status === "awaiting-acceptance") return "finalizing";
  if (status === "queued") return "preparing";
  return pick(rng, ["acting", "verifying", "deciding"] as const);
}

function terminalReasonFor(rng: SeededRandom, status: LoopRunStatus): LoopTerminalReason | null {
  if (status === "succeeded") return "goal-met";
  if (status === "failed") return pick(rng, ["max-iterations", "verification-failed", "runtime-errors", "no-progress", "time-budget"] as const);
  if (status === "cancelled") return pick(rng, ["user-stopped", "user-rejected"] as const);
  if (status === "paused" && chance(rng, 0.5)) return "recovery-required";
  return null;
}

function buildVerificationCommands(rng: SeededRandom): LoopVerificationCommand[] {
  const count = nextInt(rng, 1, 4);
  return Array.from({ length: count }, (_unused, index) => ({
    id: `command-${index}`,
    program: pick(rng, ["npm", "cargo", "pytest"] as const),
    args: [pick(rng, ["test", "check", "lint"] as const)],
    workingDirectory: null,
    timeoutSeconds: nextInt(rng, 30, 900),
    required: chance(rng, 0.8),
  }));
}

function buildDefinition(rng: SeededRandom, nextDefinitionId: () => string): LoopDefinition {
  const createdAt = isoTimestamp(rng, FIXTURE_RANGE_START_MS, FIXTURE_RANGE_END_MS);
  return {
    id: nextDefinitionId(),
    name: maybeLong(rng, () => title(rng, 2, 6), () => title(rng, 200, 320)),
    enabled: chance(rng, 0.8),
    projectPath: `D:/workspace/${title(rng, 1, 2)}`,
    baseBranch: pick(rng, ["main", "develop"] as const),
    goal: words(rng, 10, 40),
    acceptanceCriteria: Array.from({ length: nextInt(rng, 1, 4) }, () => words(rng, 4, 12)),
    allowedPaths: ["src"],
    protectedPaths: [".git", "openspec/changes/archive"],
    workerAgentId: pick(rng, ["claude-code", "codex-cli", "opencode"] as const),
    verifierAgentId: pick(rng, ["claude-code", "codex-cli", "gemini-cli"] as const),
    verificationCommands: buildVerificationCommands(rng),
    limits: {
      maxIterations: nextInt(rng, 2, 8),
      stepTimeoutSeconds: nextInt(rng, 60, 600),
      totalTimeoutSeconds: nextInt(rng, 900, 7200),
      maxConsecutiveRuntimeErrors: nextInt(rng, 1, 4),
      maxConsecutiveNoProgress: nextInt(rng, 1, 4),
    },
    version: nextInt(rng, 1, 6),
    createdAt,
    updatedAt: offsetTimestamp(createdAt, 0, 1000 * 60 * 60 * 24 * 10, rng),
  };
}

function buildEvidence(rng: SeededRandom, runId: string, iterationId: string, index: number): LoopEvidence {
  const kind = pick<LoopEvidenceKind>(rng, ["worktree", "worker", "verification", "verifier", "decision", "recovery"]);
  const status = pick<LoopEvidenceStatus>(rng, ["passed", "failed", "blocked", "cancelled"]);
  return {
    id: `evidence-${runId}-${iterationId}-${index}`,
    runId,
    iterationId,
    kind,
    status,
    summary: words(rng, 6, 20),
    operationId: chance(rng, 0.6) ? `operation-${runId}-${index}` : null,
    commandId: kind === "verification" ? `command-${index % 3}` : null,
    exitCode: kind === "verification" ? pick(rng, [0, 0, 1] as const) : null,
    durationMs: nextInt(rng, 200, 120_000),
    details: null,
    createdAt: isoTimestamp(rng, FIXTURE_RANGE_START_MS, FIXTURE_RANGE_END_MS),
  };
}

function buildIteration(rng: SeededRandom, runId: string, sequence: number, status: LoopRunStatus): LoopIteration {
  const id = `iteration-${runId}-${sequence}`;
  const startedAt = isoTimestamp(rng, FIXTURE_RANGE_START_MS, FIXTURE_RANGE_END_MS);
  const finished = status !== "running" && status !== "queued";
  const evidenceCount = nextInt(rng, 1, 3);
  return {
    id,
    runId,
    sequence,
    status,
    workerSessionId: `worker-${runId}-${sequence}`,
    verifierSessionId: `verifier-${runId}-${sequence}`,
    workerSummary: words(rng, 10, 40),
    verifierRecommendation: finished ? pick(rng, ["pass", "revise", "blocked"] as const) : null,
    verifierFindings: chance(rng, 0.4) ? [words(rng, 5, 15)] : [],
    decisionReason: finished ? words(rng, 5, 20) : null,
    diffFingerprint: `diff-${runId}-${sequence}`,
    checkFailureFingerprint: status === "failed" ? `check-${runId}-${sequence}` : null,
    userFeedback: chance(rng, 0.15) ? words(rng, 5, 25) : null,
    evidence: Array.from({ length: evidenceCount }, (_unused, index) => buildEvidence(rng, runId, id, index)),
    startedAt,
    completedAt: finished ? offsetTimestamp(startedAt, 60_000, 1000 * 60 * 30, rng) : null,
  };
}

/**
 * `count` deterministic loop runs, each with a full nested `LoopDefinition` snapshot and one to
 * `limits.maxIterations` `LoopIteration`s (with `LoopEvidence`), covering every `LoopRunStatus`
 * including the "paused + recovery-required" combination used elsewhere in this repo's fixtures.
 */
export function generateLoopRuns(count: number, seed: number = DEFAULT_SEED): LoopRun[] {
  const rng = createSeededRandom(seed);
  const nextRunId = createIdFactory("loop-run");
  const nextDefinitionId = createIdFactory("loop-definition");
  const runs: LoopRun[] = [];

  for (let index = 0; index < count; index += 1) {
    const status = pickWeighted(rng, STATUS_WEIGHTS);
    const definition = buildDefinition(rng, nextDefinitionId);
    const runId = nextRunId();
    const terminal = TERMINAL.has(status);
    const iterationCount = status === "queued" ? 0 : nextInt(rng, 1, definition.limits.maxIterations + 1);
    const iterations = Array.from({ length: iterationCount }, (_unused, sequence) =>
      buildIteration(rng, runId, sequence + 1, sequence === iterationCount - 1 ? status : "succeeded"),
    );
    const createdAt = isoTimestamp(rng, FIXTURE_RANGE_START_MS, FIXTURE_RANGE_END_MS);
    const startedAt = status === "queued" ? null : offsetTimestamp(createdAt, 1000, 60_000, rng);
    const updatedAt = startedAt ? offsetTimestamp(startedAt, 60_000, 1000 * 60 * 60 * 6, rng) : createdAt;
    const hasWorktree = status !== "queued";

    runs.push({
      id: runId,
      definitionId: definition.id,
      definitionSnapshot: definition,
      status,
      phase: phaseFor(rng, status),
      terminalReason: terminalReasonFor(rng, status),
      currentIteration: iterations.length,
      consecutiveRuntimeErrors: nextInt(rng, 0, 3),
      consecutiveNoProgress: nextInt(rng, 0, 3),
      pauseRequested: status === "running" && chance(rng, 0.1),
      projectPath: definition.projectPath,
      worktreePath: hasWorktree ? `D:/worktrees/loop-${index}` : null,
      worktreeName: hasWorktree ? `loop-${index}` : null,
      worktreeBranch: hasWorktree ? `vanehub/loop-${index}` : null,
      activeOperationId: status === "running" ? `operation-${runId}` : null,
      iterations,
      simulated: chance(rng, 0.05),
      createdAt,
      startedAt,
      updatedAt,
      completedAt: terminal ? updatedAt : null,
    });
  }

  return runs;
}
