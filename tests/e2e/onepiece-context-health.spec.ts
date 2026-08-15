import { expect, test, type Page } from "@playwright/test";

async function openOnePieceParameters(page: Page) {
  await page.goto("/");
  await page.getByRole("button", { name: /设置|Settings/ }).click();
  await page.getByRole("button", { name: /^(CLI 参数|CLI Parameters)$/ }).click();
  await page.getByRole("button", { name: "OnePiece" }).click();
}

test.describe("OnePiece context policy health", () => {
  test("supports keyboard range controls and retains the selected history window", async ({ page }) => {
    await openOnePieceParameters(page);
    const region = page.getByRole("region", { name: "上下文策略健康" });
    await expect(region.getByText("测量质量")).toBeVisible();
    await expect(region.getByText(/compacted · optimizer/).first()).toBeVisible();

    const sevenDays = region.getByRole("button", { name: "7 天" });
    await sevenDays.focus();
    await page.keyboard.press("Space");
    await expect(sevenDays).toHaveAttribute("aria-pressed", "true");

    await region.getByRole("combobox", { name: "保留期" }).selectOption("90");
    await page.reload();
    await page.getByRole("button", { name: /^(CLI 参数|CLI Parameters)$/ }).click();
    await page.getByRole("button", { name: "OnePiece" }).click();
    await expect(page.getByRole("combobox", { name: "保留期" })).toHaveValue("90");
  });

  test("remains readable in English minimal theme at a narrow viewport", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("combobox", { name: /应用语言|Application Language/ }).selectOption("en");
    await page.getByRole("combobox", { name: /主题|Theme/ }).selectOption("minimal");
    await page.getByRole("button", { name: "CLI Parameters" }).click();
    await page.getByRole("button", { name: "OnePiece" }).click();

    const region = page.getByRole("region", { name: "Context policy health" });
    await expect(region.getByText("Token coverage")).toBeVisible();
    await expect(region.getByRole("button", { name: "7 days" })).toBeVisible();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "minimal");
    expect(await page.evaluate(() => document.body.scrollWidth > window.innerWidth)).toBe(false);
  });
});
