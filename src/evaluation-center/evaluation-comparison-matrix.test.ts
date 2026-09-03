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
