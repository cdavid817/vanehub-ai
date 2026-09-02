import type { StatusTone } from "../ui/status/StatusBadge";
import type { EvaluationAgentSnapshot, EvaluationAttempt, EvaluationOutcome, EvaluationTask } from "../types/evaluation";
import { TERMINAL_EVALUATION_OUTCOMES } from "./use-evaluation-query";

/**
 * 18.3: pure arena-level (one experiment = one `EvaluationArena`) aggregation over its own
 * `attempts[]`, kept separate from `evaluation-arena-list.tsx` so the derivation rules are
 * unit-testable without mounting React. Several constants below intentionally mirror --
 * rather than import -- `evaluation-results-table.tsx`'s own `OUTCOME_TONE`/`OUTCOME_RANK`
 * categorization, since that file is frozen for this pass; comments cross-reference the source of
 * truth so the two coexisting lists keep reading consistently if either changes.
 */

export type ArenaState = "running" | "hasFailures" | "succeeded" | "cancelled";

// Mirrors evaluation-results-table.tsx's OUTCOME_TONE danger/warning/attention buckets: a real
// failure verdict on the task or the Agent, as opposed to still-in-flight (queued/running) or a
// deliberate operator action (cancelled) that is neither a pass nor a verdict on the Agent.
const FAILURE_OUTCOMES = new Set<EvaluationOutcome>(["task_failed", "agent_failed", "timed_out", "stuck", "benchmark_error"]);

// Mirrors evaluation-results-table.tsx's own OUTCOME_TONE map verbatim (tone per outcome value),
// reused here for the per-outcome tally badges so the two lists render the same outcome in the
// same color.
export const OUTCOME_TONE: Record<EvaluationOutcome, StatusTone> = {
  queued: "neutral",
  running: "running",
  succeeded: "success",
  task_failed: "danger",
  agent_failed: "danger",
  timed_out: "warning",
  stuck: "warning",
  cancelled: "neutral",
  benchmark_error: "attention",
};

// Best-first, matching evaluation-results-table.tsx's OUTCOME_RANK reading order: a passing
// result first, cancelled last since it is neither a pass nor a rank between the two failure
// clusters.
const OUTCOME_DISPLAY_ORDER: readonly EvaluationOutcome[] = [
  "succeeded", "running", "queued", "timed_out", "stuck", "task_failed", "agent_failed", "benchmark_error", "cancelled",
];

export const ARENA_STATE_TONE: Record<ArenaState, StatusTone> = {
  running: "running",
  hasFailures: "danger",
  succeeded: "success",
  cancelled: "neutral",
};

/**
 * i18n key for each state's *value* text. Reuses `evaluation.outcome.*` for three of the four --
 * deliberately not a parallel vocabulary -- because the badge rendering this value is always
 * prefixed with the "State" field name (see evaluation-arena-list.tsx), so the full badge text can
 * never collide, as bare text, with an attempt-level outcome badge shown elsewhere on the same
 * page. "hasFailures" has no attempt-level equivalent (a single attempt cannot itself be "partly
 * failed"), so it gets its own dedicated key.
 */
export const ARENA_STATE_LABEL_KEY: Record<ArenaState, string> = {
  running: "evaluation.outcome.running",
  hasFailures: "evaluation.state.hasFailures",
  succeeded: "evaluation.outcome.succeeded",
  cancelled: "evaluation.outcome.cancelled",
};

/**
 * State-derivation rule -- a real design decision, made explicitly:
 * 1. Any attempt still non-terminal (queued/running) -> "running", even if another attempt in the
 *    same arena already failed: the experiment is not done, and Cancel is still meaningful for it
 *    (mirrors evaluation-center.tsx's own per-attempt `TERMINAL_EVALUATION_OUTCOMES` check that
 *    gates the Cancel button).
 * 2. All attempts terminal and any of them is a real failure verdict (`FAILURE_OUTCOMES`) ->
 *    "hasFailures".
 * 3. All attempts terminal, none failed, and every one of them succeeded -> "succeeded".
 * 4. All attempts terminal, none failed, but at least one was cancelled (mixed
 *    succeeded+cancelled, or entirely cancelled) -> "cancelled". Kept distinct from "hasFailures"
 *    because cancellation is a deliberate operator action, not a verdict on the task or the Agent
 *    -- the same distinction `evaluation-results-table.tsx`'s own `OUTCOME_TONE` already draws
 *    (`cancelled: "neutral"`, never grouped with the danger/warning/attention failure tones).
 *
 * Zero attempts is not a shape the backend ever produces (`EvaluationApi::start` in
 * `evaluation_api.rs` rejects an empty `agent_ids` and pushes every attempt before returning) --
 * the empty-array branch below is a defensive fallback, not an observed state, and deliberately
 * the most conservative label ("running": nothing to report yet) rather than a false "succeeded"
 * (which `attempts.every(...)` would otherwise vacuously return for an empty array).
 */
export function deriveArenaState(attempts: readonly EvaluationAttempt[]): ArenaState {
  if (attempts.length === 0) return "running";
  if (attempts.some((attempt) => !TERMINAL_EVALUATION_OUTCOMES.has(attempt.outcome))) return "running";
  if (attempts.some((attempt) => FAILURE_OUTCOMES.has(attempt.outcome))) return "hasFailures";
  if (attempts.every((attempt) => attempt.outcome === "succeeded")) return "succeeded";
  return "cancelled";
}

/**
 * De-duplicated by `agentId`, first-attempt-wins. Not a theoretical precaution: neither
 * `StartEvaluationInput.agentIds` nor the Rust loop that consumes it
 * (`evaluation_api.rs::EvaluationApi::start`, `for agent_id in request.agent_ids { ...
 * arena.attempts.push(attempt) }`) rejects a repeated Agent id, and the existing scale fixture
 * generator (`testing/fixtures/evaluation-fixtures.ts`'s `buildAgentSnapshot`) already assumes it
 * can happen -- it picks an Agent independently at random for every attempt, with no
 * without-replacement guarantee across attempts in the same arena. Today's only production entry
 * point (`EvaluationCenter`'s `toggleAgent`) happens to prevent it via checkbox toggle semantics,
 * but nothing in the type or service contract guarantees that stays true.
 */
export function deriveAgentSet(attempts: readonly EvaluationAttempt[]): EvaluationAgentSnapshot[] {
  const byAgentId = new Map<string, EvaluationAgentSnapshot>();
  for (const attempt of attempts) {
    if (!byAgentId.has(attempt.agent.agentId)) byAgentId.set(attempt.agent.agentId, attempt.agent);
  }
  return [...byAgentId.values()];
}

export interface OutcomeTallyEntry {
  outcome: EvaluationOutcome;
  count: number;
}

/** Grouped counts per distinct outcome present, in best-first reading order -- not attempt array
 *  order, which carries no meaning of its own (the ranking-version sort applies to attempts within
 *  one arena, not to which arena the backend happens to return first). */
export function deriveOutcomeTally(attempts: readonly EvaluationAttempt[]): OutcomeTallyEntry[] {
  const counts = new Map<EvaluationOutcome, number>();
  for (const attempt of attempts) counts.set(attempt.outcome, (counts.get(attempt.outcome) ?? 0) + 1);
  return OUTCOME_DISPLAY_ORDER.filter((outcome) => counts.has(outcome)).map((outcome) => ({ outcome, count: counts.get(outcome) as number }));
}

/**
 * Cross-references `EvaluationArena.taskId`/`.taskVersion` against the fetched task catalog for a
 * human-readable label beyond the raw id -- `EvaluationTask.prompt` (e.g. "Fix null authentication
 * token handling."), not `category`, since `prompt` is task-specific while `category` is only an
 * 8-value coarse bucket already implied by context. Returns null (never the raw id, never an empty
 * string) when the catalog has no matching task/version, so the caller can omit the subtitle
 * honestly instead of repeating the id or fabricating a label.
 */
export function findTaskPrompt(tasks: readonly EvaluationTask[], taskId: string, taskVersion: number): string | null {
  return tasks.find((task) => task.id === taskId && task.version === taskVersion)?.prompt ?? null;
}
