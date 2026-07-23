import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import "../i18n";
import { loopQueryKeys } from "../hooks/loop-query";
import type { LoopRun } from "../types/loop";
import { LoopCenter } from "./loop-center";
import { LoopInspector } from "./loop-inspector";
import { LoopTimeline } from "./loop-timeline";

describe("Loop Center states", () => {
  it("renders the localized empty-definition state", () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    client.setQueryData(loopQueryKeys.definitions, []);
    client.setQueryData(loopQueryKeys.runs(), []);

    const html = renderToStaticMarkup(
      <QueryClientProvider client={client}><LoopCenter /></QueryClientProvider>,
    );

    expect(html).toContain("暂无循环定义");
    expect(html).toContain('aria-label="新建循环定义"');
  });

  it("renders active controls and operation state for a running Loop", () => {
    const html = renderRun({ status: "running", phase: "acting", activeOperationId: "operation-worker" });

    expect(html).toContain("运行中");
    expect(html).toContain("执行");
    expect(html).toContain("operation-worker");
    expect(html).toContain("暂停");
    expect(html).toContain("停止");
  });

  it("renders paused and recovery-required resume boundaries", () => {
    const paused = renderRun({ status: "paused", phase: "verifying" });
    expect(paused).toContain("已暂停");
    expect(paused).toContain("恢复");
    expect(paused).toContain("将从已持久化的“验证”阶段边界恢复");

    const recovered = renderRun({
      status: "paused",
      phase: "deciding",
      terminalReason: "recovery-required",
    });
    expect(recovered).toContain("需要恢复");
    expect(recovered).toContain("将从已持久化的“决策”阶段边界恢复");
  });

  it("renders the complete human acceptance action set", () => {
    const html = renderRun({ status: "awaiting-acceptance", phase: "finalizing" });

    expect(html).toContain("等待验收");
    expect(html).toContain("接受结果");
    expect(html).toContain("下一次迭代的反馈");
    expect(html).toContain("根据反馈继续");
    expect(html).toContain("拒绝结果");
  });

  it.each([
    ["succeeded", "goal-met", "已成功", "目标已达成"],
    ["failed", "verification-failed", "已失败", "验证失败"],
    ["cancelled", "user-stopped", "已取消", "用户已停止"],
  ] as const)("renders terminal %s state without mutation controls", (status, terminalReason, statusLabel, reasonLabel) => {
    const html = renderRun({ status, terminalReason, phase: "finalizing", completedAt: "2026-07-23T00:05:00Z" });

    expect(html).toContain(statusLabel);
    expect(html).toContain(reasonLabel);
    expect(html).not.toContain("运行控制");
  });
});

function renderRun(overrides: Partial<LoopRun>) {
  const client = new QueryClient();
  const run = exampleRun(overrides);
  return renderToStaticMarkup(
    <QueryClientProvider client={client}>
      <LoopTimeline run={run} />
      <LoopInspector loading={false} run={run} />
    </QueryClientProvider>,
  );
}

function exampleRun(overrides: Partial<LoopRun>): LoopRun {
  const definition = {
    id: "loop-1", name: "Release", enabled: true, projectPath: "D:/repo", baseBranch: "main", goal: "Ship safely",
    acceptanceCriteria: ["Tests pass"], allowedPaths: ["src"], protectedPaths: [".git"], workerAgentId: "codex-cli", verifierAgentId: "claude-code",
    verificationCommands: [{ id: "tests", program: "npm", args: ["test"], workingDirectory: null, timeoutSeconds: 120, required: true }],
    limits: { maxIterations: 3, stepTimeoutSeconds: 300, totalTimeoutSeconds: 1800, maxConsecutiveRuntimeErrors: 2, maxConsecutiveNoProgress: 2 },
    version: 1, createdAt: "2026-07-23T00:00:00Z", updatedAt: "2026-07-23T00:00:00Z",
  };
  return {
    id: "run-1", definitionId: definition.id, definitionSnapshot: definition, status: "running", phase: "acting", terminalReason: null,
    currentIteration: 1, consecutiveRuntimeErrors: 0, consecutiveNoProgress: 0, pauseRequested: false, projectPath: definition.projectPath,
    worktreePath: "D:/repo-loop", worktreeName: "loop-release", worktreeBranch: "vanehub/loop-release", activeOperationId: null,
    iterations: [], simulated: true, createdAt: "2026-07-23T00:00:00Z", startedAt: "2026-07-23T00:00:00Z",
    updatedAt: "2026-07-23T00:01:00Z", completedAt: null, ...overrides,
  };
}
