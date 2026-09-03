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

  it("marks the time budget exhausted once real elapsed time crosses the definition's own total timeout", () => {
    // 17.16: distinct from iteration-*count* exhaustion (loop-run-workspace.test.tsx's
    // "shows exhausted continuation..." case, which sets `maxIterations` and never touches this
    // selector at all) -- this exercises `selectLoopBudget`'s own time branch (`elapsedMs >=
    // totalMs`) specifically, with a real nonzero elapsed value crossing a real nonzero budget.
    // Same 600s fixture budget as the "not exhausted" case above, pushed 1s past the boundary --
    // not a `totalTimeoutSeconds: 0` degenerate case (loop-acceptance-panel.test.tsx's own
    // "exhausted warning" test), which is trivially exhausted at any elapsed time and so never
    // actually proves the crossing behaviour.
    const run = loopRunFixture("running", { startedAt: "2026-08-21T00:00:00Z" });
    expect(selectLoopBudget(run, Date.parse("2026-08-21T00:10:01Z"))).toEqual({
      elapsedMs: 601_000, remainingMs: 0, consumedPercent: 100, exhausted: true,
    });
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
