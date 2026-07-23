import { renderToStaticMarkup } from "react-dom/server";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it } from "vitest";
import "../i18n";
import type { LoopEvidence, LoopRun } from "../types/loop";
import { LoopInspector } from "./loop-inspector";
import { formatLoopDuration, latestLoopOperationEvidence } from "./loop-monitoring";
import { LoopTimeline } from "./loop-timeline";

describe("Loop monitoring", () => {
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
    const html = renderToStaticMarkup(<QueryClientProvider client={client}><LoopTimeline refreshing run={run} /><LoopInspector loading={false} run={run} /></QueryClientProvider>);

    expect(html).toContain("正在刷新");
    expect(html).toContain("5:05");
    expect(html).toContain("1 / 3");
    expect(html).toContain("3 个文件，+48 / -12");
    expect(html).toContain("验证检查");
    expect(html).toContain("验证者审查");
    expect(html).toContain("Required checks passed.");
    expect(html).toContain("operation-verifier");
    expect(html).toContain("连续运行错误限制");
    expect(html).toContain("open=\"\"");
  });

  it("links owned role sessions and worktree evidence to existing inspection surfaces", () => {
    const client = new QueryClient();
    const html = renderToStaticMarkup(
      <QueryClientProvider client={client}>
        <LoopTimeline onInspect={() => undefined} run={exampleRun()} />
      </QueryClientProvider>,
    );

    ["会话记录", "变更", "文件", "终端记录", "日志", "报告", "用量"].forEach((surface) => {
      expect(html).toContain(`aria-label="打开${surface}"`);
    });
    expect(html.match(/aria-label="打开日志"/g)?.length).toBeGreaterThan(2);
    expect(html).toContain("operation-worker");
    expect(html).toContain("operation-verifier");
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
