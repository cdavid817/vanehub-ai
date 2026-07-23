import { expect, test } from "@playwright/test";
import { createAndRunLoop, openLoops } from "./loop-helpers";
import { createSession } from "./session-helpers";

test.describe("Loop engineering", () => {
  test("runs a Loop through pause, inspection, feedback, and acceptance while preserving session state", async ({ page }) => {
    await page.goto("/");
    await createSession(page, "Loop 导航保留测试");
    const workspaceInput = page.getByRole("textbox", { name: "工作区命令输入" });
    await workspaceInput.fill("保留 Loop 切换前的命令草稿");
    await page.getByRole("tab", { name: "日志" }).click();
    const logSearch = page.getByRole("textbox", { name: "搜索脱敏日志" });
    await logSearch.fill("runtime");

    await openLoops(page);
    await page.getByRole("button", { name: "折叠会话栏" }).click();
    await expect(page.getByRole("tab", { name: "日志" })).toHaveAttribute("aria-selected", "true");
    await expect(logSearch).toHaveValue("runtime");
    await page.getByRole("tab", { name: "工作区" }).click();
    await expect(workspaceInput).toHaveValue("保留 Loop 切换前的命令草稿");
    await openLoops(page);

    const loopCenter = page.locator("#loop-center");
    await createAndRunLoop(page, "Playwright 接受循环");
    await expect(loopCenter.getByText("运行中", { exact: true }).first()).toBeVisible();
    await loopCenter.getByRole("button", { name: "暂停", exact: true }).click();
    await expect(loopCenter.getByText("暂停此循环？")).toBeVisible();
    await loopCenter.getByRole("button", { name: "确认", exact: true }).click();
    await expect(loopCenter.getByText("已暂停", { exact: true }).first()).toBeVisible();
    await loopCenter.getByRole("button", { name: "恢复", exact: true }).click();

    await expect(loopCenter.getByText("等待验收", { exact: true }).first()).toBeVisible();
    await expect(loopCenter.getByText("必需的模拟检查均已通过。")).toBeVisible();
    await expect(loopCenter.getByText("验证者：通过").first()).toBeVisible();
    await loopCenter.getByRole("button", { name: "打开变更" }).first().click();
    await expect(page.getByRole("button", { name: "返回循环工程" })).toBeVisible();
    await expect(page.getByRole("tab", { name: "变更" })).toHaveAttribute("aria-selected", "true");
    await page.getByRole("button", { name: "返回循环工程" }).click();

    await loopCenter.getByLabel("下一次迭代的反馈").fill("补充边界条件回归测试");
    await loopCenter.getByRole("button", { name: "根据反馈继续" }).click();
    await expect(loopCenter.getByText("补充边界条件回归测试")).toBeVisible();
    await expect(loopCenter.getByText("第 2 次迭代")).toBeVisible();
    await expect(loopCenter.getByText("等待验收", { exact: true }).first()).toBeVisible();
    await loopCenter.getByRole("button", { name: "接受结果" }).click();
    await expect(loopCenter.getByText("已成功", { exact: true }).first()).toBeVisible();
    await expect(loopCenter.getByText("目标已达成")).toBeVisible();
  });

  test("rejects an acceptance-ready Loop while retaining its evidence", async ({ page }) => {
    await page.goto("/");
    await openLoops(page);
    const loopCenter = page.locator("#loop-center");
    await createAndRunLoop(page, "Playwright 拒绝循环");

    await expect(loopCenter.getByText("等待验收", { exact: true }).first()).toBeVisible();
    await expect(loopCenter.getByText("验证检查")).toBeVisible();
    await loopCenter.getByRole("button", { name: "拒绝结果" }).click();
    await expect(loopCenter.getByText("拒绝此结果？")).toBeVisible();
    await loopCenter.getByRole("button", { name: "确认", exact: true }).click();

    await expect(loopCenter.getByText("已取消", { exact: true }).first()).toBeVisible();
    await expect(loopCenter.getByText("用户已拒绝")).toBeVisible();
    await expect(loopCenter.getByText("验证检查")).toBeVisible();
  });
});
