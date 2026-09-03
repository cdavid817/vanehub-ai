import { describe, expect, it } from "vitest";
import {
  agentFailureAttempt,
  artifactUnavailableAttempt,
  benchmarkErrorAttempt,
  cancelledAttempt,
  deterministicFailureAttempt,
  flakyAttemptPair,
  missingMetricsAttempt,
  passedAttempt,
  stuckAttempt,
  timedOutAttempt,
} from "./evaluation-attempt-fixtures";

describe("evaluation attempt fixtures (task 18.15)", () => {
  it.each([
    ["passedAttempt", passedAttempt, "succeeded"],
    ["deterministicFailureAttempt", deterministicFailureAttempt, "task_failed"],
    ["agentFailureAttempt", agentFailureAttempt, "agent_failed"],
    ["timedOutAttempt", timedOutAttempt, "timed_out"],
    ["stuckAttempt", stuckAttempt, "stuck"],
    ["cancelledAttempt", cancelledAttempt, "cancelled"],
    ["benchmarkErrorAttempt", benchmarkErrorAttempt, "benchmark_error"],
  ] as const)("%s carries the real outcome %s", (_name, build, outcome) => {
    expect(build().outcome).toBe(outcome);
  });

  it("returns a fresh object per call rather than a shared mutable reference", () => {
    const first = passedAttempt();
    const second = passedAttempt();
    expect(first).not.toBe(second);
    expect(first.metrics).not.toBe(second.metrics);
  });

  it("applies overrides on top of the named defaults rather than ignoring them", () => {
    const attempt = passedAttempt({ taskId: "custom-task", taskVersion: 3 });
    expect(attempt.taskId).toBe("custom-task");
    expect(attempt.taskVersion).toBe(3);
    // Everything not overridden keeps the named scenario's own real shape.
    expect(attempt.outcome).toBe("succeeded");
  });

  it("never sets judge on any fixture -- the field is confirmed dead, not fabricated", () => {
    for (const build of [passedAttempt, deterministicFailureAttempt, agentFailureAttempt, timedOutAttempt, stuckAttempt, cancelledAttempt, benchmarkErrorAttempt, missingMetricsAttempt, artifactUnavailableAttempt]) {
      expect(build().judge).toBeUndefined();
    }
  });

  describe("missingMetricsAttempt", () => {
    it("keeps every metric entry present with an unavailable quality and a null value, not an empty array", () => {
      const attempt = missingMetricsAttempt();
      expect(attempt.metrics.length).toBeGreaterThan(0);
      for (const metric of attempt.metrics) {
        expect(metric.quality).toBe("unavailable");
        expect(metric.value).toBeNull();
      }
    });
  });

  describe("artifactUnavailableAttempt", () => {
    it("carries real, non-empty artifactIds -- the one state a populated array renders as today", () => {
      const attempt = artifactUnavailableAttempt();
      expect(attempt.artifactIds.length).toBeGreaterThan(0);
    });
  });

  describe("flakyAttemptPair", () => {
    it("returns two attempts sharing task/version/Agent but landing on different outcomes", () => {
      const [first, second] = flakyAttemptPair();
      expect(first.taskId).toBe(second.taskId);
      expect(first.taskVersion).toBe(second.taskVersion);
      expect(first.agent.agentId).toBe(second.agent.agentId);
      expect(first.agent.configurationFingerprint).toBe(second.agent.configurationFingerprint);
      expect(first.outcome).not.toBe(second.outcome);
      expect(first.id).not.toBe(second.id);
    });
  });
});
