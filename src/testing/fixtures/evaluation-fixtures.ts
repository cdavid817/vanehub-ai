import type {
  EvaluationAgentSnapshot,
  EvaluationArena,
  EvaluationAttempt,
  EvaluationCategory,
  EvaluationCheck,
  EvaluationMetric,
  EvaluationOutcome,
  EvaluationTask,
  EvaluationTimelineItem,
} from "../../types/evaluation";
import {
  DEFAULT_SEED,
  type SeededRandom,
  chance,
  createIdFactory,
  createSeededRandom,
  distributeExact,
  nextInt,
  pick,
  pickWeighted,
  words,
} from "./seeded-random";

/**
 * Fixture-only convenience row -- not a production contract. `EvaluationCheck` (a real domain
 * type) carries no context of its own; this pairs one with the attempt/arena/task ids it came
 * from, so "10,000 evaluation result rows" can be rendered as a flat table without every consumer
 * re-deriving that context by walking `EvaluationArena.attempts[].checks[]` itself.
 */
export interface EvaluationResultRow {
  arenaId: string;
  attemptId: string;
  taskId: string;
  agentId: string;
  outcome: EvaluationOutcome;
  check: EvaluationCheck;
}

export interface EvaluationFixtureSet {
  arenas: EvaluationArena[];
  resultRows: EvaluationResultRow[];
}

const CATEGORIES: readonly EvaluationCategory[] = [
  "bugfix", "feature", "refactor", "tests", "code_review", "tool_use", "context", "planning",
];

const OUTCOME_WEIGHTS: ReadonlyArray<readonly [EvaluationOutcome, number]> = [
  ["succeeded", 45], ["task_failed", 15], ["agent_failed", 10], ["timed_out", 8], ["cancelled", 7],
  ["stuck", 5], ["benchmark_error", 4], ["running", 4], ["queued", 2],
];

const AGENT_IDS = ["claude-code", "codex-cli", "opencode", "antigravity-cli", "gemini-cli"] as const;
const PROVIDERS: Record<(typeof AGENT_IDS)[number], string> = {
  "claude-code": "anthropic", "codex-cli": "openai", opencode: "opencode", "antigravity-cli": "google", "gemini-cli": "google",
};

function buildTasks(rng: SeededRandom, count: number): EvaluationTask[] {
  const nextId = createIdFactory("eval-task");
  return Array.from({ length: count }, () => ({
    id: nextId(),
    version: nextInt(rng, 1, 4),
    category: pick(rng, CATEGORIES),
    prompt: chance(rng, 0.1) ? words(rng, 220, 340) : words(rng, 15, 60),
    timeoutSeconds: nextInt(rng, 60, 1800),
    verifierProfiles: Array.from({ length: nextInt(rng, 1, 3) }, () => pick(rng, ["default", "strict", "lenient"] as const)),
  }));
}

function buildAgentSnapshot(rng: SeededRandom): EvaluationAgentSnapshot {
  const agentId = pick(rng, AGENT_IDS);
  return {
    agentId,
    providerId: PROVIDERS[agentId],
    modelId: chance(rng, 0.9) ? `${agentId}-model-${nextInt(rng, 1, 4)}` : null,
    interactionMode: pick(rng, ["cli", "native-desktop"] as const),
    configurationFingerprint: `fingerprint-${agentId}-${nextInt(rng, 1, 50)}`,
  };
}

function buildChecks(rng: SeededRandom, count: number): EvaluationCheck[] {
  return Array.from({ length: count }, (_unused, index) => ({
    checkId: `check-${index}`,
    passed: chance(rng, 0.75),
    summary: words(rng, 4, 16),
  }));
}

function buildMetrics(rng: SeededRandom): EvaluationMetric[] {
  const pool: ReadonlyArray<readonly [string, string]> = [
    ["duration_seconds", "s"], ["tokens_used", "tokens"], ["tool_calls", "count"], ["cost_usd", "usd"],
  ];
  return Array.from({ length: nextInt(rng, 1, pool.length + 1) }, () => {
    const [name, unit] = pick(rng, pool);
    const quality = pickWeighted(rng, [["reported", 60], ["estimated", 30], ["unavailable", 10]] as const);
    return { name, unit, quality, value: quality === "unavailable" ? null : Number((rng() * 5000).toFixed(2)), source: "fixture" };
  });
}

function buildTimeline(rng: SeededRandom): EvaluationTimelineItem[] {
  const kinds: ReadonlyArray<EvaluationTimelineItem["kind"]> = ["lifecycle", "tool", "context", "verification"];
  return Array.from({ length: nextInt(rng, 2, 7) }, (_unused, index) => ({
    id: `timeline-${index}`,
    kind: pick(rng, kinds),
    label: words(rng, 2, 6),
    status: pick(rng, ["ok", "error", "skipped"] as const),
  }));
}

/**
 * Builds `arenaCount` evaluation arenas containing `attemptCount` attempts in total, and flattens
 * every attempt's `checks[]` into exactly `resultRowCount` `EvaluationResultRow`s -- the "10,000
 * evaluation result rows" task 0.9 asks for. Attempt and check counts per bucket are uneven
 * (`distributeExact`), not a flat average, while the grand totals stay exact.
 */
export function generateEvaluationFixtures(
  resultRowCount: number,
  seed: number = DEFAULT_SEED,
  arenaCount = 300,
  attemptCount = 1000,
  taskCount = 25,
): EvaluationFixtureSet {
  const rng = createSeededRandom(seed);
  const tasks = buildTasks(rng, taskCount);
  const attemptsPerArena = distributeExact(rng, arenaCount, attemptCount, 2, 6);
  const checksPerAttempt = distributeExact(rng, attemptCount, resultRowCount, 4, 20);

  const nextArenaId = createIdFactory("eval-arena");
  const nextAttemptId = createIdFactory("eval-attempt");
  const arenas: EvaluationArena[] = [];
  const resultRows: EvaluationResultRow[] = [];
  let attemptCursor = 0;

  for (let arenaIndex = 0; arenaIndex < arenaCount; arenaIndex += 1) {
    const task = pick(rng, tasks);
    const arenaId = nextArenaId();
    const attempts: EvaluationAttempt[] = [];

    for (let a = 0; a < attemptsPerArena[arenaIndex]; a += 1) {
      const attemptId = nextAttemptId();
      const outcome = pickWeighted(rng, OUTCOME_WEIGHTS);
      const agent = buildAgentSnapshot(rng);
      const checks = buildChecks(rng, checksPerAttempt[attemptCursor]);
      attemptCursor += 1;

      attempts.push({
        id: attemptId,
        arenaId,
        canonicalRunId: `canonical-${attemptId}`,
        taskId: task.id,
        taskVersion: task.version,
        agent,
        outcome,
        checks,
        metrics: buildMetrics(rng),
        contextEvidenceManifestId: chance(rng, 0.5) ? `manifest-${attemptId}` : null,
        artifactIds: Array.from({ length: nextInt(rng, 0, 4) }, (_unused, artifactIndex) => `artifact-${attemptId}-${artifactIndex}`),
        timeline: buildTimeline(rng),
      });

      for (const check of checks) {
        resultRows.push({ arenaId, attemptId, taskId: task.id, agentId: agent.agentId, outcome, check });
      }
    }

    arenas.push({ id: arenaId, operationId: `operation-${arenaId}`, taskId: task.id, taskVersion: task.version, rankingVersion: "v1", attempts });
  }

  return { arenas, resultRows };
}
