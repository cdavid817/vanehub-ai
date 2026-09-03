import type { TFunction } from "i18next";
import type { EvaluationAttempt } from "../types/evaluation";
import { compareEvaluationAttempts, type EvaluationComparisonResult, type MetricDelta, type UncomparedMetricReason } from "./evaluation-comparison";

/**
 * 18.11: "2-4 experiment comparison with aligned...task rows" is built by running the existing,
 * already-tested pairwise `compareEvaluationAttempts` (18.8/18.9) once per additional candidate
 * against one reader-picked baseline, rather than inventing new N-way eligibility/delta math --
 * every rule this reuses (cancelled-is-notRankable, the -1 `checkRatio` sentinel, unit-mismatch
 * classification, the "disclose don't gate" treatment of `configurationFingerprint`, ...) keeps its
 * existing, already-tested behavior completely unchanged. `evaluation-comparison-panel.tsx` is
 * where the reader picks a baseline plus up to `MAX_ADDITIONAL_CANDIDATES` more candidates on top of
 * the pre-existing single-candidate flow (2 attempts minimum, unchanged; 4 maximum, new).
 *
 * "Immutable configuration snapshot": `EvaluationAgentSnapshot` (`agent` on every
 * `EvaluationAttempt`) is captured once, at attempt-creation time, and never mutated afterward --
 * confirmed by reading `web-evaluation-client.ts`: `webEvaluationAttempt`/`webDispatchFailedAttempt`
 * build the `agent` object exactly once, `cancelEvaluation`'s own update spreads only `outcome`
 * (`{ ...attempt, outcome: "cancelled" }`), and every service method returns a `structuredClone` of
 * its result. Nothing anywhere reassigns an attempt's `agent` after creation, so the existing type
 * already *is* the immutable snapshot this task asks for -- the matrix below (and its Configuration
 * row in `evaluation-comparison-matrix-view.tsx`) exposes it per column, it does not need to be
 * rebuilt as a new concept.
 */
export const MAX_ADDITIONAL_CANDIDATES = 2;

export interface ComparisonMatrixColumn {
  attempt: EvaluationAttempt;
  result: EvaluationComparisonResult;
}

export type ComparisonMatrixMetricCell =
  | { kind: "compared"; delta: MetricDelta }
  | { kind: "uncompared"; reason: UncomparedMetricReason }
  /** This metric name belongs to a *different* column's own baseline/candidate pairing -- this
   *  column is otherwise eligible, the name just never appeared in its own diff against baseline. */
  | { kind: "notInColumn" };

export interface ComparisonMatrixMetricRow {
  name: string;
  /** Aligned 1:1 with `ComparisonMatrix.columns` by array index -- `buildComparisonMatrix` builds
   *  both from the same `columns.map(...)`, so index `i` here always describes `columns[i]`. */
  cells: ComparisonMatrixMetricCell[];
}

export interface ComparisonMatrix {
  baseline: EvaluationAttempt;
  columns: ComparisonMatrixColumn[];
  metricRows: ComparisonMatrixMetricRow[];
}

/**
 * One column per candidate, each independently compared against the same baseline via the
 * unmodified `compareEvaluationAttempts`. Metric rows are baseline's own metric names first, in
 * baseline's own order (baseline is the one fixed reference point every column shares, so its
 * metrics anchor the row set regardless of which candidates happen to be picked), followed by any
 * additional name a candidate introduces that baseline never recorded (`missingOnBaseline`,
 * first-seen in column order). Every "compared" name is provably already a baseline name --
 * `metricDeltas` (`evaluation-comparison.ts`) only ever classifies a pair as "compared" by walking
 * baseline's own metric names -- so this single pass covers both. An ineligible column contributes
 * no row names of its own (there is nothing real to align it on) and reports its own reason instead
 * (rendered once, in that column's header) rather than a per-row repeat.
 */
export function buildComparisonMatrix(baseline: EvaluationAttempt, candidates: readonly EvaluationAttempt[]): ComparisonMatrix {
  const columns: ComparisonMatrixColumn[] = candidates.map((attempt) => ({ attempt, result: compareEvaluationAttempts(baseline, attempt) }));

  // Gated on at least one eligible column existing -- with zero candidates, or every candidate
  // ineligible, a row for each of baseline's own metric names would carry an empty `cells` array in
  // every column: nothing to align it against, so nothing worth a row.
  const rowNames: string[] = [];
  const seen = new Set<string>();
  if (columns.some((column) => column.result.eligible)) {
    for (const metric of baseline.metrics) if (!seen.has(metric.name)) { seen.add(metric.name); rowNames.push(metric.name); }
    for (const column of columns) {
      if (!column.result.eligible) continue;
      for (const item of column.result.uncomparedMetrics) if (!seen.has(item.name)) { seen.add(item.name); rowNames.push(item.name); }
    }
  }

  const metricRows: ComparisonMatrixMetricRow[] = rowNames.map((name) => ({
    name,
    cells: columns.map((column): ComparisonMatrixMetricCell => {
      if (!column.result.eligible) return { kind: "notInColumn" };
      const compared = column.result.metrics.find((metric) => metric.name === name);
      if (compared) return { kind: "compared", delta: compared };
      const uncompared = column.result.uncomparedMetrics.find((item) => item.name === name);
      return uncompared ? { kind: "uncompared", reason: uncompared.reason } : { kind: "notInColumn" };
    }),
  }));

  return { baseline, columns, metricRows };
}

/** Shared by the baseline/candidate picker dropdowns (`evaluation-comparison-panel.tsx`) and every
 *  matrix column header (`evaluation-comparison-matrix-view.tsx`) so the same attempt is named
 *  identically everywhere it appears on this page. */
export function attemptOptionLabel(attempt: EvaluationAttempt, t: TFunction): string {
  return t("evaluation.comparison.attemptOption", {
    agent: attempt.agent.agentId,
    outcome: t(`evaluation.outcome.${attempt.outcome}`),
    task: attempt.taskId,
    version: attempt.taskVersion,
  });
}
