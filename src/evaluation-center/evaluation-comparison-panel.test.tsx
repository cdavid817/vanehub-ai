// @vitest-environment jsdom

import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage } from "../i18n";
import type { EvaluationAgentSnapshot, EvaluationAttempt } from "../types/evaluation";
import { EvaluationComparisonPanel } from "./evaluation-comparison-panel";

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

describe("EvaluationComparisonPanel", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  it("shows an honest empty state instead of a picker when fewer than two results exist", () => {
    render(<EvaluationComparisonPanel attempts={[attempt()]} />);
    expect(screen.getByText("Run at least two results before comparing them.")).toBeTruthy();
    expect(screen.queryByTestId("evaluation-comparison-baseline")).toBeNull();
  });

  it("lists every attempt in both pickers with a real label, and prompts until both are chosen", () => {
    const attempts = [attempt({ id: "a", agent: agent("claude-code"), outcome: "succeeded" }), attempt({ id: "b", agent: agent("codex-cli"), outcome: "task_failed" })];
    render(<EvaluationComparisonPanel attempts={attempts} />);
    const baseline = screen.getByTestId("evaluation-comparison-baseline") as HTMLSelectElement;
    const candidate = screen.getByTestId("evaluation-comparison-candidate") as HTMLSelectElement;
    expect(Array.from(baseline.options).map((option) => option.value)).toEqual(["", "a", "b"]);
    expect(within(baseline).getByText("claude-code · Passed · fix-null-auth-token v1")).toBeTruthy();
    expect(within(candidate).getByText("codex-cli · Task failed · fix-null-auth-token v1")).toBeTruthy();
    expect(screen.getByText("Choose a baseline and a candidate to compare.")).toBeTruthy();
  });

  it("renders the comparison once both a baseline and a candidate are chosen", () => {
    const attempts = [attempt({ id: "a", outcome: "task_failed" }), attempt({ id: "b", outcome: "succeeded" })];
    render(<EvaluationComparisonPanel attempts={attempts} />);
    fireEvent.change(screen.getByTestId("evaluation-comparison-baseline"), { target: { value: "a" } });
    fireEvent.change(screen.getByTestId("evaluation-comparison-candidate"), { target: { value: "b" } });
    expect(screen.getByTestId("evaluation-comparison-result")).toBeTruthy();
    expect(screen.getByText("Improved")).toBeTruthy();
  });

  it("does not restrict either dropdown, and reports the real reason once an ineligible pair is picked", () => {
    const attempts = [attempt({ id: "a" }), attempt({ id: "b", taskId: "different-task" })];
    render(<EvaluationComparisonPanel attempts={attempts} />);
    fireEvent.change(screen.getByTestId("evaluation-comparison-baseline"), { target: { value: "a" } });
    fireEvent.change(screen.getByTestId("evaluation-comparison-candidate"), { target: { value: "b" } });
    expect(within(screen.getByTestId("evaluation-comparison-ineligible")).getByText("Baseline and candidate ran different tasks.")).toBeTruthy();
  });
});
