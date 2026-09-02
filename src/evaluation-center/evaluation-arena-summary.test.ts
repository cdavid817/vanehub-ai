import { describe, expect, it } from "vitest";
import type { EvaluationAgentSnapshot, EvaluationAttempt, EvaluationOutcome, EvaluationTask } from "../types/evaluation";
import { deriveAgentSet, deriveArenaState, deriveOutcomeTally, findTaskPrompt } from "./evaluation-arena-summary";

function agent(agentId: string, fingerprint = "fp"): EvaluationAgentSnapshot {
  return { agentId, providerId: agentId, modelId: null, interactionMode: "cli", configurationFingerprint: fingerprint };
}

function attempt(outcome: EvaluationOutcome, agentId = "agent-a", id = `attempt-${agentId}-${outcome}`): EvaluationAttempt {
  return {
    id, arenaId: "arena-x", canonicalRunId: `${id}-run`, taskId: "task-x", taskVersion: 1,
    agent: agent(agentId), outcome, checks: [], metrics: [], contextEvidenceManifestId: null, artifactIds: [], timeline: [],
  };
}

describe("deriveArenaState", () => {
  it("reports running when any attempt is still non-terminal, even alongside a failed attempt", () => {
    expect(deriveArenaState([attempt("task_failed"), attempt("running")])).toBe("running");
    expect(deriveArenaState([attempt("queued")])).toBe("running");
  });

  it("reports hasFailures once every attempt is terminal and at least one is a real failure verdict", () => {
    for (const outcome of ["task_failed", "agent_failed", "timed_out", "stuck", "benchmark_error"] as const) {
      expect(deriveArenaState([attempt("succeeded"), attempt(outcome)])).toBe("hasFailures");
    }
  });

  it("reports succeeded only when every attempt succeeded", () => {
    expect(deriveArenaState([attempt("succeeded"), attempt("succeeded", "agent-b")])).toBe("succeeded");
  });

  it("reports cancelled for an all-cancelled arena, distinct from hasFailures", () => {
    expect(deriveArenaState([attempt("cancelled"), attempt("cancelled", "agent-b")])).toBe("cancelled");
  });

  it("reports cancelled (not succeeded, not hasFailures) for a mix of succeeded and cancelled with zero real failures", () => {
    expect(deriveArenaState([attempt("succeeded"), attempt("cancelled", "agent-b")])).toBe("cancelled");
  });

  it("prioritizes hasFailures over the cancelled bucket when both are present", () => {
    expect(deriveArenaState([attempt("succeeded"), attempt("cancelled", "agent-b"), attempt("task_failed", "agent-c")])).toBe("hasFailures");
  });

  it("falls back to running (never a false succeeded) for the defensive empty-attempts case", () => {
    expect(deriveArenaState([])).toBe("running");
  });
});

describe("deriveAgentSet", () => {
  it("keeps one entry per distinct agentId", () => {
    const set = deriveAgentSet([attempt("succeeded", "agent-a"), attempt("succeeded", "agent-b")]);
    expect(set.map((item) => item.agentId)).toEqual(["agent-a", "agent-b"]);
  });

  it("de-duplicates two attempts against the same Agent, keeping the first-seen snapshot", () => {
    const first = attempt("succeeded", "agent-a", "attempt-1");
    first.agent = agent("agent-a", "fingerprint-first");
    const second = attempt("task_failed", "agent-a", "attempt-2");
    second.agent = agent("agent-a", "fingerprint-second");
    const set = deriveAgentSet([first, second]);
    expect(set).toHaveLength(1);
    expect(set[0].configurationFingerprint).toBe("fingerprint-first");
  });

  it("returns an empty array for an arena with no attempts", () => {
    expect(deriveAgentSet([])).toEqual([]);
  });
});

describe("deriveOutcomeTally", () => {
  it("counts each outcome and orders the tally best-first (passing first, cancelled last)", () => {
    const tally = deriveOutcomeTally([
      attempt("cancelled", "agent-a"),
      attempt("succeeded", "agent-b"),
      attempt("task_failed", "agent-c"),
      attempt("succeeded", "agent-d"),
    ]);
    expect(tally).toEqual([
      { outcome: "succeeded", count: 2 },
      { outcome: "task_failed", count: 1 },
      { outcome: "cancelled", count: 1 },
    ]);
  });

  it("returns a single entry for a uniform-outcome arena", () => {
    expect(deriveOutcomeTally([attempt("succeeded", "agent-a"), attempt("succeeded", "agent-b")]))
      .toEqual([{ outcome: "succeeded", count: 2 }]);
  });
});

describe("findTaskPrompt", () => {
  const tasks: EvaluationTask[] = [
    { id: "fix-null-auth-token", version: 1, category: "bugfix", prompt: "Fix null authentication token handling.", timeoutSeconds: 120, verifierProfiles: [] },
    { id: "fix-null-auth-token", version: 2, category: "bugfix", prompt: "Fix null authentication token handling, v2.", timeoutSeconds: 120, verifierProfiles: [] },
  ];

  it("finds the prompt for a matching task id and version", () => {
    expect(findTaskPrompt(tasks, "fix-null-auth-token", 1)).toBe("Fix null authentication token handling.");
    expect(findTaskPrompt(tasks, "fix-null-auth-token", 2)).toBe("Fix null authentication token handling, v2.");
  });

  it("returns null rather than falling back to a different version of the same id", () => {
    expect(findTaskPrompt(tasks, "fix-null-auth-token", 3)).toBeNull();
  });

  it("returns null for an id the catalog does not have at all", () => {
    expect(findTaskPrompt(tasks, "unknown-task", 1)).toBeNull();
  });
});
