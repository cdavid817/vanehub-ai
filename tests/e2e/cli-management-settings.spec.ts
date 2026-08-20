import { expect, test } from "@playwright/test";

test.describe("CLI local environment management", () => {
  test("renders honest Web diagnostics in the default minimal Chinese settings surface", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: /^CLI 管理/ }).click();

    await expect(page.getByRole("heading", { name: "CLI 管理", level: 2 })).toBeVisible();
    await expect(page.getByTestId("cli-installation-summary")).toHaveCount(1);
    await expect(page.getByText("CLI 未安装", { exact: true })).toHaveCount(0);
    const cards = page.locator("[data-cli-agent]");
    await expect(cards).toHaveCount(5);
    expect(await cards.evaluateAll((items) => items.map((item) => item.getAttribute("data-cli-agent")))).toEqual([
      "claude-code",
      "codex-cli",
      "opencode",
      "antigravity-cli",
      "gemini-cli",
    ]);
    await expect(page.locator('[data-cli-agent="claude-code"]')).toContainText("Anthropic Claude Code CLI");
    await expect(page.getByText("不支持").first()).toBeVisible();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "minimal");
  });

  test("keeps English minimal CLI and About summaries readable at narrow width", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("combobox", { name: /应用语言|Application Language/ }).selectOption("en");
    await page.getByRole("combobox", { name: /主题|Theme/ }).selectOption("minimal");
    await page.getByRole("button", { name: /^CLI Management/ }).click();

    await expect(page.getByRole("heading", { name: "CLI Management", level: 2 })).toBeVisible();
    await expect(page.locator("[data-cli-agent]")).toHaveCount(5);
    await expect(page.getByText("Unsupported").first()).toBeVisible();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "minimal");

    await page.getByRole("button", { name: /^About$/ }).click();
    await expect(page.getByRole("heading", { name: "Software Details" })).toBeVisible();
    await expect(page.getByText("Release Channel", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: /^CLI Management/ }).click();
    await expect(page.getByRole("heading", { name: "CLI Management", level: 2 })).toBeVisible();

    const overflow = await page.evaluate(() => document.body.scrollWidth > window.innerWidth);
    expect(overflow).toBe(false);
  });
});
