import { expect, test, type Page } from "@playwright/test";

async function openPlans(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Plan 执行" }).click();
  await expect(page.locator("#plan-center")).toBeVisible();
}

async function generateDraft(page: Page, goal: string): Promise<void> {
  const planCenter = page.locator("#plan-center");
  await planCenter.getByRole("textbox", { name: "目标" }).fill(goal);
  await planCenter.getByRole("textbox", { name: "项目路径" }).fill("D:/fixture-project");
  await planCenter.getByRole("button", { name: "生成 Plan" }).click();
  await expect(planCenter.getByText("审批任务图")).toBeVisible();
}

test.describe("Plan execution", () => {
  test("validates edits, pauses, resumes, executes serially, and retains the accepted worktree", async ({ page }) => {
    await page.goto("/");
    await openPlans(page);
    await generateDraft(page, "实现并验证串行 Plan 执行");
    const planCenter = page.locator("#plan-center");
    const criteria = planCenter.getByLabel("验收标准（每行一条，最多三条）").first();

    await criteria.fill("");
    await planCenter.getByRole("button", { name: "审批并启动" }).click();
    await expect(planCenter.getByRole("alert")).toContainText("每个任务必须包含 1 到 3 条验收标准");
    await criteria.fill("第一项检查通过");
    await planCenter.getByLabel("任务 1 标题").fill("分析边界");
    await planCenter.getByRole("button", { name: "审批并启动" }).click();

    await expect(planCenter.getByText("模拟运行时")).toBeVisible();
    await expect(planCenter.getByText("执行中", { exact: true }).first()).toBeVisible();
    await planCenter.getByRole("button", { name: "暂停", exact: true }).click();
    await expect(planCenter.getByText("已暂停", { exact: true })).toBeVisible();
    await planCenter.getByRole("button", { name: "继续", exact: true }).click();
    await planCenter.getByRole("button", { name: "执行下一个任务" }).click();
    await expect(planCenter.getByText("成功", { exact: true }).first()).toBeVisible();
    await expect(planCenter.getByText("执行中", { exact: true }).first()).toBeVisible();
    await planCenter.getByRole("button", { name: "执行下一个任务" }).click();

    await expect(planCenter.getByText("等待验收", { exact: true })).toBeVisible();
    await expect(planCenter.getByText("已验证 2 / 2 个任务")).toBeVisible();
    await planCenter.getByRole("button", { name: "接受结果" }).click();
    await expect(planCenter.getByText("已完成", { exact: true })).toBeVisible();
    await expect(planCenter.getByText("保留的集成 worktree")).toBeVisible();
    await expect(planCenter.getByText("VaneHub 不会自动提交、合并、推送或删除此 worktree。")).toBeVisible();
  });

  test("presents recovery-required state and requires an explicit recovery action", async ({ page }) => {
    await page.goto("/");
    await openPlans(page);
    await generateDraft(page, "验证恢复门控展示");
    const planCenter = page.locator("#plan-center");
    await planCenter.getByRole("button", { name: "审批并启动" }).click();
    await expect(planCenter.getByText("模拟运行时")).toBeVisible();

    await page.evaluate(async () => {
      const modulePath = "/src/services/web-plan-client.ts";
      const planRuntime = await import(modulePath);
      planRuntime.markFirstWebPlanRunRecoveryRequired();
    });

    await expect(planCenter.getByText("需要恢复", { exact: true })).toBeVisible();
    await planCenter.getByRole("button", { name: "恢复", exact: true }).click();
    await expect(planCenter.getByText("已暂停", { exact: true })).toBeVisible();
  });
});
