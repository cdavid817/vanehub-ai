import { expect, test } from "@playwright/test";

async function openUsageStatistics(page: import("@playwright/test").Page) {
  await page.goto("/");
  await page.getByRole("button", { name: /设置|Settings/ }).click();
  await page.getByRole("button", { name: /使用统计|Usage Statistics/ }).click();
  await page.getByRole("button", { name: /^全部$|^All time$/ }).click();
}

test.describe("Usage statistics", () => {
  test.setTimeout(120_000);

  test("filters one deterministic ledger across summaries and breakdowns", async ({ page }) => {
    await openUsageStatistics(page);

    await expect(page.getByRole("heading", { name: /使用统计|Usage Statistics/, level: 2 })).toBeVisible();
    await page.getByRole("combobox", { name: /Agent/ }).selectOption("onepiece");
    await page.getByRole("combobox", { name: /质量|Quality/ }).selectOption("reported");
    await page.getByRole("combobox", { name: /状态|Status/ }).selectOption("failed");

    await expect(page.getByText(/真实总 Token|Reported Total Tokens/)).toBeVisible();
    await expect(page.getByText("90", { exact: true }).first()).toBeVisible();
    await expect(page.getByText("tool-continuation", { exact: true })).toBeVisible();
    await expect(page.getByText("failed", { exact: true })).toBeVisible();
    await expect(page.getByRole("heading", { name: /每日趋势|Daily Trend/ })).toBeVisible();
    await expect(page.getByRole("heading", { name: /消耗拆分|Consumption breakdowns/ })).toBeVisible();
    await expect(page.getByRole("heading", { name: /统计口径|Accounting Notes/ })).toBeVisible();

    await page.getByRole("button", { name: /重置筛选|Reset filters/ }).click();
    await expect(page.getByText("480", { exact: true }).first()).toBeVisible();
    await expect(page.getByText("900", { exact: true }).first()).toBeVisible();
  });

  test("shows OnePiece multi-call session details lazily", async ({ page }) => {
    await page.goto("/");
    await page.evaluate(async () => {
      const module = await import("/src/services/web-agent-client.ts");
      await module.webAgentClient.saveOnePieceProviderConfig({
        provider: "Anthropic",
        modelId: "claude-sonnet-4-6",
        interfaceFormat: "anthropic",
        baseUrl: null,
        apiKey: "playwright-local-only-key",
      });
    });
    await page.getByRole("button", { name: /新建/ }).click();
    const dialog = page.getByRole("dialog");
    await dialog.locator("button").filter({ hasText: "OnePiece" }).first().click();
    await dialog.getByPlaceholder(/code.*project/).fill("D:\\token-usage-workspace");
    await dialog.getByPlaceholder("新会话").fill("OnePiece Token usage");
    await dialog.getByRole("button", { name: "创建", exact: true }).click();

    const infoPanel = page.locator("aside").filter({ hasText: /信息面板|Info Panel/ });
    await infoPanel.getByRole("tab", { name: /Token 使用|Token Usage/ }).click();
    await expect(infoPanel.getByText("330", { exact: true }).first()).toBeVisible();
    await expect(infoPanel.getByText(/工具续接|Tool continuation/)).toBeVisible();
    const details = infoPanel.getByRole("button", { name: /调用明细|Invocation details/ });
    await expect(details).toHaveAttribute("aria-expanded", "false");
    await details.click();
    await expect(details).toHaveAttribute("aria-expanded", "true");
    await expect(infoPanel.getByText("openai-compatible · reasoning-model")).toBeVisible();
    await expect(infoPanel.locator("article")).toHaveCount(4);
  });

  for (const variant of [
    { theme: "futuristic", width: 1440, height: 900 },
    { theme: "minimal", width: 390, height: 844 },
  ]) {
    test(`fits ${variant.theme} at ${variant.width}px`, async ({ page }) => {
      await page.setViewportSize({ width: variant.width, height: variant.height });
      await page.goto("/");
      await page.getByRole("button", { name: /设置|Settings/ }).click();
      await page.getByRole("button", { name: /基础配置|Basic Settings/ }).click();
      await page.getByRole("combobox", { name: /^主题$|^Theme$/ }).selectOption(variant.theme);
      await page.getByRole("button", { name: /使用统计|Usage Statistics/ }).click();

      await expect(page.getByRole("heading", { name: /使用统计|Usage Statistics/, level: 2 })).toBeVisible();
      await expect(page.getByRole("heading", { name: /每日趋势|Daily Trend/ })).toBeVisible();
      await expect(page.getByRole("heading", { name: /消耗拆分|Consumption breakdowns/ })).toBeVisible();
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    });
  }
});
