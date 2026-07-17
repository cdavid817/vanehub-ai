import { expect, test } from "@playwright/test";

test.describe("Skills settings page", () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      window.localStorage.setItem(
        "vanehub.appSettings",
        JSON.stringify({ applicationLanguage: "en" }),
      );
    });
  });

  test("renders scope controls, Skill cards, and Agent mount paths", async ({ page }) => {
    await page.goto("/");

    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: /^Skills\b/ }).click();

    await expect(page.getByText("Agent mount paths")).toBeVisible();
    await expect(page.getByRole("button", { name: "Global", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Workspace", exact: true })).toBeVisible();
    await expect(page.getByText("TDD 开发纪律助手")).toBeVisible();
  });

  test("supports workspace mode, filtering, and restore dialog entry", async ({ page }) => {
    await page.goto("/");

    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: /^Skills\b/ }).click();
    await page.getByRole("button", { name: "Workspace", exact: true }).click();
    await page.getByPlaceholder("Select local project directory").fill("D:\\example-workspace");
    await page.getByPlaceholder("Search by id, name, category, trigger, or source").fill("readme");
    await page.getByRole("button", { name: /Restore Built-in/ }).click();

    await expect(page.getByText("Restore built-in Skill")).toBeVisible();
  });
});
