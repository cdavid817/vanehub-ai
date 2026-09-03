import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import type { EvaluationAgentSnapshot, EvaluationAttempt, EvaluationMetric } from "../types/evaluation";
import { attemptOptionLabel, buildComparisonMatrix } from "./evaluation-comparison-matrix";

function agent(agentId: string, fingerprint = "fp"): EvaluationAgentSnapshot {
  return { agentId, providerId: agentId, modelId: null, interactionMode: "cli", configurationFingerprint: fingerprint };
}

function attempt(overrides: Partial<EvaluationAttempt> = {}): EvaluationAttempt {
  return {
    id: "attempt-a", arenaId: "arena-x", canonicalRunId: "run-a", taskId: "task-x", taskVersion: 1,
    agent: agent("agent-a"), outcome: "succeeded", checks: [], metrics: [], contextEvidenceManifestId: null, artifactIds: [], timeline: [],
    ...overrides,
  };
}

function metric(name: string, value: number | null, overrides: Partial<EvaluationMetric> = {}): EvaluationMetric {
  return { name, value, unit: "ms", quality: "reported", source: "harness", ...overrides };
}

describe("buildComparisonMatrix", () => {
  const baseline = attempt({ id: "baseline", metrics: [metric("duration", 1000), metric("tokens", 500)] });

  it("builds one column per candidate, each independently compared against the same baseline", () => {
    const candidateA = attempt({ id: "candidate-a", metrics: [metric("duration", 1200)] });
    const candidateB = attempt({ id: "candidate-b", outcome: "task_failed" });
    const matrix = buildComparisonMatrix(baseline, [candidateA, candidateB]);
    expect(matrix.baseline).toBe(baseline);
    expect(matrix.columns).toHaveLength(2);
    expect(matrix.columns[0]?.attempt).toBe(candidateA);
    expect(matrix.columns[0]?.result.eligible).toBe(true);
    expect(matrix.columns[1]?.attempt).toBe(candidateB);
    if (matrix.columns[1]?.result.eligible) expect(matrix.columns[1].result.outcomeTier.verdict).toBe("regressed");
  });

  it("unions metric row names across columns in first-seen order, not alphabetically", () => {
    const candidateA = attempt({ id: "candidate-a", metrics: [metric("tokens", 600)] });
    // Introduces a name absent from both the baseline and candidate-a: only candidate-b's own diff covers it.
    const candidateB = attempt({ id: "candidate-b", metrics: [metric("duration", 900), metric("cost", 2)] });
    const matrix = buildComparisonMatrix(baseline, [candidateA, candidateB]);
    expect(matrix.metricRows.map((row) => row.name)).toEqual(["duration", "tokens", "cost"]);
  });

  it("marks a row 'notInColumn' for a column whose own baseline/candidate pairing never mentions that name", () => {
    const candidateA = attempt({ id: "candidate-a", metrics: [metric("tokens", 600)] }); // no "cost" metric anywhere on this side
    const candidateB = attempt({ id: "candidate-b", metrics: [metric("cost", 2)] });
    const matrix = buildComparisonMatrix(baseline, [candidateA, candidateB]);
    const costRow = matrix.metricRows.find((row) => row.name === "cost");
    expect(costRow?.cells[0]).toEqual({ kind: "notInColumn" });
    expect(costRow?.cells[1]).toEqual({ kind: "uncompared", reason: "missingOnBaseline" });
  });

  it("puts baseline's own metric names first, in baseline's own order, ahead of any candidate-introduced name", () => {
    // baseline itself records duration then tokens (declared in that order above); candidateA only
    // shares "tokens", so if row order were driven by per-column compared/uncompared classification
    // instead of baseline's own order, "tokens" (compared) could wrongly sort ahead of "duration"
    // (uncompared on this column) -- this pins baseline order as the actual contract.
    const candidateA = attempt({ id: "candidate-a", metrics: [metric("tokens", 600)] });
    const matrix = buildComparisonMatrix(baseline, [candidateA]);
    expect(matrix.metricRows.map((row) => row.name)).toEqual(["duration", "tokens"]);
  });

  it("classifies a directly comparable metric as 'compared' with its real delta", () => {
    const candidateA = attempt({ id: "candidate-a", metrics: [metric("duration", 1200)] });
    const matrix = buildComparisonMatrix(baseline, [candidateA]);
    const durationRow = matrix.metricRows.find((row) => row.name === "duration");
    expect(durationRow?.cells[0]).toEqual({ kind: "compared", delta: expect.objectContaining({ baselineValue: 1000, candidateValue: 1200, delta: 200 }) });
  });

  it("contributes no row names of its own from an ineligible column, and keeps its own result reason on the column itself", () => {
    const ineligible = attempt({ id: "different-task", taskId: "task-y", metrics: [metric("latency_unique_to_ineligible", 1)] });
    const eligible = attempt({ id: "candidate-a", metrics: [metric("tokens", 700)] });
    const matrix = buildComparisonMatrix(baseline, [ineligible, eligible]);
    expect(matrix.columns[0]?.result).toEqual({ eligible: false, reason: "differentTask" });
    // "duration"/"tokens" come from baseline's own metrics regardless of any column; the
    // ineligible column's own "latency_unique_to_ineligible" name never appears anywhere.
    expect(matrix.metricRows.map((row) => row.name)).toEqual(["duration", "tokens"]);
    const durationRow = matrix.metricRows.find((row) => row.name === "duration");
    // The ineligible column gets "notInColumn" for every row, never a fabricated delta.
    expect(durationRow?.cells[0]).toEqual({ kind: "notInColumn" });
    expect(durationRow?.cells[1]).toEqual({ kind: "uncompared", reason: "missingOnCandidate" });
  });

  it("returns empty columns and metric rows for zero candidates", () => {
    const matrix = buildComparisonMatrix(baseline, []);
    expect(matrix.columns).toEqual([]);
    expect(matrix.metricRows).toEqual([]);
  });

  it("returns empty metric rows when every candidate is ineligible, not a row per baseline metric with empty cells", () => {
    const ineligible = attempt({ id: "different-task", taskId: "task-y" });
    const matrix = buildComparisonMatrix(baseline, [ineligible]);
    expect(matrix.metricRows).toEqual([]);
  });

  // 21.13 "comparison" budget: every case above uses 1-2 metrics, far below what a real harness
  // reports (evaluation-fixtures.ts's own `buildMetrics` picks from a 4-name pool per attempt, and
  // `generateEvaluationFixtures`'s own checks run into the dozens) -- this proves row union,
  // baseline-first ordering, and per-column classification all stay correct at a realistically busy
  // scale, not just the toy 2-3 name cases above. Deliberately still `MAX_ADDITIONAL_CANDIDATES` (2)
  // candidates, matching this component's own real 2-4-total-attempt cap (18.11) -- comparison is a
  // small-N operation between individual attempts, not the whole result list, so scaling candidate
  // *count* up would not reflect how this feature is actually used; scaling metric *count* per
  // attempt is the real axis worth stressing.
  it("stays correct at a realistic scale of many metrics per attempt, both shared and column-exclusive", () => {
    const manyBaselineMetrics = Array.from({ length: 20 }, (_unused, index) => metric(`metric-${index}`, index * 10));
    const busyBaseline = attempt({ id: "busy-baseline", metrics: manyBaselineMetrics });
    // Candidate A shares every baseline name plus 5 of its own; candidate B shares only half of
    // baseline's names plus a different 5 of its own -- a realistic "not every run reports every
    // metric" shape, not a clean superset/subset.
    const candidateA = attempt({
      id: "candidate-a",
      metrics: [...manyBaselineMetrics.map((item) => metric(item.name, item.value! + 5)), ...Array.from({ length: 5 }, (_unused, index) => metric(`candidate-a-only-${index}`, index))],
    });
    const candidateB = attempt({
      id: "candidate-b",
      metrics: [...manyBaselineMetrics.slice(0, 10).map((item) => metric(item.name, item.value! - 5)), ...Array.from({ length: 5 }, (_unused, index) => metric(`candidate-b-only-${index}`, index))],
    });

    const start = performance.now();
    const matrix = buildComparisonMatrix(busyBaseline, [candidateA, candidateB]);
    const elapsedMs = performance.now() - start;
    console.info(`buildComparisonMatrix 2-candidate x 20+metric build: ${elapsedMs.toFixed(2)}ms`);

    // Row count: 20 baseline names, then candidate-a's 5 exclusive names (first-seen), then
    // candidate-b's 5 exclusive names -- 30 total, no duplicates, no dropped name.
    expect(matrix.metricRows).toHaveLength(30);
    expect(matrix.metricRows.slice(0, 20).map((row) => row.name)).toEqual(manyBaselineMetrics.map((item) => item.name));
    expect(matrix.metricRows.slice(20, 25).map((row) => row.name)).toEqual(["candidate-a-only-0", "candidate-a-only-1", "candidate-a-only-2", "candidate-a-only-3", "candidate-a-only-4"]);
    expect(matrix.metricRows.slice(25, 30).map((row) => row.name)).toEqual(["candidate-b-only-0", "candidate-b-only-1", "candidate-b-only-2", "candidate-b-only-3", "candidate-b-only-4"]);

    // Spot-check classification stays correct per column at this scale: every baseline name is
    // "compared" for candidate-a (it reported all 20), only the first 10 are "compared" for
    // candidate-b (it reported only half), and each candidate's own exclusive names are
    // "notInColumn" for the other column, never a fabricated cell.
    const metric15Row = matrix.metricRows.find((row) => row.name === "metric-15");
    expect(metric15Row?.cells[0].kind).toBe("compared");
    expect(metric15Row?.cells[1].kind).toBe("uncompared");
    const candidateAOnlyRow = matrix.metricRows.find((row) => row.name === "candidate-a-only-0");
    expect(candidateAOnlyRow?.cells[0].kind).toBe("uncompared");
    expect(candidateAOnlyRow?.cells[1].kind).toBe("notInColumn");
  });
});

describe("attemptOptionLabel", () => {
  beforeAll(async () => {
    // Plain `i18n.changeLanguage("en")` is not enough here: "en" resources are lazy-loaded
    // (`activateAppLanguage`/`ensureAppLanguage`, `src/i18n/index.ts`) and only the default app
    // language ships preloaded, so an unloaded "en" would silently fall back to that default
    // instead of throwing -- `activateAppLanguage` is what every sibling test file in this
    // directory already uses to load it for real (e.g. `evaluation-comparison-panel.test.tsx`).
    await activateAppLanguage("en");
  });

  it("formats agent, translated outcome, task, and version", () => {
    const label = attemptOptionLabel(attempt({ agent: agent("claude-code"), outcome: "task_failed", taskId: "fix-null-auth-token", taskVersion: 2 }), i18n.getFixedT("en"));
    expect(label).toBe("claude-code · Task failed · fix-null-auth-token v2");
  });
});
