import { expect, test } from "@playwright/test";

test.describe("Usage statistics", () => {
  test("navigates, changes range, refreshes, and renders monitoring sections", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: /使用统计|Usage Statistics/ }).click();

    await expect(page.getByRole("heading", { name: /使用统计|Usage Statistics/ })).toBeVisible();
    await page.getByRole("button", { name: /近 7 天|Last 7 days/ }).click();
    await expect(page.getByRole("button", { name: /近 7 天|Last 7 days/ })).toHaveAttribute("aria-pressed", "true");
    await page.getByRole("button", { name: /^刷新$|^Refresh$/ }).click();

    await expect(page.getByText(/真实总 Token|Reported Total Tokens/)).toBeVisible();
    await expect(page.getByRole("heading", { name: /每日趋势|Daily Trend/ })).toBeVisible();
    await expect(page.getByRole("heading", { name: /Agent 使用量|Usage by Agent/ })).toBeVisible();
    await expect(page.getByRole("heading", { name: /统计口径|Accounting Notes/ })).toBeVisible();
    await expect(page.getByText("155", { exact: true }).first()).toBeVisible();
    await expect(page.getByText("1,400", { exact: true }).first()).toBeVisible();
    await expect(page.getByRole("img", { name: /真实 Token|Reported Tokens/ })).toBeVisible();
    await expect(page.locator("article").filter({ hasText: "codex-cli" })).toBeVisible();
    await expect(page.locator("article").filter({ hasText: "claude-code" })).toBeVisible();
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

      await expect(page.getByRole("heading", { name: /使用统计|Usage Statistics/ })).toBeVisible();
      await expect(page.getByRole("heading", { name: /每日趋势|Daily Trend/ })).toBeVisible();
      await expect(page.getByRole("heading", { name: /Agent 使用量|Usage by Agent/ })).toBeVisible();
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    });
  }
});
