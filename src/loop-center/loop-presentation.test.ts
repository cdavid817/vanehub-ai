import { describe, expect, it } from "vitest";
import { loopEvidenceFixture, loopIterationFixture, loopRunFixture } from "../test/loop-fixtures";
import {
  compareConsecutiveIterations,
  selectChangeStatistics,
  selectCurrentLoopActivity,
  selectLatestDecision,
  selectLoopBudget,
  selectRecoveryGuidance,
  selectRequiredCheckOutcomes,
} from "./loop-presentation";

describe("Loop presentation selectors", () => {
  it("derives activity, budget, latest decision, and recovery guidance", () => {
    const run = loopRunFixture("running", { startedAt: "2026-08-21T00:00:00Z" });
    expect(selectCurrentLoopActivity(run)).toBe("Worker completed.");
    expect(selectLoopBudget(run, Date.parse("2026-08-21T00:05:00Z"))).toEqual({
      elapsedMs: 300_000, remainingMs: 300_000, consumedPercent: 50, exhausted: false,
    });
    expect(selectLatestDecision(run)).toBe("Ready for acceptance.");
    expect(selectRecoveryGuidance(loopRunFixture("paused", { terminalReason: "recovery-required" }))).toBe("inspect");
  });

  it("keeps absent check and change evidence unknown", () => {
    const iteration = loopIterationFixture({ evidence: [] });
    const run = loopRunFixture("running", { iterations: [iteration] });
    expect(selectRequiredCheckOutcomes(run)).toEqual([{ commandId: "tests", outcome: "not-evaluated" }]);
    expect(selectChangeStatistics(iteration)).toBeNull();
  });

  it("compares failures and changes only when both iterations have durable evidence", () => {
    const previous = loopIterationFixture({ evidence: [
      loopEvidenceFixture({ commandId: "lint", kind: "verification", status: "failed", details: null }),
      loopEvidenceFixture({ id: "old-changes", details: { changedFiles: 2, additions: 8, deletions: 1 } }),
    ] });
    const current = loopIterationFixture({ evidence: [
      loopEvidenceFixture({ commandId: "test", kind: "verification", status: "failed", details: null }),
      loopEvidenceFixture({ id: "new-changes", details: { changedFiles: 3, additions: 12, deletions: 2 } }),
    ] });
    expect(compareConsecutiveIterations(previous, current)).toEqual({
      resolvedFailures: ["lint"], newFailures: ["test"],
      changeDelta: { changedFiles: 1, additions: 4, deletions: 1 },
    });
    expect(compareConsecutiveIterations(previous, { ...current, evidence: current.evidence.slice(0, 1) }).changeDelta).toBeNull();
  });
});

describe("verification evidence kinds", () => {
  it("reads required check outcomes from the native kind and the historical kind alike", () => {
    const native = loopRunFixture("awaiting-acceptance", { iterations: [loopIterationFixture({ evidence: [
      loopEvidenceFixture({ id: "native", kind: "verification-command", commandId: "tests", status: "passed", details: { required: true } }),
    ] })] });
    expect(selectRequiredCheckOutcomes(native)).toEqual([{ commandId: "tests", outcome: "passed" }]);
    const legacy = loopRunFixture("awaiting-acceptance", { iterations: [loopIterationFixture({ evidence: [
      loopEvidenceFixture({ id: "legacy", kind: "verification", commandId: "tests", status: "failed", details: null }),
    ] })] });
    expect(selectRequiredCheckOutcomes(legacy)).toEqual([{ commandId: "tests", outcome: "failed" }]);
    // Re-verification appends a fresh row for the same command; the latest one is the current result.
    const reverified = loopRunFixture("awaiting-acceptance", { iterations: [loopIterationFixture({ evidence: [
      loopEvidenceFixture({ id: "first", kind: "verification-command", commandId: "tests", status: "failed", details: null }),
      loopEvidenceFixture({ id: "second", kind: "verification-command", commandId: "tests", status: "passed", details: null }),
    ] })] });
    expect(selectRequiredCheckOutcomes(reverified)).toEqual([{ commandId: "tests", outcome: "passed" }]);
  });

  it("compares consecutive iterations across both kinds", () => {
    const previous = loopIterationFixture({ sequence: 1, evidence: [loopEvidenceFixture({ commandId: "lint", kind: "verification", status: "failed", details: null })] });
    const current = loopIterationFixture({ id: "iteration-2", sequence: 2, evidence: [loopEvidenceFixture({ commandId: "lint", kind: "verification-command", status: "passed", details: null })] });
    expect(compareConsecutiveIterations(previous, current).resolvedFailures).toEqual(["lint"]);
  });
});

