// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { agentService } from "../services/runtime-agent-client";
import { loopRunFixture } from "../test/loop-fixtures";
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

// Task 17.13: accept/continue now preview consequences first, the same way pause/cancel/reject
// already did (design.md Decision 14: "Accept/Continue/Reject 前显示后果").
describe("LoopRunControls accept/continue confirmation", () => {
  afterEach(() => vi.restoreAllMocks());

  it("previews consequences before accepting, and only calls acceptLoop after confirming", async () => {
    const accept = vi.spyOn(agentService, "acceptLoop").mockResolvedValue(loopRunFixture("succeeded"));
    renderInteractive(loopRunFixture("awaiting-acceptance"));
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "接受结果" }));
    expect(accept).not.toHaveBeenCalled();
    expect(screen.getByRole("alertdialog")).toBeTruthy();
    expect(screen.getByText("接受此结果？")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "确认" }));
    expect(accept).toHaveBeenCalledWith("run-1");
  });

  it("previews consequences before continuing, and sends the typed feedback only after confirming", async () => {
    const continueLoop = vi.spyOn(agentService, "continueLoop").mockResolvedValue(loopRunFixture("running"));
    renderInteractive(loopRunFixture("awaiting-acceptance"));
    const user = userEvent.setup();

    await user.type(screen.getByLabelText("下一次迭代的反馈"), "please retry");
    await user.click(screen.getByRole("button", { name: "根据反馈继续" }));
    expect(continueLoop).not.toHaveBeenCalled();
    expect(screen.getByText("根据此反馈继续？")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "确认" }));
    expect(continueLoop).toHaveBeenCalledWith({ runId: "run-1", feedback: "please retry" });
  });

  it("disables confirm, rather than silently no-op-ing, if feedback is cleared while continue's preview is open", async () => {
    renderInteractive(loopRunFixture("awaiting-acceptance"));
    const user = userEvent.setup();

    await user.type(screen.getByLabelText("下一次迭代的反馈"), "please retry");
    await user.click(screen.getByRole("button", { name: "根据反馈继续" }));
    await user.clear(screen.getByLabelText("下一次迭代的反馈"));

    expect((screen.getByRole("button", { name: "确认" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("still requires confirmation for reject, unchanged from before this task", async () => {
    const reject = vi.spyOn(agentService, "rejectLoop").mockResolvedValue(loopRunFixture("cancelled"));
    renderInteractive(loopRunFixture("awaiting-acceptance"));
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "拒绝结果" }));
    expect(reject).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "确认" }));
    expect(reject).toHaveBeenCalledWith("run-1");
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

function renderInteractive(run: LoopRun) {
  const client = new QueryClient();
  render(<QueryClientProvider client={client}><LoopRunControls run={run} /></QueryClientProvider>);
}
