import { expect, test } from "@playwright/test";

test.describe("CLI local environment management", () => {
  test("renders honest Web diagnostics in the futuristic Chinese settings surface", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: /^CLI 管理/ }).click();

    await expect(page.getByRole("heading", { name: "CLI 管理" })).toBeVisible();
    await expect(page.locator("[data-cli-agent]")).toHaveCount(4);
    await expect(page.locator('[data-cli-agent="claude-code"]')).toContainText("Anthropic Claude Code CLI");
    await expect(page.getByText("不支持").first()).toBeVisible();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "futuristic");
  });

  test("keeps English minimal CLI and About summaries readable at narrow width", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("combobox", { name: /应用语言|Application Language/ }).selectOption("en");
    await page.getByRole("combobox", { name: /主题|Theme/ }).selectOption("minimal");
    await page.getByRole("button", { name: /^CLI Management/ }).click();

    await expect(page.getByRole("heading", { name: "CLI Management" })).toBeVisible();
    await expect(page.locator("[data-cli-agent]")).toHaveCount(4);
    await expect(page.getByText("Unsupported").first()).toBeVisible();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "minimal");

    await page.getByRole("button", { name: /^About$/ }).click();
    await expect(page.getByText("Local CLI Environment")).toBeVisible();
    await page.getByRole("button", { name: "Open CLI Management" }).click();
    await expect(page.getByRole("heading", { name: "CLI Management" })).toBeVisible();

    const overflow = await page.evaluate(() => document.body.scrollWidth > window.innerWidth);
    expect(overflow).toBe(false);
  });
});
