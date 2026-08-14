import { expect, test, type Locator, type Page } from "@playwright/test";

function agentButton(dialog: Locator, name: string) {
  return dialog.locator("button").filter({ hasText: name }).first();
}

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
    test.setTimeout(180_000);
    await page.goto("/", { waitUntil: "domcontentloaded" });
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
    await expect(planCenter.getByText("成功", { exact: true }).first()).toBeVisible();

    await expect(planCenter.getByText("等待验收", { exact: true })).toBeVisible();
    await expect(planCenter.getByText("已验证 2 / 2 个任务")).toBeVisible();
    await expect(planCenter.getByText("集成最终验证", { exact: true })).toBeVisible();
    await expect(planCenter.getByText("Simulated final verification passed.")).toBeVisible();
    await planCenter.getByRole("button", { name: "接受结果" }).click();
    await expect(planCenter.getByText("已完成", { exact: true })).toBeVisible();
    await expect(planCenter.getByText("保留的集成 worktree")).toBeVisible();
    await expect(planCenter.getByText("VaneHub 不会自动提交、合并、推送或删除此 worktree。")).toBeVisible();
  });

  test("presents recovery-required state and requires an explicit recovery action", async ({ page }) => {
    await page.goto("/", { waitUntil: "domcontentloaded" });
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

  test("retains failed verification evidence while an automatic repair succeeds", async ({ page }) => {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await openPlans(page);
    await generateDraft(page, "验证失败后自动修复");
    const planCenter = page.locator("#plan-center");
    await planCenter.getByRole("button", { name: "审批并启动" }).click();
    await page.evaluate(async () => {
      const module = await import("/src/services/web-plan-client.ts");
      module.triggerLatestWebPlanRepairForTest();
    });
    await expect(planCenter.getByText("修复中", { exact: true })).toBeVisible();
    await expect(planCenter.getByText("等待验收", { exact: true })).toBeVisible({ timeout: 15_000 });
    await planCenter.getByText("第 1 次尝试 · failed", { exact: true }).click();
    await expect(planCenter.getByText("Simulated verification failure retained for repair.")).toBeVisible();
  });

  test("pauses the associated write-capable run before presenting Plan mode", async ({ page }) => {
    test.setTimeout(120_000);
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await page.evaluate(async () => {
      const module = await import("/src/services/web-agent-client.ts");
      await module.webAgentClient.saveOnePieceProviderConfig({ provider: "Anthropic", modelId: "claude-opus-4-8", interfaceFormat: "anthropic", baseUrl: null, apiKey: "playwright-local-only-key" });
    });
    await page.getByRole("button", { name: /新建/ }).click();
    const dialog = page.getByRole("dialog");
    await agentButton(dialog, "OnePiece").click();
    await dialog.getByPlaceholder(/code.*project/).fill("D:\\onepiece-plan-workspace");
    await dialog.getByPlaceholder("新会话").fill("Plan 模式切换会话");
    await dialog.getByRole("button", { name: "创建", exact: true }).click();
    await openPlans(page);
    await generateDraft(page, "关联会话安全切换");
    const planCenter = page.locator("#plan-center");
    await planCenter.getByRole("button", { name: "审批并启动" }).click();
    await page.getByRole("button", { name: "会话" }).click();
    await expect(page.getByRole("button", { name: "打开计划" })).toBeVisible();
    await page.getByRole("button", { name: "Agent · 可修改", exact: true }).click();
    await page.getByRole("menuitemradio", { name: /^计划 · 只读/ }).click();
    await expect(page.getByRole("button", { name: "计划 · 只读", exact: true })).toBeVisible();
    await expect.poll(() => page.evaluate(async () => {
      const plans = await import("/src/services/web-plan-client.ts");
      const agents = await import("/src/services/web-agent-client.ts");
      const session = await agents.webAgentClient.getActiveSession();
      const run = await plans.webPlanClient.getPlanRunForSession(session?.id ?? "");
      return run?.status ?? null;
    })).toBe("paused");
    await page.getByRole("button", { name: "打开计划" }).click();
    await expect(planCenter.getByText("已暂停", { exact: true })).toBeVisible();
  });
});
