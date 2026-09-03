import type { EvaluationAttempt, EvaluationMetric, EvaluationOutcome, MetricQuality } from "../types/evaluation";
import { deriveArenaState, type ArenaState } from "./evaluation-arena-summary";
import { checkRatio, OUTCOME_RANK } from "./evaluation-results-table";
import { TERMINAL_EVALUATION_OUTCOMES } from "./use-evaluation-query";

/**
 * 18.8/18.9: a pairwise comparison between two `EvaluationAttempt`s, one designated "baseline"
 * (the reference point) and one "candidate" (what changed). There is no stored/marked "baseline"
 * concept anywhere in this codebase -- grepped `src/`, `src-tauri/src/`, and design.md's own
 * Decision 15 page model (`Comparison > Baseline selector`) confirms this is a *reader-driven*
 * designation made at comparison time, not a persisted server concept. "Baseline" is therefore just
 * whichever of the two attempts the reader put in the baseline slot -- either attempt can play
 * either role; the picker (`evaluation-comparison-panel.tsx`) never restricts a choice, it only
 * reports honestly on it.
 */

export type ComparisonIneligibilityReason = "sameAttempt" | "differentTask" | "differentVersion" | "inProgress";

export type ComparisonEligibility =
  | { eligible: true }
  | { eligible: false; reason: ComparisonIneligibilityReason };

/**
 * The real, checkable compatibility rules (18.8), checked in priority order -- only the first
 * failing rule is reported so the reader is never shown a compound reason for what is, underneath,
 * one root cause:
 * 1. Comparing an attempt against itself is a degenerate pick, not a real comparison.
 * 2. Different `taskId` -- metrics/checks are defined per task; a duration or a check name from one
 *    task carries no meaning against a different task's own.
 * 3. Same task but different `taskVersion` -- a version can change the prompt, timeout, or verifier
 *    profiles (`EvaluationTask`'s own fields), so "same id" alone does not make two attempts
 *    comparable.
 * 4. Either attempt not yet terminal (`TERMINAL_EVALUATION_OUTCOMES`) -- an in-flight attempt has no
 *    stable verdict, metrics, or checks to diff against.
 *
 * `agent.configurationFingerprint` is deliberately *not* a gate here: the two most common real
 * comparisons -- two different Agents competing on one task, or the same Agent re-run after a
 * config change -- both need the comparison to work across *different* configurations, and a
 * same-Agent regression check needs it to work across the *same* configuration too. A hard gate
 * either way would break one of those two real use cases for no honest reason. It is instead
 * surfaced as disclosed context on every eligible result (`sameAgentConfiguration` below), the same
 * "stay selectable, disclose the real reason instead of blocking" choice `isEvaluationAgentIncompatible`
 * (18.5, `evaluation-agent-filters.ts`) already made for incompatible Agents.
 */
export function checkEligibility(baseline: EvaluationAttempt, candidate: EvaluationAttempt): ComparisonEligibility {
  if (baseline.id === candidate.id) return { eligible: false, reason: "sameAttempt" };
  if (baseline.taskId !== candidate.taskId) return { eligible: false, reason: "differentTask" };
  if (baseline.taskVersion !== candidate.taskVersion) return { eligible: false, reason: "differentVersion" };
  if (!TERMINAL_EVALUATION_OUTCOMES.has(baseline.outcome) || !TERMINAL_EVALUATION_OUTCOMES.has(candidate.outcome)) {
    return { eligible: false, reason: "inProgress" };
  }
  return { eligible: true };
}

export type DeltaVerdict = "improved" | "regressed" | "unchanged" | "notRankable";

export interface OutcomeTierDelta {
  baselineOutcome: EvaluationOutcome;
  candidateOutcome: EvaluationOutcome;
  baselineTier: ArenaState;
  candidateTier: ArenaState;
  verdict: DeltaVerdict;
}

/**
 * Reuses `evaluation-results-table.tsx`'s own `OUTCOME_RANK` (best-first order over all 9
 * outcomes, exported for this reuse) rather than a second invented ordering, and
 * `evaluation-arena-summary.ts`'s `deriveArenaState` (applied to a single-element list -- the
 * function is generic over `readonly EvaluationAttempt[]` with no special-casing that assumes more
 * than one) for the coarse tier label/tone already shown elsewhere on this page. `cancelled` is
 * excluded from ranking on purpose: `OUTCOME_RANK`'s own comment already establishes it "is neither
 * a pass nor a verdict on the Agent, not a rank between the two failure clusters"; ranking a
 * deliberate operator action against a real verdict would fabricate a preference this codebase has
 * never actually taken a position on, so a cancellation on either side makes the verdict
 * `notRankable` rather than a guessed direction.
 */
function outcomeTierDelta(baseline: EvaluationAttempt, candidate: EvaluationAttempt): OutcomeTierDelta {
  const baselineTier = deriveArenaState([baseline]);
  const candidateTier = deriveArenaState([candidate]);
  const baselineRank = OUTCOME_RANK[baseline.outcome];
  const candidateRank = OUTCOME_RANK[candidate.outcome];
  const verdict: DeltaVerdict = baseline.outcome === "cancelled" || candidate.outcome === "cancelled"
    ? "notRankable"
    : candidateRank < baselineRank ? "improved" : candidateRank > baselineRank ? "regressed" : "unchanged";
  return { baselineOutcome: baseline.outcome, candidateOutcome: candidate.outcome, baselineTier, candidateTier, verdict };
}

export interface MetricDelta {
  name: string;
  unit: string;
  baselineValue: number;
  candidateValue: number;
  delta: number;
  /** Null when the baseline value is 0 -- a percentage change from zero is undefined, not infinite. */
  percentChange: number | null;
  baselineQuality: MetricQuality;
  candidateQuality: MetricQuality;
}

export type UncomparedMetricReason = "missingOnBaseline" | "missingOnCandidate" | "unavailableQuality" | "unitMismatch";

export interface UncomparedMetric {
  name: string;
  reason: UncomparedMetricReason;
}

function firstByName(metrics: readonly EvaluationMetric[]): Map<string, EvaluationMetric> {
  const byName = new Map<string, EvaluationMetric>();
  for (const metric of metrics) if (!byName.has(metric.name)) byName.set(metric.name, metric);
  return byName;
}

/**
 * Diffs every metric name present on the baseline against the candidate's own same-named metric
 * (first-match-wins on a duplicate name, mirroring `evaluation-results-table.tsx`'s own
 * `findMetric`), plus reports any candidate-only metric name the other way. A metric pair is only
 * ever silently *classified*, never silently *dropped*: every name that could not be diffed lands in
 * `uncompared` with a real reason (missing on one side, an `unavailable` quality or a null value on
 * either side, or a unit mismatch under the same name), not omitted from the result entirely.
 */
function metricDeltas(baseline: EvaluationAttempt, candidate: EvaluationAttempt): { metrics: MetricDelta[]; uncompared: UncomparedMetric[] } {
  const metrics: MetricDelta[] = [];
  const uncompared: UncomparedMetric[] = [];
  const candidateByName = firstByName(candidate.metrics);
  const baselineNames = new Set<string>();

  for (const [name, baseMetric] of firstByName(baseline.metrics)) {
    baselineNames.add(name);
    const candidateMetric = candidateByName.get(name);
    if (!candidateMetric) { uncompared.push({ name, reason: "missingOnCandidate" }); continue; }
    if (baseMetric.unit !== candidateMetric.unit) { uncompared.push({ name, reason: "unitMismatch" }); continue; }
    if (baseMetric.quality === "unavailable" || candidateMetric.quality === "unavailable" || baseMetric.value == null || candidateMetric.value == null) {
      uncompared.push({ name, reason: "unavailableQuality" });
      continue;
    }
    const delta = candidateMetric.value - baseMetric.value;
    metrics.push({
      name, unit: baseMetric.unit, baselineValue: baseMetric.value, candidateValue: candidateMetric.value, delta,
      percentChange: baseMetric.value !== 0 ? (delta / baseMetric.value) * 100 : null,
      baselineQuality: baseMetric.quality, candidateQuality: candidateMetric.quality,
    });
  }
  for (const name of candidateByName.keys()) {
    if (!baselineNames.has(name)) uncompared.push({ name, reason: "missingOnBaseline" });
  }
  return { metrics, uncompared };
}

export type ReliabilityDelta =
  | { available: true; baselineRatio: number; candidateRatio: number; delta: number; verdict: "improved" | "regressed" | "unchanged" }
  | { available: false };

/**
 * "Reliability" honestly available at comparison time is check pass rate (`checkRatio`, exported
 * from `evaluation-results-table.tsx` and reused rather than reimplemented), not flakiness. A real
 * `flaky: bool` concept does exist server-side (`evaluation_verifier.rs::aggregate_verification`,
 * mirrored in `evaluation_engine.rs`), computed by re-running checks and diffing the two results --
 * but it is folded into `outcome` (a flaky result becomes `task_failed`) and never persisted or
 * serialized: `commands/evaluation/dto.rs`'s own `EvaluationAttempt` struct has no such field, and
 * `evaluation_api.rs::attempt_aggregate` hard-codes `flaky: false` when reconstructing a *stored*
 * attempt for ranking, meaning even the backend treats it as permanently unknown once an attempt is
 * read back. Check pass rate is what is actually left to report.
 */
function reliabilityDelta(baseline: EvaluationAttempt, candidate: EvaluationAttempt): ReliabilityDelta {
  const baselineRatio = checkRatio(baseline);
  const candidateRatio = checkRatio(candidate);
  // checkRatio's own -1 sentinel means "no checks ran at all", not a real 0% -- diffing it
  // arithmetically would report a fabricated multi-hundred-percentage-point swing.
  if (baselineRatio < 0 || candidateRatio < 0) return { available: false };
  const delta = candidateRatio - baselineRatio;
  return { available: true, baselineRatio, candidateRatio, delta, verdict: delta > 0 ? "improved" : delta < 0 ? "regressed" : "unchanged" };
}

export interface EvidenceDelta {
  baselineChecksCount: number;
  candidateChecksCount: number;
  checksCountDelta: number;
  baselineArtifactsCount: number;
  candidateArtifactsCount: number;
  artifactsCountDelta: number;
}

/**
 * Volume of evidence recorded, not its quality -- how much changed, deliberately without a
 * good/bad verdict: more checks or artifacts recorded is not inherently an improvement, it can just
 * as easily mean the harness captured more (or less) this run.
 */
function evidenceDelta(baseline: EvaluationAttempt, candidate: EvaluationAttempt): EvidenceDelta {
  return {
    baselineChecksCount: baseline.checks.length, candidateChecksCount: candidate.checks.length,
    checksCountDelta: candidate.checks.length - baseline.checks.length,
    baselineArtifactsCount: baseline.artifactIds.length, candidateArtifactsCount: candidate.artifactIds.length,
    artifactsCountDelta: candidate.artifactIds.length - baseline.artifactIds.length,
  };
}

export type EvaluationComparisonResult =
  | { eligible: false; reason: ComparisonIneligibilityReason }
  | {
      eligible: true;
      outcomeTier: OutcomeTierDelta;
      metrics: MetricDelta[];
      uncomparedMetrics: UncomparedMetric[];
      reliability: ReliabilityDelta;
      evidence: EvidenceDelta;
      /** Disclosed context, not a gate -- see `checkEligibility`'s own doc comment. */
      sameAgentConfiguration: boolean;
    };

export function compareEvaluationAttempts(baseline: EvaluationAttempt, candidate: EvaluationAttempt): EvaluationComparisonResult {
  const eligibility = checkEligibility(baseline, candidate);
  if (!eligibility.eligible) return eligibility;
  const { metrics, uncompared } = metricDeltas(baseline, candidate);
  return {
    eligible: true,
    outcomeTier: outcomeTierDelta(baseline, candidate),
    metrics,
    uncomparedMetrics: uncompared,
    reliability: reliabilityDelta(baseline, candidate),
    evidence: evidenceDelta(baseline, candidate),
    sameAgentConfiguration: baseline.agent.configurationFingerprint === candidate.agent.configurationFingerprint,
  };
}
