import { describe, expect, it } from "vitest";
import type { EvaluationAgentSnapshot, EvaluationAttempt, EvaluationCheck, EvaluationMetric, EvaluationOutcome } from "../types/evaluation";
import { checkEligibility, compareEvaluationAttempts } from "./evaluation-comparison";

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

function check(checkId: string, passed: boolean): EvaluationCheck {
  return { checkId, passed, summary: passed ? "ok" : "failed" };
}

describe("checkEligibility", () => {
  const baseline = attempt({ id: "baseline" });

  it("rejects comparing an attempt against itself", () => {
    expect(checkEligibility(baseline, baseline)).toEqual({ eligible: false, reason: "sameAttempt" });
  });

  it("rejects a different task id", () => {
    const candidate = attempt({ id: "candidate", taskId: "task-y" });
    expect(checkEligibility(baseline, candidate)).toEqual({ eligible: false, reason: "differentTask" });
  });

  it("rejects a different task version under the same task id", () => {
    const candidate = attempt({ id: "candidate", taskVersion: 2 });
    expect(checkEligibility(baseline, candidate)).toEqual({ eligible: false, reason: "differentVersion" });
  });

  it("rejects a non-terminal baseline or candidate", () => {
    const runningBaseline = attempt({ id: "baseline", outcome: "running" });
    const candidate = attempt({ id: "candidate" });
    expect(checkEligibility(runningBaseline, candidate)).toEqual({ eligible: false, reason: "inProgress" });
    expect(checkEligibility(baseline, attempt({ id: "candidate", outcome: "queued" }))).toEqual({ eligible: false, reason: "inProgress" });
  });

  it("prioritizes sameAttempt over every other rule", () => {
    // A degenerate self-pick that also happens to differ in task/version is still, first and
    // foremost, the same attempt -- id equality is checked before anything else.
    expect(checkEligibility(baseline, baseline)).toEqual({ eligible: false, reason: "sameAttempt" });
  });

  it("is eligible for the same task/version, both terminal, distinct attempts", () => {
    expect(checkEligibility(baseline, attempt({ id: "candidate", outcome: "task_failed" }))).toEqual({ eligible: true });
  });
});

describe("compareEvaluationAttempts / ineligible pairs", () => {
  it("returns the ineligibility reason instead of any deltas", () => {
    const baseline = attempt({ id: "baseline", taskId: "task-x" });
    const candidate = attempt({ id: "candidate", taskId: "task-y" });
    expect(compareEvaluationAttempts(baseline, candidate)).toEqual({ eligible: false, reason: "differentTask" });
  });
});

describe("compareEvaluationAttempts / outcome tier", () => {
  it("reports improved when the candidate's outcome ranks better than the baseline's", () => {
    const result = compareEvaluationAttempts(attempt({ id: "baseline", outcome: "task_failed" }), attempt({ id: "candidate", outcome: "succeeded" }));
    expect(result.eligible).toBe(true);
    if (!result.eligible) throw new Error("unreachable");
    expect(result.outcomeTier).toEqual({
      baselineOutcome: "task_failed", candidateOutcome: "succeeded",
      baselineTier: "hasFailures", candidateTier: "succeeded", verdict: "improved",
    });
  });

  it("reports regressed when the candidate's outcome ranks worse than the baseline's", () => {
    const result = compareEvaluationAttempts(attempt({ id: "baseline", outcome: "succeeded" }), attempt({ id: "candidate", outcome: "agent_failed" }));
    if (!result.eligible) throw new Error("unreachable");
    expect(result.outcomeTier.verdict).toBe("regressed");
  });

  it("reports unchanged for the same outcome on both sides", () => {
    const result = compareEvaluationAttempts(attempt({ id: "baseline", outcome: "succeeded" }), attempt({ id: "candidate", outcome: "succeeded" }));
    if (!result.eligible) throw new Error("unreachable");
    expect(result.outcomeTier.verdict).toBe("unchanged");
  });

  it.each<[EvaluationOutcome, EvaluationOutcome]>([["cancelled", "succeeded"], ["succeeded", "cancelled"], ["cancelled", "cancelled"]])(
    "reports notRankable rather than a guessed direction when either side is cancelled (%s -> %s)",
    (baselineOutcome, candidateOutcome) => {
      const result = compareEvaluationAttempts(attempt({ id: "baseline", outcome: baselineOutcome }), attempt({ id: "candidate", outcome: candidateOutcome }));
      if (!result.eligible) throw new Error("unreachable");
      expect(result.outcomeTier.verdict).toBe("notRankable");
    },
  );
});

describe("compareEvaluationAttempts / metric deltas", () => {
  it("diffs a metric present with a real value and reported quality on both sides", () => {
    const baseline = attempt({ id: "baseline", metrics: [metric("duration", 1000)] });
    const candidate = attempt({ id: "candidate", metrics: [metric("duration", 1200)] });
    const result = compareEvaluationAttempts(baseline, candidate);
    if (!result.eligible) throw new Error("unreachable");
    expect(result.metrics).toEqual([{
      name: "duration", unit: "ms", baselineValue: 1000, candidateValue: 1200, delta: 200, percentChange: 20,
      baselineQuality: "reported", candidateQuality: "reported",
    }]);
    expect(result.uncomparedMetrics).toEqual([]);
  });

  it("reports a null percentChange rather than Infinity when the baseline value is zero", () => {
    const baseline = attempt({ id: "baseline", metrics: [metric("retries", 0)] });
    const candidate = attempt({ id: "candidate", metrics: [metric("retries", 3)] });
    const result = compareEvaluationAttempts(baseline, candidate);
    if (!result.eligible) throw new Error("unreachable");
    expect(result.metrics[0]?.percentChange).toBeNull();
    expect(result.metrics[0]?.delta).toBe(3);
  });

  it("classifies a metric missing on the candidate, missing on the baseline, unavailable quality, and a unit mismatch", () => {
    const baseline = attempt({
      id: "baseline",
      metrics: [metric("only_baseline", 1), metric("unit_mismatch", 5, { unit: "ms" }), metric("unavailable_side", null, { quality: "unavailable" })],
    });
    const candidate = attempt({
      id: "candidate",
      metrics: [metric("only_candidate", 1), metric("unit_mismatch", 5, { unit: "s" }), metric("unavailable_side", 9)],
    });
    const result = compareEvaluationAttempts(baseline, candidate);
    if (!result.eligible) throw new Error("unreachable");
    expect(result.metrics).toEqual([]);
    expect(result.uncomparedMetrics).toEqual(expect.arrayContaining([
      { name: "only_baseline", reason: "missingOnCandidate" },
      { name: "only_candidate", reason: "missingOnBaseline" },
      { name: "unit_mismatch", reason: "unitMismatch" },
      { name: "unavailable_side", reason: "unavailableQuality" },
    ]));
    expect(result.uncomparedMetrics).toHaveLength(4);
  });

  it("keeps only the first-seen metric when a name repeats on one side, mirroring findMetric", () => {
    const baseline = attempt({ id: "baseline", metrics: [metric("tokens", 100), metric("tokens", 999)] });
    const candidate = attempt({ id: "candidate", metrics: [metric("tokens", 150)] });
    const result = compareEvaluationAttempts(baseline, candidate);
    if (!result.eligible) throw new Error("unreachable");
    expect(result.metrics).toEqual([expect.objectContaining({ name: "tokens", baselineValue: 100, candidateValue: 150 })]);
  });
});

describe("compareEvaluationAttempts / reliability", () => {
  it("computes a check-pass-rate delta and verdict when both sides ran checks", () => {
    const baseline = attempt({ id: "baseline", checks: [check("a", true), check("b", false)] });
    const candidate = attempt({ id: "candidate", checks: [check("a", true), check("b", true)] });
    const result = compareEvaluationAttempts(baseline, candidate);
    if (!result.eligible) throw new Error("unreachable");
    expect(result.reliability).toEqual({ available: true, baselineRatio: 0.5, candidateRatio: 1, delta: 0.5, verdict: "improved" });
  });

  it("reports regressed when the candidate's pass rate is lower", () => {
    const baseline = attempt({ id: "baseline", checks: [check("a", true)] });
    const candidate = attempt({ id: "candidate", checks: [check("a", false)] });
    const result = compareEvaluationAttempts(baseline, candidate);
    if (!result.eligible) throw new Error("unreachable");
    expect(result.reliability).toEqual({ available: true, baselineRatio: 1, candidateRatio: 0, delta: -1, verdict: "regressed" });
  });

  it("is unavailable, not a fabricated ratio, when either side ran zero checks", () => {
    const baseline = attempt({ id: "baseline", checks: [] });
    const candidate = attempt({ id: "candidate", checks: [check("a", true)] });
    const result = compareEvaluationAttempts(baseline, candidate);
    if (!result.eligible) throw new Error("unreachable");
    expect(result.reliability).toEqual({ available: false });
  });
});

describe("compareEvaluationAttempts / evidence and configuration", () => {
  it("computes checks and artifact count deltas without a good/bad verdict", () => {
    const baseline = attempt({ id: "baseline", checks: [check("a", true)], artifactIds: ["x"] });
    const candidate = attempt({ id: "candidate", checks: [check("a", true), check("b", true)], artifactIds: [] });
    const result = compareEvaluationAttempts(baseline, candidate);
    if (!result.eligible) throw new Error("unreachable");
    expect(result.evidence).toEqual({
      baselineChecksCount: 1, candidateChecksCount: 2, checksCountDelta: 1,
      baselineArtifactsCount: 1, candidateArtifactsCount: 0, artifactsCountDelta: -1,
    });
  });

  it("discloses, rather than gates on, matching or differing Agent configuration", () => {
    const same = compareEvaluationAttempts(
      attempt({ id: "baseline", agent: agent("agent-a", "fp-1") }),
      attempt({ id: "candidate", agent: agent("agent-a", "fp-1") }),
    );
    const different = compareEvaluationAttempts(
      attempt({ id: "baseline", agent: agent("agent-a", "fp-1") }),
      attempt({ id: "candidate", agent: agent("agent-b", "fp-2") }),
    );
    if (!same.eligible || !different.eligible) throw new Error("unreachable");
    expect(same.sameAgentConfiguration).toBe(true);
    expect(different.sameAgentConfiguration).toBe(false);
  });
});
