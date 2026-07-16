import { expect, test } from "@playwright/test";

test.describe("Skills settings page", () => {
  test("renders scope controls, Skill cards, and Agent mount paths", async ({ page }) => {
    await page.goto("/");

    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByText("Skills").click();

    await expect(page.getByText("Agent mount paths")).toBeVisible();
    await expect(page.getByText("Global")).toBeVisible();
    await expect(page.getByText("Workspace")).toBeVisible();
    await expect(page.getByText("TDD 开发纪律助手")).toBeVisible();
  });

  test("supports workspace mode, filtering, and restore dialog entry", async ({ page }) => {
    await page.goto("/");

    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByText("Skills").click();
    await page.getByText("Workspace").click();
    await page.getByPlaceholder("Select local project directory").fill("D:\\example-workspace");
    await page.getByPlaceholder("Search by id, name, category, trigger, or source").fill("readme");
    await page.getByRole("button", { name: /Restore Built-in/ }).click();

    await expect(page.getByText("Restore built-in Skill")).toBeVisible();
  });
});
