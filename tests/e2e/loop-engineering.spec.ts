import { expect, test } from "@playwright/test";
import { createAndRunLoop, openLoops } from "./loop-helpers";
import { createSession } from "./session-helpers";

test.describe("Loop engineering", () => {
  test("runs a Loop through pause, inspection, feedback, and acceptance while preserving session state", async ({ page }) => {
    test.setTimeout(120_000);
    await page.goto("/");
    await createSession(page, "Loop 导航保留测试");
    // The session-created toast auto-dismisses after ~5s (see notification-reducer.ts's
    // DEFAULT_NOTIFICATION_DURATION_MS), but this test's own setup before the Loop pause/confirm
    // step below can occasionally still be within that window, and the toast's `pointer-events-auto`
    // article intercepts the confirm click underneath it — dismiss it immediately since this test,
    // unlike notifications.spec.ts, has no assertions about toast timing or history.
    await page.getByRole("status").filter({ hasText: "会话创建成功" }).getByRole("button", { name: "关闭通知" }).click();

    await openLoops(page);
    // Returning to the same session by identity, and an in-progress composer draft, both survive
    // this round trip (workspace-routing.spec.ts's "preserves session state, including an
    // in-progress draft, across navigating to another destination and back" covers this directly
    // and explains why it needed a real fix rather than falling out of the `hidden` toggle alone).
    await page.getByRole("button", { name: "折叠会话栏" }).click();
    await expect(page.getByTestId("session-conversation-header").getByText("Loop 导航保留测试")).toBeVisible();
    await openLoops(page);

    const loopCenter = page.getByTestId("loop-center");
    await page.evaluate(async () => {
      const module = await import("/src/services/web-agent-client.ts");
      module.setWebLoopPhaseDelayForTest(1_000);
    });
    await createAndRunLoop(page, "Playwright 接受循环");
    await expect(loopCenter.getByText("运行中", { exact: true }).first()).toBeVisible();
    await loopCenter.getByRole("button", { name: "暂停", exact: true }).click({ force: true });
    await expect(loopCenter.getByText("暂停此循环？")).toBeVisible();
    await loopCenter.getByRole("button", { name: "确认", exact: true }).click({ force: true });
    await expect(loopCenter.getByText("已暂停", { exact: true }).first()).toBeVisible();
    await expect(page.getByTestId("agent-run-status")).toHaveAttribute("data-state", "paused");
    await loopCenter.getByRole("button", { name: "恢复", exact: true }).click();

    await expect(loopCenter.getByText("等待验收", { exact: true }).first()).toBeVisible();
    await expect(page.getByTestId("agent-run-status")).toHaveAttribute("data-state", "verifying");
    const acceptance = loopCenter.getByLabel("人工验收");
    await expect(acceptance.getByText("必需的模拟检查均已通过。")).toBeVisible();
    await expect(acceptance.getByText("验证者：通过")).toBeVisible();
    await acceptance.getByRole("button", { name: "打开变更" }).first().click();
    await expect(page.getByRole("button", { name: "返回循环工程" })).toBeVisible();
    await expect(page.getByRole("tab", { name: "变更" })).toHaveAttribute("aria-selected", "true");
    await page.getByRole("button", { name: "返回循环工程" }).click();
    await expect(loopCenter).toBeVisible();

    // 17.8: Continue moved into the More menu (Accept is now the one primary action while
    // awaiting acceptance) but still opens the same preview-then-confirm step as before -- see
    // loop-run-controls.tsx's own ConfirmAction comment for why -- so it still needs a "确认" click.
    await loopCenter.getByLabel("下一次迭代的反馈").fill("补充边界条件回归测试");
    await loopCenter.getByRole("button", { name: "更多操作" }).click();
    await loopCenter.getByRole("menuitem", { name: "根据反馈继续" }).click();
    await expect(loopCenter.getByText("根据此反馈继续？")).toBeVisible();
    await loopCenter.getByRole("button", { name: "确认", exact: true }).click();
    await expect(loopCenter.getByText("补充边界条件回归测试")).toBeVisible();
    await expect(loopCenter.getByText("第 2 次迭代")).toBeVisible();
    await expect(loopCenter.getByText("等待验收", { exact: true }).first()).toBeVisible();
    await loopCenter.getByRole("button", { name: "接受结果" }).click();
    await expect(loopCenter.getByText("接受此结果？")).toBeVisible();
    await loopCenter.getByRole("button", { name: "确认", exact: true }).click();
    await expect(loopCenter.getByText("已成功", { exact: true }).first()).toBeVisible();
    await expect(page.getByTestId("agent-run-status")).toHaveAttribute("data-state", "completed");
    await expect(loopCenter.getByText("目标已达成").first()).toBeVisible();
  });

  test("rejects an acceptance-ready Loop while retaining its evidence", async ({ page }) => {
    await page.goto("/");
    await openLoops(page);
    const loopCenter = page.getByTestId("loop-center");
    await createAndRunLoop(page, "Playwright 拒绝循环");

    await expect(loopCenter.getByText("等待验收", { exact: true }).first()).toBeVisible();
    const acceptance = loopCenter.getByLabel("人工验收");
    await expect(acceptance.getByText("验证检查")).toBeVisible();
    // 17.8: Reject moved into the More menu (Accept is now the one primary action while awaiting
    // acceptance) and now confirms via ActionMenu's own dialog rather than the hand-rolled
    // alertdialog block -- see loop-run-controls.tsx's own ConfirmAction comment for why.
    await loopCenter.getByRole("button", { name: "更多操作" }).click();
    await loopCenter.getByRole("menuitem", { name: "拒绝结果" }).click();
    await expect(loopCenter.getByText("拒绝此结果？")).toBeVisible();
    await loopCenter.getByRole("button", { name: "确认", exact: true }).click();

    await expect(loopCenter.getByText("已取消", { exact: true }).first()).toBeVisible();
    await expect(loopCenter.getByText("用户已拒绝").first()).toBeVisible();
    await expect(loopCenter.getByText("验证检查").first()).toBeVisible();
  });
});
