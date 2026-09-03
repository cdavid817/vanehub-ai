import type { EvaluationAgentSnapshot, EvaluationAttempt } from "../../types/evaluation";

/**
 * Task 18.15: deterministic, individually-reachable `EvaluationAttempt` fixtures, one named builder
 * per real state -- follows `workspace-fixtures.ts`'s own established convention (one function per
 * named scenario, `Partial<T>` overrides, a fresh object per call), distinct from this directory's
 * pre-existing `evaluation-fixtures.ts`, which is a bulk/scale generator (`generateEvaluationFixtures`,
 * task 0.9's 10,000-row structural test) with no named, single-attempt entry points of its own.
 *
 * 7 of the 10 states task 18.15 names map directly to a real `EvaluationOutcome` value (see
 * `evaluation.ts`): pass, deterministic failure, Agent failure, timeout, stuck, cancelled, and
 * benchmark error. "Missing metrics" and "artifact-unavailable" are real, buildable states of
 * `EvaluationMetric`/`artifactIds` -- see `missingMetricsAttempt`/`artifactUnavailableAttempt`'s own
 * doc comments for exactly what backs each. "Flaky" is not a real per-attempt field (see
 * `flakyAttemptPair`'s own doc comment) and is represented compositionally instead.
 */

function fixtureAgent(overrides: Partial<EvaluationAgentSnapshot> = {}): EvaluationAgentSnapshot {
  return {
    agentId: "claude-code", providerId: "anthropic", modelId: "claude-sonnet-5",
    interactionMode: "cli", configurationFingerprint: "evaluation-fixture-claude-code-v1",
    ...overrides,
  };
}

/** Shared scaffolding every named attempt below starts from -- one real task/version
 *  (`fix-null-auth-token` v1, the same task `web-evaluation-client.ts`'s own mock catalog uses,
 *  with a real 120s `timeoutSeconds`), one real Agent snapshot, and a passed deterministic check.
 *  Each named export below overrides only the fields that actually differ for its own state. */
function baseAttempt(overrides: Partial<EvaluationAttempt> = {}): EvaluationAttempt {
  return {
    id: "evaluation-fixture-attempt", arenaId: "evaluation-fixture-arena", canonicalRunId: "evaluation-fixture-attempt-run",
    taskId: "fix-null-auth-token", taskVersion: 1, agent: fixtureAgent(), outcome: "succeeded",
    checks: [{ checkId: "deterministic-tests", passed: true, summary: "42/42 tests passed." }],
    metrics: [
      { name: "duration", value: 12_000, unit: "ms", quality: "reported", source: "runtime" },
      { name: "input_tokens", value: 820, unit: "tokens", quality: "reported", source: "provider" },
      { name: "tool_calls", value: 6, unit: "count", quality: "reported", source: "runtime" },
    ],
    contextEvidenceManifestId: "evaluation-fixture-manifest", artifactIds: [],
    timeline: [
      { id: "prepare", kind: "lifecycle", label: "Clean fixture prepared", status: "completed" },
      { id: "tool", kind: "tool", label: "Patch applied", status: "completed" },
      { id: "verify", kind: "verification", label: "Deterministic verification", status: "passed" },
    ],
    ...overrides,
  };
}

/** A clean pass -- every deterministic check succeeded. */
export function passedAttempt(overrides: Partial<EvaluationAttempt> = {}): EvaluationAttempt {
  return baseAttempt({ id: "evaluation-fixture-passed", canonicalRunId: "evaluation-fixture-passed-run", outcome: "succeeded", ...overrides });
}

/** The Agent produced a solution, but it failed deterministic verification -- a real verdict on
 *  the task, distinct from the Agent never running at all (see `agentFailureAttempt`). */
export function deterministicFailureAttempt(overrides: Partial<EvaluationAttempt> = {}): EvaluationAttempt {
  return baseAttempt({
    id: "evaluation-fixture-task-failed", canonicalRunId: "evaluation-fixture-task-failed-run", outcome: "task_failed",
    checks: [{ checkId: "deterministic-tests", passed: false, summary: "41/42 tests passed; 1 assertion failed." }],
    timeline: [
      { id: "prepare", kind: "lifecycle", label: "Clean fixture prepared", status: "completed" },
      { id: "tool", kind: "tool", label: "Patch applied", status: "completed" },
      { id: "verify", kind: "verification", label: "Deterministic verification", status: "failed" },
    ],
    ...overrides,
  });
}

/** The Agent itself could not be dispatched -- mirrors `web-evaluation-client.ts`'s own
 *  `WEB_DISPATCH_DIAGNOSTIC`/`webDispatchFailedAttempt`: no metrics recorded, since the outcome
 *  never reaches verification either in the real native engine or in the mock. */
export function agentFailureAttempt(overrides: Partial<EvaluationAttempt> = {}): EvaluationAttempt {
  return baseAttempt({
    id: "evaluation-fixture-agent-failed", canonicalRunId: "evaluation-fixture-agent-failed-run", outcome: "agent_failed",
    checks: [{ checkId: "agent-dispatch", passed: false, summary: "evaluation Agent is not installed and available" }],
    metrics: [],
    timeline: [
      { id: "prepare", kind: "lifecycle", label: "Clean fixture prepared", status: "completed" },
      { id: "dispatch", kind: "lifecycle", label: "Canonical evaluation attempt", status: "agent_failed" },
    ],
    ...overrides,
  });
}

/** Ran out of the task's own `timeoutSeconds` (120s for this fixture's task) before finishing --
 *  verification never completed, so there are no checks to report. */
export function timedOutAttempt(overrides: Partial<EvaluationAttempt> = {}): EvaluationAttempt {
  return baseAttempt({
    id: "evaluation-fixture-timed-out", canonicalRunId: "evaluation-fixture-timed-out-run", outcome: "timed_out",
    checks: [], metrics: [{ name: "duration", value: 120_000, unit: "ms", quality: "reported", source: "runtime" }],
    timeline: [
      { id: "prepare", kind: "lifecycle", label: "Clean fixture prepared", status: "completed" },
      { id: "tool", kind: "tool", label: "Patch applied", status: "completed" },
      { id: "timeout", kind: "lifecycle", label: "Task timeout reached", status: "timed_out" },
    ],
    ...overrides,
  });
}

/** Stopped making progress before finishing or timing out -- distinct from `timedOutAttempt`,
 *  which ran the full timeout budget; this one gave up earlier, on its own. */
export function stuckAttempt(overrides: Partial<EvaluationAttempt> = {}): EvaluationAttempt {
  return baseAttempt({
    id: "evaluation-fixture-stuck", canonicalRunId: "evaluation-fixture-stuck-run", outcome: "stuck",
    checks: [], metrics: [{ name: "duration", value: 45_000, unit: "ms", quality: "reported", source: "runtime" }],
    timeline: [
      { id: "prepare", kind: "lifecycle", label: "Clean fixture prepared", status: "completed" },
      { id: "tool", kind: "tool", label: "Patch applied", status: "stuck" },
    ],
    ...overrides,
  });
}

/** An operator stopped this attempt -- a deliberate action, not a verdict on the task or the
 *  Agent (`evaluation-results-table.tsx`'s own `OUTCOME_TONE` comment: cancelled is "neither a
 *  pass nor a verdict"). */
export function cancelledAttempt(overrides: Partial<EvaluationAttempt> = {}): EvaluationAttempt {
  return baseAttempt({
    id: "evaluation-fixture-cancelled", canonicalRunId: "evaluation-fixture-cancelled-run", outcome: "cancelled",
    checks: [],
    timeline: [
      { id: "prepare", kind: "lifecycle", label: "Clean fixture prepared", status: "completed" },
      { id: "cancel", kind: "lifecycle", label: "Cancelled by operator", status: "cancelled" },
    ],
    ...overrides,
  });
}

/** An infrastructure or harness failure -- not a verdict on the Agent or the task
 *  (`evaluation-results-table.tsx`'s own `OUTCOME_TONE` comment). */
export function benchmarkErrorAttempt(overrides: Partial<EvaluationAttempt> = {}): EvaluationAttempt {
  return baseAttempt({
    id: "evaluation-fixture-benchmark-error", canonicalRunId: "evaluation-fixture-benchmark-error-run", outcome: "benchmark_error",
    checks: [], metrics: [],
    timeline: [
      { id: "prepare", kind: "lifecycle", label: "Clean fixture prepared", status: "completed" },
      { id: "harness", kind: "lifecycle", label: "Benchmark harness error", status: "benchmark_error" },
    ],
    ...overrides,
  });
}

/**
 * "Missing metrics": every metric entry is present but carries `quality: "unavailable"` and
 * `value: null`, deliberately not an empty `metrics` array. An empty array reads as "nothing was
 * ever measured"; this reads as "the harness expected and attempted to record these, but the
 * values are genuinely unavailable" -- exactly what `MetricQuality`'s own `"unavailable"` value
 * (`evaluation.ts`) exists to distinguish, and the more informative of the two real, reachable
 * shapes (`evaluation-results-table.tsx`'s own `formatMetric` already renders this as "—").
 */
export function missingMetricsAttempt(overrides: Partial<EvaluationAttempt> = {}): EvaluationAttempt {
  return baseAttempt({
    id: "evaluation-fixture-missing-metrics", canonicalRunId: "evaluation-fixture-missing-metrics-run", outcome: "succeeded",
    metrics: [
      { name: "duration", value: null, unit: "ms", quality: "unavailable", source: "runtime" },
      { name: "input_tokens", value: null, unit: "tokens", quality: "unavailable", source: "provider" },
    ],
    ...overrides,
  });
}

/**
 * "Artifact-unavailable": investigated before building, not assumed. There is no artifact
 * resolution service or a distinct "recorded but failed to resolve" signal anywhere in `src/` --
 * `evaluation-center.tsx`'s own detail pane (18.13) renders *every* non-empty `artifactIds` entry
 * as `availability="unavailable"` unconditionally (`EvidenceLink`), because no artifact-preview
 * navigation target exists anywhere in the app yet (18.13's own evidence in tasks.md). That makes a
 * populated `artifactIds` array itself the one real, honest "artifact-unavailable" state this app
 * currently has to offer -- not a special-cased failure, the *only* state a non-empty array can
 * render as today. This fixture therefore differs from `passedAttempt` only in carrying real
 * (non-empty) `artifactIds`.
 */
export function artifactUnavailableAttempt(overrides: Partial<EvaluationAttempt> = {}): EvaluationAttempt {
  return baseAttempt({
    id: "evaluation-fixture-artifact-unavailable", canonicalRunId: "evaluation-fixture-artifact-unavailable-run",
    artifactIds: ["evaluation-fixture-artifact-unavailable-diff-1", "evaluation-fixture-artifact-unavailable-diff-2"],
    ...overrides,
  });
}

/**
 * "Flaky": confirmed NOT a real per-attempt field before building, per task 18.9's own evidence in
 * tasks.md -- a real `flaky: bool` exists server-side (`evaluation_verifier.rs`) but is folded into
 * `outcome` and never persisted; `evaluation_api.rs::attempt_aggregate` hard-codes `flaky: false`
 * when reconstructing a *stored* attempt, so even the backend treats it as permanently unknown once
 * an attempt is read back. There is no `flaky` field on `EvaluationAttempt` to build a fixture for
 * without fabricating one the type does not have.
 *
 * Represented compositionally instead: two attempts sharing the same `taskId`/`taskVersion`/Agent
 * but landing on different outcomes across separate runs -- the real-world pattern a reader would
 * call "flaky" by looking at repeated results for the same task/Agent pair side by side (e.g. in
 * the results table or the comparison panel), not a property read off one attempt in isolation.
 * Deliberately takes no `overrides` param, unlike every named builder above: it returns a *pair*
 * whose two members must keep distinct `id`/`canonicalRunId`/`outcome` values, which a single flat
 * `Partial<EvaluationAttempt>` applied identically to both sides could silently collide on (e.g. a
 * caller-supplied `id` override would give both attempts the same id).
 */
export function flakyAttemptPair(): [EvaluationAttempt, EvaluationAttempt] {
  const agent = fixtureAgent();
  return [
    baseAttempt({ id: "evaluation-fixture-flaky-run-1", canonicalRunId: "evaluation-fixture-flaky-run-1", agent, outcome: "succeeded" }),
    baseAttempt({
      id: "evaluation-fixture-flaky-run-2", canonicalRunId: "evaluation-fixture-flaky-run-2", agent, outcome: "task_failed",
      checks: [{ checkId: "deterministic-tests", passed: false, summary: "41/42 tests passed; 1 assertion failed." }],
    }),
  ];
}
