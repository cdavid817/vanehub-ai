// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
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

  // 17.8: only the one primary action per status (plus the feedback textarea, for
  // awaiting-acceptance) renders directly now -- Cancel/Continue/Reject moved behind the
  // closed-by-default "More" menu, so they are deliberately absent from this static render. See
  // the "More menu" and "accept/continue/reject confirmation" describes below for those.
  it("renders exactly the primary action and feedback field directly, plus a More trigger for the rest", () => {
    const active = renderControls("running");
    expect(active).toContain("暂停");
    expect(active).toContain("更多操作");
    expect(active).not.toContain("接受结果");
    expect(active).not.toContain("停止");

    const paused = renderControls("paused");
    expect(paused).toContain("恢复");
    expect(paused).toContain("将从已持久化的“验证”阶段边界恢复");
    expect(paused).toContain("更多操作");

    const awaiting = renderControls("awaiting-acceptance");
    expect(awaiting).toContain("接受结果");
    expect(awaiting).toContain("下一次迭代的反馈");
    expect(awaiting).toContain("更多操作");
    expect(awaiting).not.toContain("根据反馈继续");
    expect(awaiting).not.toContain("拒绝结果");
  });

  it("renders no mutation controls for terminal runs", () => {
    expect(renderControls("failed")).toBe("");
  });
});

// 17.8: Cancel/Continue/Reject moved from their own always-visible buttons into the "More" menu.
// This checks they are still reachable and carry the right disabled/reason state once opened,
// matching scheduled-task-row.test.tsx's own "More > action" pattern.
describe("LoopRunControls More menu", () => {
  it("running: More > Stop is enabled", async () => {
    renderInteractive(loopRunFixture("running"));
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "更多操作" }));
    expect(screen.getByRole("menuitem", { name: "停止" }).getAttribute("aria-disabled")).toBe("false");
  });

  it("paused: More > Stop is enabled", async () => {
    renderInteractive(loopRunFixture("paused"));
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "更多操作" }));
    expect(screen.getByRole("menuitem", { name: "停止" }).getAttribute("aria-disabled")).toBe("false");
  });

  it("awaiting-acceptance: More has Continue (disabled until feedback is typed) and an enabled Reject", async () => {
    renderInteractive(loopRunFixture("awaiting-acceptance"));
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "更多操作" }));
    expect(screen.getByRole("menuitem", { name: "根据反馈继续" }).getAttribute("aria-disabled")).toBe("true");
    expect(screen.getByRole("menuitem", { name: "拒绝结果" }).getAttribute("aria-disabled")).toBe("false");
  });

  it("awaiting-acceptance: Continue's disabled reason names the iteration limit once it is reached", async () => {
    renderInteractive(loopRunFixture("awaiting-acceptance", { currentIteration: 3 }));
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "更多操作" }));
    // The same warning also renders unconditionally near the feedback field (outside the menu) --
    // scope to the open menu so this only asserts the menu item's own `disabledReason` wiring.
    expect(within(screen.getByRole("menu")).getByText("已达到最大迭代次数，请接受或拒绝此结果。")).toBeTruthy();
  });
});

// Task 17.13 gave accept/continue/reject a consequence preview before executing (design.md
// Decision 14: "Accept/Continue/Reject 前显示后果"). Task 17.8 kept Accept (the primary action,
// rendered outside any menu) and Continue (whose confirm step must leave the feedback textarea
// live -- see loop-run-controls.tsx's own `ConfirmAction` comment) on the pre-existing hand-rolled
// preview-then-confirm block (`role="alertdialog"`); Reject and Cancel moved to `ActionMenu`'s own
// built-in `confirmation`, a real modal `role="dialog"`, since neither has a live input to protect.
describe("LoopRunControls accept/continue/reject/cancel confirmation", () => {
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

  it("previews consequences before continuing (opened from More), and sends the typed feedback only after confirming", async () => {
    const continueLoop = vi.spyOn(agentService, "continueLoop").mockResolvedValue(loopRunFixture("running"));
    renderInteractive(loopRunFixture("awaiting-acceptance"));
    const user = userEvent.setup();

    await user.type(screen.getByLabelText("下一次迭代的反馈"), "please retry");
    await user.click(screen.getByRole("button", { name: "更多操作" }));
    await user.click(screen.getByRole("menuitem", { name: "根据反馈继续" }));
    expect(continueLoop).not.toHaveBeenCalled();
    expect(screen.getByRole("alertdialog")).toBeTruthy();
    expect(screen.getByText("根据此反馈继续？")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "确认" }));
    expect(continueLoop).toHaveBeenCalledWith({ runId: "run-1", feedback: "please retry" });
  });

  it("disables confirm, rather than silently no-op-ing, if feedback is cleared while continue's preview is open", async () => {
    renderInteractive(loopRunFixture("awaiting-acceptance"));
    const user = userEvent.setup();

    await user.type(screen.getByLabelText("下一次迭代的反馈"), "please retry");
    await user.click(screen.getByRole("button", { name: "更多操作" }));
    await user.click(screen.getByRole("menuitem", { name: "根据反馈继续" }));
    await user.clear(screen.getByLabelText("下一次迭代的反馈"));

    expect((screen.getByRole("button", { name: "确认" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("requires confirmation for reject via More's own dialog, and only calls rejectLoop after confirming", async () => {
    const reject = vi.spyOn(agentService, "rejectLoop").mockResolvedValue(loopRunFixture("cancelled"));
    renderInteractive(loopRunFixture("awaiting-acceptance"));
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "更多操作" }));
    await user.click(screen.getByRole("menuitem", { name: "拒绝结果" }));
    expect(reject).not.toHaveBeenCalled();
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("拒绝此结果？")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "确认" }));
    expect(reject).toHaveBeenCalledWith("run-1");
  });

  it("requires confirmation for cancel via More's own dialog, and only calls cancelLoop after confirming", async () => {
    const cancel = vi.spyOn(agentService, "cancelLoop").mockResolvedValue(loopRunFixture("cancelled"));
    renderInteractive(loopRunFixture("running"));
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "更多操作" }));
    await user.click(screen.getByRole("menuitem", { name: "停止" }));
    expect(cancel).not.toHaveBeenCalled();
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("立即停止此循环？")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "确认" }));
    expect(cancel).toHaveBeenCalledWith("run-1");
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
