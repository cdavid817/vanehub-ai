import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import "../i18n";
import type { LoopRun, LoopRunStatus } from "../types/loop";
import { availableLoopActions, LoopRunControls } from "./loop-run-controls";

describe("LoopRunControls", () => {
  it("maps run states to valid actions", () => {
    const expected = {
      queued: ["pause", "cancel"],
      running: ["pause", "cancel"],
      paused: ["resume", "cancel"],
      "awaiting-acceptance": ["accept", "continue", "reject"],
      succeeded: [],
      failed: [],
      cancelled: [],
    } satisfies Record<LoopRunStatus, ReturnType<typeof availableLoopActions>>;

    for (const [status, actions] of Object.entries(expected)) {
      expect(availableLoopActions({ status: status as LoopRunStatus })).toEqual(actions);
    }
  });

  it("renders active, paused, and acceptance controls with localized consequences", () => {
    const active = renderControls("running");
    expect(active).toContain("暂停");
    expect(active).toContain("停止");
    expect(active).not.toContain("接受结果");

    const paused = renderControls("paused");
    expect(paused).toContain("恢复");
    expect(paused).toContain("将从已持久化的“验证”阶段边界恢复");

    const awaiting = renderControls("awaiting-acceptance");
    expect(awaiting).toContain("接受结果");
    expect(awaiting).toContain("下一次迭代的反馈");
    expect(awaiting).toContain("根据反馈继续");
    expect(awaiting).toContain("拒绝结果");
    expect(awaiting).toContain("disabled=\"\"");
  });

  it("renders no mutation controls for terminal runs", () => {
    expect(renderControls("failed")).toBe("");
  });
});

function renderControls(status: LoopRunStatus) {
  const client = new QueryClient();
  const run = {
    id: "run-1", status, phase: "verifying", pauseRequested: false, currentIteration: 1,
    definitionSnapshot: { limits: { maxIterations: 3 } },
  } as LoopRun;
  return renderToStaticMarkup(<QueryClientProvider client={client}><LoopRunControls run={run} /></QueryClientProvider>);
}
