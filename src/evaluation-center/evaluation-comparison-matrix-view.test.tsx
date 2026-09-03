// @vitest-environment jsdom

import { render, screen, within } from "@testing-library/react";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage } from "../i18n";
import type { EvaluationAgentSnapshot, EvaluationAttempt, EvaluationMetric } from "../types/evaluation";
import { buildComparisonMatrix } from "./evaluation-comparison-matrix";
import { EvaluationComparisonMatrixView } from "./evaluation-comparison-matrix-view";

function agent(agentId: string, fingerprint = "fp"): EvaluationAgentSnapshot {
  return { agentId, providerId: agentId, modelId: null, interactionMode: "cli", configurationFingerprint: fingerprint };
}

function attempt(overrides: Partial<EvaluationAttempt> = {}): EvaluationAttempt {
  return {
    id: "attempt-a", arenaId: "arena-x", canonicalRunId: "run-a", taskId: "fix-null-auth-token", taskVersion: 1,
    agent: agent("claude-code"), outcome: "succeeded", checks: [], metrics: [], contextEvidenceManifestId: null, artifactIds: [], timeline: [],
    ...overrides,
  };
}

function metric(name: string, value: number | null, overrides: Partial<EvaluationMetric> = {}): EvaluationMetric {
  return { name, value, unit: "ms", quality: "reported", source: "harness", ...overrides };
}

describe("EvaluationComparisonMatrixView", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  it("renders the baseline column plus one column per candidate, aligned by attempt", () => {
    const baseline = attempt({ id: "baseline", agent: agent("claude-code") });
    const candidateA = attempt({ id: "candidate-a", agent: agent("codex-cli"), outcome: "task_failed" });
    const candidateB = attempt({ id: "candidate-b", agent: agent("opencode"), outcome: "agent_failed" });
    render(<EvaluationComparisonMatrixView matrix={buildComparisonMatrix(baseline, [candidateA, candidateB])} />);
    expect(screen.getByText(/Baseline: claude-code/)).toBeTruthy();
    expect(screen.getByText(/codex-cli/)).toBeTruthy();
    expect(screen.getByText(/opencode/)).toBeTruthy();
  });

  it("shows an outcome badge and a real verdict for every eligible candidate column", () => {
    const baseline = attempt({ id: "baseline", outcome: "task_failed" });
    const candidate = attempt({ id: "candidate", outcome: "succeeded" });
    render(<EvaluationComparisonMatrixView matrix={buildComparisonMatrix(baseline, [candidate])} />);
    expect(screen.getByText("Task failed")).toBeTruthy();
    expect(screen.getByText("Passed")).toBeTruthy();
    expect(screen.getByText("Improved")).toBeTruthy();
  });

  it("discloses the real reason once in an ineligible column's own header, and shows a dash for every one of its rows", () => {
    const baseline = attempt({ id: "baseline", metrics: [metric("duration", 1000)] });
    const ineligible = attempt({ id: "ineligible", taskId: "different-task" });
    render(<EvaluationComparisonMatrixView matrix={buildComparisonMatrix(baseline, [ineligible])} />);
    expect(within(screen.getByTestId("evaluation-comparison-matrix-ineligible")).getByText("Baseline and candidate ran different tasks.")).toBeTruthy();
    expect(screen.getAllByText("—").length).toBeGreaterThan(0);
  });

  it("renders same/different Agent configuration per column", () => {
    const baseline = attempt({ id: "baseline", agent: agent("claude-code", "fp-1") });
    const sameConfig = attempt({ id: "same", agent: agent("claude-code", "fp-1") });
    const diffConfig = attempt({ id: "diff", agent: agent("codex-cli", "fp-2") });
    render(<EvaluationComparisonMatrixView matrix={buildComparisonMatrix(baseline, [sameConfig, diffConfig])} />);
    expect(screen.getByText("Same Agent configuration.")).toBeTruthy();
    expect(screen.getByText("Different Agent configuration.")).toBeTruthy();
    // Baseline's own reference cell shows the raw fingerprint, not a same/different sentence.
    expect(screen.getByText("fp-1", { exact: false })).toBeTruthy();
  });

  it("renders a compared metric delta, an uncompared reason, and a not-in-column cell in the same row set", () => {
    const baseline = attempt({ id: "baseline", metrics: [metric("duration", 1000)] });
    // candidateA shares "duration" with baseline (compared); candidateB introduces "cost", which
    // never appears in candidateA's own diff against baseline, so candidateA's own "cost" cell must
    // read as not-in-column rather than a fabricated value.
    const candidateA = attempt({ id: "candidate-a", metrics: [metric("duration", 1500)] });
    const candidateB = attempt({ id: "candidate-b", metrics: [metric("cost", 3, { unit: "usd" })] });
    render(<EvaluationComparisonMatrixView matrix={buildComparisonMatrix(baseline, [candidateA, candidateB])} />);
    expect(screen.getByText(/1500 ms · Δ \+500/)).toBeTruthy();
    expect(screen.getByText("Not recorded on the baseline.")).toBeTruthy();
    expect(screen.getByText("Not measured for this experiment.")).toBeTruthy();
  });

  it("shows the baseline's own unavailable state for a metric it never recorded", () => {
    const baseline = attempt({ id: "baseline", metrics: [] });
    const candidate = attempt({ id: "candidate", metrics: [metric("duration", 500)] });
    render(<EvaluationComparisonMatrixView matrix={buildComparisonMatrix(baseline, [candidate])} />);
    expect(screen.getAllByText("Unavailable").length).toBeGreaterThan(0);
  });
});
