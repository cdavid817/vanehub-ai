import { expect, test } from "@playwright/test";

async function seedRecovery(page: import("@playwright/test").Page, status: "action_required" | "quarantined") {
  await page.evaluate(async (recoveryStatus) => {
    const { seedWebRecoverySessionForTest } = await import("/src/services/web-agent-client.ts");
    seedWebRecoverySessionForTest(recoveryStatus);
  }, status);
}

test.describe("session recovery review", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
  });

  test("reviews and acknowledges action-required recovery without offering stop", async ({ page }) => {
    await seedRecovery(page, "action_required");

    await expect(page.getByTestId("session-recovery-notice")).toBeVisible();
    await expect(page.getByText("检查中断的工作")).toBeVisible();
    await expect(page.getByPlaceholder("请选择会话后发送消息")).toBeDisabled();
    await expect(page.getByRole("button", { name: "停止" })).toHaveCount(0);

    await page.getByRole("button", { name: "检查并继续" }).click();
    await expect(page.getByText(/不会重试中断的响应或任何工具调用/)).toBeVisible();
    await expect(page.getByText(/不能证明它是否发生/)).toBeVisible();
    await page.getByRole("button", { name: "确认并重新开放" }).click();

    await expect(page.getByTestId("session-recovery-notice")).toHaveCount(0);
    await expect(page.getByPlaceholder("输入指令，下发任务给当前 Agent...")).toBeEnabled();
  });

  test("unsticks a session runtime from the session list without switching sessions", async ({ page }) => {
    await seedRecovery(page, "action_required");

    // The runtime failure banner and the crash-recovery notice are independent surfaces and both
    // apply here: one asks for an acknowledgement decision, the other offers a retry.
    const banner = page.getByTestId("session-runtime-failure-notice");
    await expect(banner).toBeVisible();
    await expect(banner.getByRole("button", { name: "恢复会话" })).toBeEnabled();

    const sessionCard = page.locator("[data-session-id]").filter({ hasText: "Recovered Web session" });
    await sessionCard.click({ button: "right" });
    // Recovery is idempotent, so it is offered for any live session rather than only for one that
    // already looks broken — a user who suspects a stuck runtime should not have to prove it first.
    const menu = page.locator("div.ucd-panel.fixed").filter({ hasText: "导出会话" });
    await menu.getByRole("button", { name: "恢复会话" }).click();

    await expect(page.getByRole("status").filter({ hasText: "会话已恢复" })).toBeVisible();
    await expect(banner).toHaveCount(0);
  });

  test("keeps quarantined sessions inspectable and exportable while execution stays blocked", async ({ page }) => {
    await seedRecovery(page, "quarantined");

    await expect(page.getByText("会话已隔离")).toBeVisible();
    await page.getByRole("tab", { name: "报告" }).click();
    await expect(page.getByRole("tabpanel", { name: "报告" })).toBeVisible();

    const sessionCard = page.locator("[data-session-id]").filter({ hasText: "Recovered Web session" });
    await sessionCard.click({ button: "right" });
    await expect(page.getByText("导出会话")).toBeVisible();
    await expect(page.getByRole("button", { name: "JSON", exact: true })).toBeEnabled();
    await expect(page.getByPlaceholder("请选择会话后发送消息")).toBeDisabled();
  });
});
