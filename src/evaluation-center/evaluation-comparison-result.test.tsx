// @vitest-environment jsdom

import { render, screen, within } from "@testing-library/react";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage } from "../i18n";
import type { EvaluationComparisonResult } from "./evaluation-comparison";
import { EvaluationComparisonResultView } from "./evaluation-comparison-result";

describe("EvaluationComparisonResultView", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  it("renders the not-comparable state with a real reason for each ineligibility cause", () => {
    for (const reason of ["sameAttempt", "differentTask", "differentVersion", "inProgress"] as const) {
      const { unmount } = render(<EvaluationComparisonResultView result={{ eligible: false, reason }} />);
      const view = within(screen.getByTestId("evaluation-comparison-ineligible"));
      expect(view.getByText("Not comparable")).toBeTruthy();
      expect(view.getByText(/Baseline/)).toBeTruthy();
      unmount();
    }
  });

  it("shows the outcome badges, the improved verdict, and a real reason sentence", () => {
    const result: EvaluationComparisonResult = {
      eligible: true,
      outcomeTier: { baselineOutcome: "task_failed", candidateOutcome: "succeeded", baselineTier: "hasFailures", candidateTier: "succeeded", verdict: "improved" },
      metrics: [],
      uncomparedMetrics: [],
      reliability: { available: false },
      evidence: { baselineChecksCount: 1, candidateChecksCount: 1, checksCountDelta: 0, baselineArtifactsCount: 0, candidateArtifactsCount: 0, artifactsCountDelta: 0 },
      sameAgentConfiguration: true,
    };
    render(<EvaluationComparisonResultView result={result} />);
    expect(screen.getByText("Task failed")).toBeTruthy();
    expect(screen.getByText("Passed")).toBeTruthy();
    expect(screen.getByText("Improved")).toBeTruthy();
    expect(screen.getByText("Outcome moved from Task failed to Passed.")).toBeTruthy();
  });

  it.each(["regressed", "unchanged", "notRankable"] as const)("labels the %s verdict distinctly from improved", (verdict) => {
    const result: EvaluationComparisonResult = {
      eligible: true,
      outcomeTier: { baselineOutcome: "succeeded", candidateOutcome: "succeeded", baselineTier: "succeeded", candidateTier: "succeeded", verdict },
      metrics: [], uncomparedMetrics: [], reliability: { available: false },
      evidence: { baselineChecksCount: 0, candidateChecksCount: 0, checksCountDelta: 0, baselineArtifactsCount: 0, candidateArtifactsCount: 0, artifactsCountDelta: 0 },
      sameAgentConfiguration: true,
    };
    render(<EvaluationComparisonResultView result={result} />);
    const labels: Record<string, string> = { regressed: "Regressed", unchanged: "Unchanged", notRankable: "Not rankable" };
    expect(screen.getByText(labels[verdict] as string)).toBeTruthy();
  });

  it("renders a metric delta line with the signed delta and percent change", () => {
    const result: EvaluationComparisonResult = {
      eligible: true,
      outcomeTier: { baselineOutcome: "succeeded", candidateOutcome: "succeeded", baselineTier: "succeeded", candidateTier: "succeeded", verdict: "unchanged" },
      metrics: [{ name: "duration", unit: "ms", baselineValue: 1000, candidateValue: 1200, delta: 200, percentChange: 20, baselineQuality: "reported", candidateQuality: "reported" }],
      uncomparedMetrics: [{ name: "tokens", reason: "missingOnCandidate" }],
      reliability: { available: false },
      evidence: { baselineChecksCount: 0, candidateChecksCount: 0, checksCountDelta: 0, baselineArtifactsCount: 0, candidateArtifactsCount: 0, artifactsCountDelta: 0 },
      sameAgentConfiguration: true,
    };
    render(<EvaluationComparisonResultView result={result} />);
    const metricRow = screen.getByTestId("evaluation-comparison-metric");
    expect(metricRow.textContent).toContain("duration");
    expect(metricRow.textContent).toContain("1000");
    expect(metricRow.textContent).toContain("1200");
    expect(metricRow.textContent).toContain("+200");
    expect(metricRow.textContent).toContain("+20");
    const uncomparedRow = screen.getByTestId("evaluation-comparison-uncompared-metric");
    expect(uncomparedRow.textContent).toContain("tokens");
    expect(uncomparedRow.textContent).toContain("Not recorded on the candidate.");
  });

  it("shows an honest unavailable message instead of a fabricated ratio when reliability has no data", () => {
    const result: EvaluationComparisonResult = {
      eligible: true,
      outcomeTier: { baselineOutcome: "succeeded", candidateOutcome: "succeeded", baselineTier: "succeeded", candidateTier: "succeeded", verdict: "unchanged" },
      metrics: [], uncomparedMetrics: [], reliability: { available: false },
      evidence: { baselineChecksCount: 0, candidateChecksCount: 0, checksCountDelta: 0, baselineArtifactsCount: 0, candidateArtifactsCount: 0, artifactsCountDelta: 0 },
      sameAgentConfiguration: true,
    };
    render(<EvaluationComparisonResultView result={result} />);
    expect(within(screen.getByTestId("evaluation-comparison-reliability")).getByText(/one side recorded no checks/)).toBeTruthy();
  });

  it("renders a real reliability percentage and verdict when both sides ran checks", () => {
    const result: EvaluationComparisonResult = {
      eligible: true,
      outcomeTier: { baselineOutcome: "succeeded", candidateOutcome: "succeeded", baselineTier: "succeeded", candidateTier: "succeeded", verdict: "unchanged" },
      metrics: [], uncomparedMetrics: [],
      reliability: { available: true, baselineRatio: 0.5, candidateRatio: 1, delta: 0.5, verdict: "improved" },
      evidence: { baselineChecksCount: 2, candidateChecksCount: 2, checksCountDelta: 0, baselineArtifactsCount: 0, candidateArtifactsCount: 0, artifactsCountDelta: 0 },
      sameAgentConfiguration: true,
    };
    render(<EvaluationComparisonResultView result={result} />);
    const reliability = within(screen.getByTestId("evaluation-comparison-reliability"));
    expect(reliability.getByText("50% → 100%")).toBeTruthy();
  });

  it("discloses matching and differing Agent configuration in plain text", () => {
    const base: EvaluationComparisonResult = {
      eligible: true,
      outcomeTier: { baselineOutcome: "succeeded", candidateOutcome: "succeeded", baselineTier: "succeeded", candidateTier: "succeeded", verdict: "unchanged" },
      metrics: [], uncomparedMetrics: [], reliability: { available: false },
      evidence: { baselineChecksCount: 0, candidateChecksCount: 0, checksCountDelta: 0, baselineArtifactsCount: 0, candidateArtifactsCount: 0, artifactsCountDelta: 0 },
      sameAgentConfiguration: true,
    };
    const { rerender } = render(<EvaluationComparisonResultView result={base} />);
    expect(screen.getByTestId("evaluation-comparison-configuration").textContent).toBe("Same Agent configuration.");
    rerender(<EvaluationComparisonResultView result={{ ...base, sameAgentConfiguration: false }} />);
    expect(screen.getByTestId("evaluation-comparison-configuration").textContent).toBe("Different Agent configuration.");
  });
});
