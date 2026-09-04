// @vitest-environment jsdom

import { renderToStaticMarkup } from "react-dom/server";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, renderHook, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { getActivePendingTimerCount } from "../testing/resource-tracking";
import type { LoopEvidence, LoopRun } from "../types/loop";
import { LoopInspector } from "./loop-inspector";
import { formatLoopDuration, latestLoopOperationEvidence, useLoopElapsed } from "./loop-monitoring";
import { LoopTimeline } from "./loop-timeline";

describe("Loop monitoring", () => {
  afterEach(() => vi.useRealTimers());

  it("formats bounded elapsed durations", () => {
    expect(formatLoopDuration(65_000)).toBe("1:05");
    expect(formatLoopDuration(3_661_000)).toBe("1:01:01");
  });

  it("finds the latest evidence associated with an operation", () => {
    expect(latestLoopOperationEvidence(exampleRun())?.operationId).toBe("operation-verifier");
  });

  it("renders phase progress, limits, operation state, and expanded iteration evidence", () => {
    const run = exampleRun();
    const client = new QueryClient();
    const html = renderToStaticMarkup(
      <MemoryRouter>
        <QueryClientProvider client={client}><LoopTimeline refreshing run={run} /><LoopInspector loading={false} run={run} /></QueryClientProvider>
      </MemoryRouter>,
    );

    expect(html).toContain("正在刷新");
    expect(html).toContain("5:05");
    expect(html).toContain("1 / 3");
    expect(html).toContain("3 个文件，+48 / -12");
    expect(html).toContain("验证检查");
    expect(html).toContain("验证者审查");
    expect(html).toContain("Required checks passed.");
    expect(html).toContain("operation-verifier");
    expect(html).toContain("连续运行错误限制");
    // The only (and therefore latest) iteration auto-expands -- task 17.10 replaced the former
    // native `<details open>` accordion with a controlled row toggle, so the equivalent check is
    // this attribute on the row's own button rather than a `<details>` boolean attribute.
    expect(html).toContain('aria-expanded="true"');
  });

  it("links owned role sessions and worktree evidence to existing inspection surfaces", async () => {
    const user = userEvent.setup();
    const client = new QueryClient();
    render(
      <QueryClientProvider client={client}>
        <LoopTimeline onInspect={() => undefined} run={exampleRun()} />
      </QueryClientProvider>,
    );

    ["会话记录", "变更", "文件", "终端记录", "日志", "报告", "用量"].forEach((surface) => {
      expect(screen.getAllByLabelText(`打开${surface}`).length).toBeGreaterThan(0);
    });
    expect(screen.getAllByLabelText("打开日志").length).toBeGreaterThan(2);

    // The worker/verifier evidence's own operation ids are only reachable through the nested raw
    // evidence disclosure now (task 17.10 no longer dumps every evidence item unconditionally) --
    // opening it proves the fact is still available on demand, not silently dropped.
    expect(screen.queryByText(/operation-worker/)).toBeNull();
    await user.click(screen.getByRole("button", { name: "证据时间线" }));
    expect(screen.getByText(/operation-worker/)).toBeTruthy();
    expect(screen.getByText(/operation-verifier/)).toBeTruthy();
  });

  // 21.16: `useLoopElapsed` (loop-run-header.tsx's own per-second ticking clock) had no direct
  // test at all before this -- `formatLoopDuration`/`latestLoopOperationEvidence` above are the
  // only two exports of this module previously exercised here. Its own doc comment already
  // discloses a *different*, real, out-of-scope-here gap (it does not pause while hidden, unlike
  // Mission Control/Loop-poll/Evaluation's pollers); this only proves the property 21.16 asks for --
  // the interval handle itself is released, not merely "no longer observed ticking".
  describe("useLoopElapsed", () => {
    it("arms exactly one interval for a running run, and releases it on unmount", () => {
      vi.useFakeTimers();
      const runningRun = { status: "running", startedAt: "2026-07-23T00:00:00Z", createdAt: "2026-07-23T00:00:00Z" } as LoopRun;

      const { unmount } = renderHook(() => useLoopElapsed(runningRun));
      expect(getActivePendingTimerCount()).toBe(1);

      unmount();
      expect(getActivePendingTimerCount()).toBe(0);
    });

    it("also releases the interval when the run leaves an active status without unmounting", () => {
      vi.useFakeTimers();
      const runningRun = { status: "running", startedAt: "2026-07-23T00:00:00Z", createdAt: "2026-07-23T00:00:00Z" } as LoopRun;
      const completedRun = {
        status: "succeeded", startedAt: "2026-07-23T00:00:00Z", createdAt: "2026-07-23T00:00:00Z",
        completedAt: "2026-07-23T00:05:00Z", updatedAt: "2026-07-23T00:05:00Z",
      } as LoopRun;

      const { rerender } = renderHook(({ run }) => useLoopElapsed(run), { initialProps: { run: runningRun } });
      expect(getActivePendingTimerCount()).toBe(1);

      rerender({ run: completedRun });
      expect(getActivePendingTimerCount()).toBe(0);
    });
  });
});

function evidence(overrides: Partial<LoopEvidence>): LoopEvidence {
  return {
    id: "evidence", runId: "run-1", iterationId: "iteration-1", kind: "worker", status: "passed", summary: "Completed work.",
    operationId: null, commandId: null, exitCode: null, durationMs: 100, details: null, createdAt: "2026-07-23T00:03:00Z", ...overrides,
  };
}

function exampleRun(): LoopRun {
  const definition = {
    id: "loop-1", name: "Release", enabled: true, projectPath: "D:/repo", baseBranch: "main", goal: "Ship safely",
    acceptanceCriteria: ["Tests pass"], allowedPaths: ["src"], protectedPaths: [".git"], workerAgentId: "codex-cli", verifierAgentId: "claude-code",
    verificationCommands: [{ id: "tests", program: "npm", args: ["test"], workingDirectory: null, timeoutSeconds: 120, required: true }],
    limits: { maxIterations: 3, stepTimeoutSeconds: 300, totalTimeoutSeconds: 1800, maxConsecutiveRuntimeErrors: 2, maxConsecutiveNoProgress: 2 },
    version: 1, createdAt: "2026-07-23T00:00:00Z", updatedAt: "2026-07-23T00:00:00Z",
  };
  return {
    id: "run-1", definitionId: definition.id, definitionSnapshot: definition, status: "awaiting-acceptance", phase: "finalizing", terminalReason: null,
    currentIteration: 1, consecutiveRuntimeErrors: 0, consecutiveNoProgress: 1, pauseRequested: false, projectPath: definition.projectPath,
    worktreePath: "D:/repo-loop", worktreeName: "loop-release", worktreeBranch: "vanehub/loop-release", activeOperationId: null, simulated: true,
    createdAt: "2026-07-23T00:00:00Z", startedAt: "2026-07-23T00:00:00Z", updatedAt: "2026-07-23T00:05:05Z", completedAt: null,
    iterations: [{
      id: "iteration-1", runId: "run-1", sequence: 1, status: "awaiting-acceptance", workerSessionId: "worker-session", verifierSessionId: "verifier-session",
      workerSummary: "Implemented the release change.", verifierRecommendation: "pass", verifierFindings: ["Required checks passed."], decisionReason: "Ready for human acceptance.",
      diffFingerprint: "diff-123", checkFailureFingerprint: null, userFeedback: null, startedAt: "2026-07-23T00:01:00Z", completedAt: "2026-07-23T00:05:05Z",
      evidence: [
        evidence({ id: "worker", details: { changedFiles: 3, additions: 48, deletions: 12 }, operationId: "operation-worker" }),
        evidence({ id: "check", kind: "verification", summary: "npm test", commandId: "tests", exitCode: 0, operationId: "operation-check" }),
        evidence({ id: "verifier", kind: "verifier", summary: "Verifier passed.", operationId: "operation-verifier" }),
        evidence({ id: "decision", kind: "decision", summary: "Ready for human acceptance." }),
      ],
    }],
  };
}
