import { describe, expect, it } from "vitest";
import { evidenceSessionIdSchema } from "../contracts/session-workspace-evidence-ids";
import type {
  EvidenceCoverageState,
  WorkspaceEvidenceSummary,
} from "../types/session-workspace-evidence";
import { workspaceTabBadges, type WorkspaceTabBadge } from "./workspace-evidence-badges";

const sessionId = evidenceSessionIdSchema.parse("session-a");

function summary(
  coverage: EvidenceCoverageState,
  counts: Partial<{
    unviewedFiles: number;
    running: number;
    failed: number;
    liveShells: number;
    newErrors: number;
    traceRunning: number;
    traceFailed: number;
    verificationFailed: number;
  }> = {},
): WorkspaceEvidenceSummary {
  return {
    sessionId,
    generatedAt: "2026-08-23T10:00:00.000Z",
    coverage: { state: coverage, reasonCodes: [], truncated: false },
    runState: { status: "running" },
    changes: { changedFiles: 0, unviewedFiles: counts.unviewedFiles ?? 0 },
    executionRecords: { running: counts.running ?? 0, failed: counts.failed ?? 0 },
    shells: { live: counts.liveShells ?? 0 },
    logs: { newErrors: counts.newErrors ?? 0 },
    traces: { running: counts.traceRunning ?? 0, failed: counts.traceFailed ?? 0 },
    verification: { passed: 0, failed: counts.verificationFailed ?? 0 },
    usage: { coverage: "complete" },
  };
}

/** Every badge the mapper can produce, so a zero cannot hide in a case nobody checked. */
function badgeValues(badges: Partial<Record<string, WorkspaceTabBadge>>): WorkspaceTabBadge[] {
  return Object.values(badges).flatMap((badge) => (badge === undefined ? [] : [badge]));
}

describe("workspace tab badges", () => {
  it("shows nothing at all when the summary cannot be read", () => {
    // A console that cannot answer must not report its own limitation as a property of the
    // session; six placeholders across the bar would say the session is uncertain, not the tool.
    expect(workspaceTabBadges(undefined, "loading")).toEqual({});
    expect(workspaceTabBadges(undefined, "unavailable")).toEqual({});
  });

  it("omits a badge for a count that is known to be zero", () => {
    const badges = workspaceTabBadges(summary("complete"), "ready");
    for (const badge of badgeValues(badges)) expect(badge).toEqual({ kind: "none" });
  });

  it("never renders an uncounted value as zero", () => {
    for (const coverage of ["indexing", "partial", "unavailable"] as const) {
      const badges = workspaceTabBadges(summary(coverage), "ready");
      for (const badge of badgeValues(badges)) {
        expect(badge.kind, coverage).toBe("unknown");
        // The whole point: `0` is a claim, and this state has no claim to make.
        expect(badge).not.toHaveProperty("count");
      }
    }
  });

  it("reads a positive count under partial coverage as a floor", () => {
    const badges = workspaceTabBadges(summary("partial", { liveShells: 2 }), "ready");
    expect(badges.shell).toEqual({ kind: "count", count: 2, tone: "neutral", atLeast: true });
    // Zero under partial coverage carries no information, so it stays unknown.
    expect(badges.logs).toEqual({ kind: "unknown", reason: "partial" });
  });

  it("counts what each tab is about", () => {
    const badges = workspaceTabBadges(
      summary("complete", {
        unviewedFiles: 4,
        running: 2,
        liveShells: 1,
        newErrors: 3,
        traceRunning: 5,
        verificationFailed: 0,
      }),
      "ready",
    );
    expect(badges.changes).toEqual({ kind: "count", count: 4, tone: "neutral", atLeast: false });
    expect(badges["terminal-history"]).toEqual({ kind: "count", count: 2, tone: "neutral", atLeast: false });
    expect(badges.shell).toEqual({ kind: "count", count: 1, tone: "neutral", atLeast: false });
    expect(badges.logs).toEqual({ kind: "count", count: 3, tone: "danger", atLeast: false });
    expect(badges.traces).toEqual({ kind: "count", count: 5, tone: "neutral", atLeast: false });
    expect(badges.report).toEqual({ kind: "none" });
  });

  it("lets a failure count outrank a running count", () => {
    const badges = workspaceTabBadges(
      summary("complete", { running: 7, failed: 2, traceRunning: 3, traceFailed: 1 }),
      "ready",
    );
    // Showing `running` alone would hide a tab whose work has already failed, and a badge has
    // room for one number.
    expect(badges["terminal-history"]).toEqual({ kind: "count", count: 2, tone: "danger", atLeast: false });
    expect(badges.traces).toEqual({ kind: "count", count: 1, tone: "danger", atLeast: false });
  });

  it("badges only the tabs that count something", () => {
    const badges = workspaceTabBadges(summary("complete", { running: 1 }), "ready");
    expect(Object.keys(badges).sort()).toEqual(
      ["changes", "logs", "report", "shell", "terminal-history", "traces"].sort(),
    );
  });
});
